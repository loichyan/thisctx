use crate::utils::{punctuated_parse, punctuated_tokens, tokens_with, Attributes, Braced, Parened};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{punctuated::Punctuated, Ident, Token, Type};
use syn::{token, FieldsNamed, FieldsUnnamed, Visibility};

mod kw {
    use syn::custom_keyword;
    custom_keyword!(source);
    custom_keyword!(context);
}

const INTO_ERROR: crate::shared::IntoError = crate::shared::IntoError;
const NONE_ERROR: crate::shared::NoneError = crate::shared::NoneError;

pub struct ThisCtx(Enum);

impl Parse for ThisCtx {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for ThisCtx {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens)
    }
}

struct Enum {
    attr: Attributes,
    vis: Visibility,
    enum_token: token::Enum,
    name: Ident,
    brace_token: token::Brace,
    variants: Punctuated<Variant, Token![,]>,
}

impl Enum {
    fn get_enum_def(&self) -> TokenStream {
        let Self {
            attr,
            vis,
            enum_token,
            name,
            brace_token,
            variants,
        } = self;
        let body = tokens_with(|tokens| {
            brace_token.surround(tokens, |tokens| {
                punctuated_tokens::<Token![,], _, _>(
                    tokens,
                    variants.iter().map(Variant::get_variant_def),
                );
            })
        });
        quote!(#attr #vis #enum_token #name #body)
    }

    fn get_context_def(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants
                .iter()
                .for_each(|variant| variant.get_context_def(&self.vis).to_tokens(tokens));
        })
        .to_token_stream()
    }

    fn get_impl_into_error(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants
                .iter()
                .for_each(|variant| variant.get_impl_into_error(&self.name).to_tokens(tokens));
        })
        .to_token_stream()
    }

    fn get_from_ctx_for_enum(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants.iter().for_each(|variant| {
                variant
                    .get_impl_from_ctx_for_enum(&self.name)
                    .to_tokens(tokens)
            });
        })
        .to_token_stream()
    }
}

impl Parse for Enum {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr = input.parse()?;
        let vis = input.parse()?;
        let enum_token = input.parse()?;
        let name = input.parse()?;
        let (brace_token, variants) =
            Braced::parse_with(input, |input| input.parse_terminated(Variant::parse))?;
        Ok(Self {
            attr,
            vis,
            enum_token,
            name,
            brace_token,
            variants,
        })
    }
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.get_enum_def().to_tokens(tokens);
        self.get_context_def().to_tokens(tokens);
        self.get_impl_into_error().to_tokens(tokens);
        self.get_from_ctx_for_enum().to_tokens(tokens);
    }
}

struct Variant {
    attr: Attributes,
    name: Ident,
    body: VariantBody,
}

impl Variant {
    fn get_variant_def(&self) -> TokenStream {
        let Self {
            attr,
            name: variant_name,
            body,
        } = self;
        let struct_body = body.get_struct_body(
            variant_name,
            &tokens_with(|tokens| body.get_src_ty().to_tokens(tokens)),
        );
        quote!(#attr #variant_name #struct_body)
    }

    fn get_context_def(&self, vis: &Visibility) -> TokenStream {
        let Self { name, body, .. } = self;
        let unit_def = || quote!(#vis struct #name;);
        match body {
            VariantBody::Struct { ctx, .. } => match ctx {
                ContextField::Some { anon_struct, .. } => anon_struct.get_struct_def(vis, name),
                ContextField::None => unit_def(),
            },
            VariantBody::Tuple { anon_struct, .. } => match anon_struct {
                Some(anon_struct) => anon_struct.get_struct_def(vis, name),
                None => unit_def(),
            },
            VariantBody::Unit => unit_def(),
        }
    }

    fn get_impl_into_error(&self, enum_name: &Ident) -> TokenStream {
        let Self {
            name: variant_name,
            body,
            ..
        } = self;
        let src_ty = body.get_src_ty().unwrap_or_else(|| quote!(#NONE_ERROR));
        let src_name = quote!(source);
        let expr_struct_body = body.get_struct_body(
            &tokens_with(|tokens| quote!(self).to_tokens(tokens)),
            &tokens_with(|tokens| src_name.to_tokens(tokens)),
        );
        quote!(
            #[allow(unused)]
            impl #INTO_ERROR for #variant_name {
                type Error = #enum_name;
                type Source = #src_ty;

                fn into_error(self, #src_name: Self::Source) -> Self::Error {
                    Self::Error::#variant_name #expr_struct_body
                }
            }
        )
    }

    fn get_impl_from_ctx_for_enum(&self, enum_name: &Ident) -> Option<TokenStream> {
        let Self {
            name: variant_name,
            body,
            ..
        } = self;
        if body.get_src_ty().is_some() {
            return None;
        }
        let ctx_name = quote!(context);
        let expr_struct_body = body.get_struct_body(&ctx_name, &tokens_with(|_| ()));
        let gen = quote!(
            #[allow(unused)]
            impl From<#variant_name> for #enum_name {
                fn from(#ctx_name: #variant_name) -> Self {
                    Self::#variant_name #expr_struct_body
                }
            }
        );
        Some(gen)
    }
}

impl Parse for Variant {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr = input.parse()?;
        let variant_name = input.parse()?;
        let body = input.parse()?;
        Ok(Self {
            attr,
            name: variant_name,
            body,
        })
    }
}

enum VariantBody {
    Struct {
        brace_token: token::Brace,
        src: SourceField,
        ctx: ContextField,
    },
    Tuple {
        paren_token: token::Paren,
        anon_struct: Option<AnonStruct>,
    },
    Unit,
}

impl VariantBody {
    fn get_src_ty(&self) -> Option<TokenStream> {
        match self {
            Self::Struct { src, .. } => match src {
                SourceField::Some { ty, .. } => Some(quote!(#ty)),
                SourceField::None => None,
            },
            Self::Tuple { .. } | Self::Unit => None,
        }
    }

    fn get_struct_body(
        &self,
        ctx_rhs: &impl ToTokens,
        src_rhs: &impl ToTokens,
    ) -> Option<TokenStream> {
        match self {
            Self::Struct {
                brace_token,
                src,
                ctx,
            } => {
                let src_field = match src {
                    SourceField::Some {
                        name, colon_token, ..
                    } => Some(quote!(#name #colon_token #src_rhs)),
                    SourceField::None => None,
                };
                let ctx_field = match ctx {
                    ContextField::Some {
                        name, colon_token, ..
                    } => Some(quote!(#name #colon_token #ctx_rhs)),
                    ContextField::None => None,
                };
                Some(
                    tokens_with(|tokens| {
                        brace_token.surround(tokens, |tokens| {
                            punctuated_tokens::<Token![,], _, _>(
                                tokens,
                                src_field.iter().chain(&ctx_field),
                            )
                        })
                    })
                    .to_token_stream(),
                )
            }
            Self::Tuple {
                paren_token,
                anon_struct,
            } => anon_struct.as_ref().map(|_| {
                tokens_with(|tokens| {
                    paren_token.surround(tokens, |tokens| quote!(#ctx_rhs).to_tokens(tokens))
                })
                .to_token_stream()
            }),
            Self::Unit => None,
        }
    }
}

impl Parse for VariantBody {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Brace) {
            let mut src = SourceField::None;
            let mut ctx = ContextField::None;
            let (brace_token, _) = Braced::parse_with(input, |input| {
                punctuated_parse::<Token![,], _>(input, |input| {
                    input.parse::<Token![@]>()?;
                    let lookhead = input.lookahead1();
                    if lookhead.peek(kw::source) {
                        if let SourceField::Some { .. } = src {
                            return Err(input.error("too many sources"));
                        }
                        input.parse::<kw::source>()?;
                        src = input.parse()?;
                    } else if lookhead.peek(kw::context) {
                        if let ContextField::Some { .. } = ctx {
                            return Err(input.error("too many contextx"));
                        }
                        input.parse::<kw::context>()?;
                        ctx = input.parse()?;
                    } else {
                        return Err(lookhead.error());
                    }
                    Ok(())
                })
            })?;
            Ok(Self::Struct {
                brace_token,
                src,
                ctx,
            })
        } else if input.peek(token::Paren) {
            let mut anon_struct = None;
            let (paren_token, _) = Parened::parse_with(input, |input| {
                punctuated_parse::<Token![,], _>(input, |input| {
                    anon_struct = Some(input.parse()?);
                    Ok(())
                })
            })?;
            Ok(Self::Tuple {
                paren_token,
                anon_struct,
            })
        } else {
            Ok(Self::Unit)
        }
    }
}

enum SourceField {
    Some {
        name: Ident,
        colon_token: Token![:],
        ty: Type,
    },
    None,
}

impl Parse for SourceField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let colon_token = input.parse()?;
        let ty = input.parse()?;
        Ok(Self::Some {
            name,
            colon_token,
            ty,
        })
    }
}

enum ContextField {
    Some {
        name: Ident,
        colon_token: Token![:],
        anon_struct: AnonStruct,
    },
    None,
}

impl Parse for ContextField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        let colon_token = input.parse()?;
        let anon_struct = input.parse()?;
        Ok(Self::Some {
            name,
            colon_token,
            anon_struct,
        })
    }
}

struct AnonStruct {
    attr: Attributes,
    struct_token: Token![struct],
    body: StructBody,
}

impl AnonStruct {
    fn get_struct_def(&self, vis: &Visibility, name: &Ident) -> TokenStream {
        let Self {
            attr,
            struct_token,
            body,
        } = self;
        let body = match body {
            StructBody::Struct(named) => quote!(#named),
            StructBody::Tuple(unnamed) => {
                let content = quote!(#unnamed);
                quote!(#content;)
            }
            StructBody::Unit => quote!(;),
        };
        quote!(#attr #vis #struct_token #name #body)
    }
}

impl Parse for AnonStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr = input.parse()?;
        let struct_token = input.parse()?;
        let body = input.parse()?;
        Ok(Self {
            attr,
            struct_token,
            body,
        })
    }
}

enum StructBody {
    Struct(FieldsNamed),
    Tuple(FieldsUnnamed),
    Unit,
}

impl Parse for StructBody {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Brace) {
            Ok(Self::Struct(input.parse()?))
        } else if input.peek(token::Paren) {
            Ok(Self::Tuple(input.parse()?))
        } else {
            Ok(Self::Unit)
        }
    }
}
