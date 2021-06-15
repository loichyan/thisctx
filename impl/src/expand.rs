use crate::utils::{punctuated_parse, punctuated_tokens, tokens_with, Attributes, Braced, Parened};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{punctuated::Punctuated, Ident, Token, Type};
use syn::{token, FieldsNamed, FieldsUnnamed};

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
    enum_token: token::Enum,
    name: Ident,
    brace_token: token::Brace,
    variants: Punctuated<Variant, Token![,]>,
}

impl Enum {
    fn gen_enum_def(&self) -> TokenStream {
        let Self {
            attr,
            enum_token,
            name,
            brace_token,
            variants,
        } = self;
        let body = tokens_with(|tokens| {
            brace_token.surround(tokens, |tokens| {
                punctuated_tokens(
                    tokens,
                    quote!(,),
                    variants.iter().map(Variant::gen_variant_def),
                );
            })
        });
        quote!(#attr #enum_token #name #body)
    }

    fn gen_context_def(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants
                .iter()
                .for_each(|variant| variant.gen_context_def().to_tokens(tokens));
        })
        .to_token_stream()
    }

    fn gen_impl_into_error(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants
                .iter()
                .for_each(|variant| variant.gen_impl_into_error(&self.name).to_tokens(tokens));
        })
        .to_token_stream()
    }

    fn gen_from_ctx_for_enum(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants.iter().for_each(|variant| {
                variant
                    .gen_impl_from_ctx_for_enum(&self.name)
                    .to_tokens(tokens)
            });
        })
        .to_token_stream()
    }
}

impl Parse for Enum {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr = input.parse()?;
        let enum_token = input.parse()?;
        let name = input.parse()?;
        let (brace_token, variants) =
            Braced::parse_with(input, |input| input.parse_terminated(Variant::parse))?;
        Ok(Self {
            attr,
            enum_token,
            name,
            brace_token,
            variants,
        })
    }
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.gen_enum_def().to_tokens(tokens);
        self.gen_context_def().to_tokens(tokens);
        self.gen_impl_into_error().to_tokens(tokens);
        self.gen_from_ctx_for_enum().to_tokens(tokens);
    }
}

struct Variant {
    attr: Attributes,
    name: Ident,
    body: VariantBody,
}

impl Variant {
    fn gen_variant_def(&self) -> TokenStream {
        let Self {
            body,
            name: variant_name,
            attr,
        } = self;
        let body = match body {
            VariantBody::Struct {
                brace_token,
                src,
                ctx,
            } => {
                let src_field = match src {
                    SourceField::Some {
                        name,
                        colon_token,
                        ty,
                    } => Some(quote!(#name #colon_token #ty)),
                    SourceField::None => None,
                };
                let ctx_field = match ctx {
                    ContextField::Some {
                        name, colon_token, ..
                    } => Some(quote!(#name #colon_token #variant_name)),

                    ContextField::None => None,
                };
                tokens_with(|tokens| {
                    brace_token.surround(tokens, |tokens| {
                        punctuated_tokens(tokens, quote!(,), src_field.iter().chain(&ctx_field))
                    })
                })
                .to_token_stream()
            }
            VariantBody::Tuple { paren_token, .. } => tokens_with(|tokens| {
                paren_token.surround(tokens, |tokens| variant_name.to_tokens(tokens))
            })
            .to_token_stream(),
            VariantBody::Unit => quote!(),
        };
        quote!(#attr #variant_name #body)
    }

    fn gen_context_def(&self) -> TokenStream {
        let Self { name, body, .. } = self;
        match body {
            VariantBody::Struct { ctx, .. } => match ctx {
                ContextField::Some { anon_struct, .. } => anon_struct.gen_struct_def(name),
                ContextField::None => quote!(struct #name;),
            },
            VariantBody::Tuple { anno_struct, .. } => anno_struct.gen_struct_def(name),
            VariantBody::Unit => quote!(struct #name;),
        }
    }

    fn gen_variant_expr_body(&self) -> Option<TokenStream> {
        match &self.body {
            VariantBody::Struct {
                brace_token,
                src,
                ctx,
            } => {
                let src_field = match src {
                    SourceField::Some { name, .. } => Some(quote!(#name)),
                    SourceField::None => None,
                };
                let ctx_field = match ctx {
                    ContextField::Some { name, .. } => Some(quote!(#name)),
                    ContextField::None => None,
                };
                let gen = tokens_with(|tokens| {
                    brace_token.surround(tokens, |tokens| {
                        punctuated_tokens(tokens, quote!(,), src_field.iter().chain(&ctx_field))
                    })
                })
                .to_token_stream();
                Some(gen)
            }
            VariantBody::Tuple { paren_token, .. } => Some(
                tokens_with(|tokens| {
                    paren_token.surround(tokens, |tokens| quote!(inner).to_tokens(tokens))
                })
                .to_token_stream(),
            ),
            VariantBody::Unit => None,
        }
    }

    fn gen_impl_into_error(&self, enum_name: &Ident) -> TokenStream {
        let Self {
            name: variant_name,
            body,
            ..
        } = self;
        let (src_name, src_ty, ctx_assign_stmt) = match body {
            VariantBody::Struct { src, ctx, .. } => {
                let (src_name, src_ty) = match src {
                    SourceField::Some { name, ty, .. } => (quote!(#name), quote!(#ty)),
                    SourceField::None => (quote!(_), quote!(#NONE_ERROR)),
                };
                let ctx_assign_stmt = match ctx {
                    ContextField::Some { name, .. } => quote!(let #name = self;),
                    ContextField::None => quote!(),
                };
                (src_name, src_ty, ctx_assign_stmt)
            }
            VariantBody::Tuple { .. } => {
                (quote!(_), quote!(#NONE_ERROR), quote!(let inner = self;))
            }
            VariantBody::Unit => (quote!(_), quote!(#NONE_ERROR), quote!()),
        };
        let variant_expr = match self.gen_variant_expr_body() {
            Some(body) => quote!(Self::Error::#variant_name #body),
            None => quote!(Self::Error::#variant_name),
        };
        quote!(
            impl #INTO_ERROR for #variant_name {
                type Error = #enum_name;
                type Source = #src_ty;

                fn into_error(self, #src_name: Self::Source) -> Self::Error {
                    #ctx_assign_stmt
                    #variant_expr
                }
            }
        )
    }

    fn gen_impl_from_ctx_for_enum(&self, enum_name: &Ident) -> Option<TokenStream> {
        let Self {
            name: variant_name,
            body,
            ..
        } = self;
        let ctx_name = match body {
            VariantBody::Struct { src, ctx, .. } => match src {
                SourceField::Some { .. } => return None,
                SourceField::None => match ctx {
                    ContextField::Some { name, .. } => quote!(#name),
                    ContextField::None => quote!(_),
                },
            },
            VariantBody::Tuple { .. } => quote!(inner),
            VariantBody::Unit => quote!(_),
        };
        let variant_expr = match self.gen_variant_expr_body() {
            Some(body) => quote!(Self::#variant_name #body),
            None => quote!(Self::#variant_name),
        };
        let gen = quote!(
            impl From<#variant_name> for #enum_name {
                fn from(#ctx_name: #variant_name) -> Self {
                    #variant_expr
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
        anno_struct: AnonStruct,
    },
    Unit,
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
            let (paren_token, anno_struct) = Parened::parse_with(input, AnonStruct::parse)?;
            Ok(Self::Tuple {
                paren_token,
                anno_struct,
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
    fn gen_struct_def(&self, name: &Ident) -> TokenStream {
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
        quote!(#attr #struct_token #name #body)
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
