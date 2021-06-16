use std::str::FromStr;

use crate::utils::{
    custom_token, tokens_with, AngleBracket, Attributes, Brace, Paren, ParseWith, Punctuated,
    Surround, SurroundEnum, WithSurround,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::parse_quote;
use syn::{token, Ident, Token, Type, Visibility};

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
    attrs: Attributes,
    vis: Visibility,
    enum_token: token::Enum,
    name: Ident,
    brace_token: Brace,
    variants: Punctuated<Variant, Token![,]>,
}

impl Enum {
    fn get_enum_def(&self) -> TokenStream {
        let Self {
            attrs,
            vis,
            enum_token,
            name,
            brace_token,
            variants,
        } = self;
        let body = tokens_with(|tokens| {
            brace_token.surround(tokens, |tokens| {
                variants
                    .to_tokens_with(Variant::get_variant_def)
                    .to_tokens(tokens)
            })
        });
        quote!(#attrs #vis #enum_token #name #body)
    }

    fn get_context_def(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants
                .iter()
                .for_each(|variant| variant.get_context_def(&self.vis).to_tokens(tokens))
        })
        .to_token_stream()
    }

    fn get_impl_into_error(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants
                .iter()
                .for_each(|variant| variant.get_impl_into_error(&self.name).to_tokens(tokens))
        })
        .to_token_stream()
    }

    fn get_from_ctx_for_enum(&self) -> TokenStream {
        tokens_with(|tokens| {
            self.variants.iter().for_each(|variant| {
                variant
                    .get_impl_from_ctx_for_enum(&self.name)
                    .to_tokens(tokens)
            })
        })
        .to_token_stream()
    }
}

impl Parse for Enum {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.parse()?;
        let vis = input.parse()?;
        let enum_token = input.parse()?;
        let name = input.parse()?;
        let (brace_token, variants) = Brace::parse_with(input, Punctuated::parse)?;
        Ok(Self {
            attrs,
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
    attrs: Attributes,
    name: Ident,
    body: VariantBody,
}

impl Variant {
    fn get_variant_def(&self) -> TokenStream {
        let Self { attrs, name, body } = self;
        let generic = body.get_context_generic_with(StructBody::get_generic_ty_f);
        let struct_body = body.get_struct_body(
            &quote!(#name #generic),
            &tokens_with(|tokens| body.get_src_ty().to_tokens(tokens)),
        );
        quote!(#attrs #name #struct_body)
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
            &tokens_with(|tokens| {
                let from = quote!(self);
                let convert_struct_body = body.get_context_struct_body_with(|field| {
                    StructBody::get_convert_struct_body_f(&from, field)
                });
                quote!(#variant_name #convert_struct_body).to_tokens(tokens)
            }),
            &tokens_with(|tokens| src_name.to_tokens(tokens)),
        );
        let generic_bounded = body.get_context_generic_with(StructBody::get_generic_bounded_f);
        let generic_name = body.get_context_generic_with(StructBody::get_generic_name_f);
        quote!(
            #[allow(unused)]
            impl #generic_bounded #INTO_ERROR for #variant_name #generic_name {
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
        let expr_struct_body = body.get_struct_body(
            &tokens_with(|tokens| {
                let convert_struct_body = body.get_context_struct_body_with(|field| {
                    StructBody::get_convert_struct_body_f(&ctx_name, field)
                });
                quote!(#variant_name #convert_struct_body).to_tokens(tokens)
            }),
            &tokens_with(|_| ()),
        );
        let generic_bounded = body.get_context_generic_with(StructBody::get_generic_bounded_f);
        let generic_name = body.get_context_generic_with(StructBody::get_generic_name_f);
        let gen = quote!(
            #[allow(unused)]
            impl #generic_bounded From<#variant_name #generic_name> for #enum_name {
                fn from(#ctx_name: #variant_name #generic_name) -> Self {
                    Self::#variant_name #expr_struct_body
                }
            }
        );
        Some(gen)
    }
}

impl Parse for Variant {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.parse()?;
        let name = input.parse()?;
        let body = input.parse()?;
        Ok(Self { attrs, name, body })
    }
}

enum VariantBody {
    Struct {
        brace_token: Brace,
        src: SourceField,
        ctx: ContextField,
    },
    Tuple {
        paren_token: Paren,
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

    fn get_context_generic_with<F>(&self, f: F) -> Option<TokenStream>
    where
        F: FnMut(&GenericField) -> TokenStream,
    {
        match self {
            Self::Struct { ctx, .. } => match ctx {
                ContextField::Some { anon_struct, .. } => anon_struct.body.get_generic_with(f),
                ContextField::None => None,
            },
            Self::Tuple { anon_struct, .. } => anon_struct
                .as_ref()
                .map(|anon_struct| anon_struct.body.get_generic_with(f))
                .flatten(),
            Self::Unit => None,
        }
    }

    fn get_context_struct_body_with<F>(&self, f: F) -> Option<TokenStream>
    where
        F: FnMut(&GenericField) -> TokenStream,
    {
        match self {
            Self::Struct { ctx, .. } => match ctx {
                ContextField::Some { anon_struct, .. } => {
                    Some(anon_struct.body.get_struct_body(f).to_token_stream())
                }
                ContextField::None => None,
            },
            Self::Tuple { anon_struct, .. } => anon_struct
                .as_ref()
                .map(|anon_struct| anon_struct.body.get_struct_body(f).to_token_stream()),
            Self::Unit => None,
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
                    tokens_with(move |tokens| {
                        brace_token.surround(tokens, |tokens| {
                            src_field
                                .iter()
                                .chain(&ctx_field)
                                .collect::<Punctuated<_, Token![,]>>()
                                .to_tokens(tokens)
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
                    paren_token.surround(tokens, |tokens| {
                        std::iter::once(quote!(#ctx_rhs))
                            .collect::<Punctuated<_, Token![,]>>()
                            .to_tokens(tokens)
                    })
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
            let (brace_token, _) = Brace::parse_with(input, |input| {
                Punctuated::<_, Token![,]>::visit_parse_with(input, |input| {
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
            let (paren_token, _) = Paren::parse_with(input, |input| {
                Punctuated::<_, Token![,]>::visit_parse_with(input, |input| {
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
    attrs: Attributes,
    struct_token: Token![struct],
    body: StructBody,
}

impl AnonStruct {
    fn get_struct_def(&self, vis: &Visibility, name: &Ident) -> TokenStream {
        let Self {
            attrs,
            struct_token,
            body,
        } = self;
        let generic = body.get_generic_with(StructBody::get_generic_bounded_f);
        let struct_body = body.get_struct_body(
            |GenericField {
                 generic,
                 attrss,
                 vis,
                 ident,
                 colon_token,
                 ..
             }| quote!(#attrss #vis #ident #colon_token #generic),
        );
        let semi = match struct_body.as_ref() {
            Some(WithSurround { surround, .. }) => match surround {
                SurroundEnum::Brace(..) => None,
                SurroundEnum::Paren(..) => Some(quote!(;)),
            },
            None => Some(quote!(;)),
        };
        quote!(#attrs #vis #struct_token #name #generic #struct_body #semi)
    }
}

impl Parse for AnonStruct {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.parse()?;
        let struct_token = input.parse()?;
        let body = input.parse()?;
        Ok(Self {
            attrs,
            struct_token,
            body,
        })
    }
}

enum StructBody {
    Struct {
        brace_token: Brace,
        named: GenericFields,
    },
    Tuple {
        paren_token: Paren,
        unnamed: GenericFields,
    },
    Unit,
}

impl StructBody {
    fn get_struct_body<F>(
        &self,
        f: F,
    ) -> Option<WithSurround<Punctuated<TokenStream, Token![,]>, SurroundEnum>>
    where
        F: FnMut(&GenericField) -> TokenStream,
    {
        match self {
            Self::Struct { brace_token, named } => Some(WithSurround {
                surround: SurroundEnum::Brace(brace_token.clone()),
                content: named.0.iter().map(f).collect(),
            }),
            Self::Tuple {
                paren_token,
                unnamed,
            } => Some(WithSurround {
                surround: SurroundEnum::Paren(paren_token.clone()),
                content: unnamed.0.iter().map(f).collect(),
            }),
            Self::Unit => None,
        }
    }

    fn get_convert_struct_body_f(from: &TokenStream, field: &GenericField) -> TokenStream {
        let GenericField {
            ident, colon_token, ..
        } = field;
        let from_field = match ident {
            FieldIdent::Some(ident) => quote!(#ident),
            FieldIdent::None(idx) => TokenStream::from_str(&idx.to_string()).unwrap(),
        };
        quote!(#ident #colon_token #from.#from_field.into())
    }

    fn get_generic_with<F>(&self, f: F) -> Option<TokenStream>
    where
        F: FnMut(&GenericField) -> TokenStream,
    {
        self.get_struct_body(f).map(|WithSurround { content, .. }| {
            WithSurround {
                surround: AngleBracket(parse_quote!(<), parse_quote!(>)),
                content,
            }
            .to_token_stream()
        })
    }

    fn get_generic_bounded_f(field: &GenericField) -> TokenStream {
        let GenericField { generic, ty, .. } = field;
        quote!(#generic: Into<#ty>)
    }

    fn get_generic_name_f(field: &GenericField) -> TokenStream {
        let GenericField { generic, .. } = field;
        quote!(#generic)
    }

    fn get_generic_ty_f(field: &GenericField) -> TokenStream {
        let GenericField { ty, .. } = field;
        quote!(#ty)
    }
}

impl Parse for StructBody {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Brace) {
            let (brace_token, named) =
                Brace::parse_with(input, |input| GenericFields::parse(true, input))?;
            Ok(Self::Struct { brace_token, named })
        } else if input.peek(token::Paren) {
            let (paren_token, unnamed) =
                Paren::parse_with(input, |input| GenericFields::parse(false, input))?;
            Ok(Self::Tuple {
                paren_token,
                unnamed,
            })
        } else {
            Ok(Self::Unit)
        }
    }
}

struct GenericFields(Punctuated<GenericField, Token![,]>);

impl GenericFields {
    fn parse(named: bool, input: ParseStream) -> Result<Self> {
        let mut idx = 0;
        Ok(Self(Punctuated::parse_with(input, |input| {
            let generic = custom_token(&format!("T{}", idx));
            let attrss = input.parse()?;
            let vis = input.parse()?;
            let (ident, colon_token) = match named {
                true => (FieldIdent::Some(input.parse()?), Some(input.parse()?)),
                false => (FieldIdent::None(idx), None),
            };
            let ty = input.parse()?;
            idx += 1;
            Ok(GenericField {
                generic,
                attrss,
                vis,
                ident,
                colon_token,
                ty,
            })
        })?))
    }
}

struct GenericField {
    generic: Type,
    attrss: Attributes,
    vis: Visibility,
    ident: FieldIdent,
    colon_token: Option<Token![:]>,
    ty: Type,
}

enum FieldIdent {
    Some(Ident),
    None(usize),
}

impl ToTokens for FieldIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Some(ident) => ident.to_tokens(tokens),
            Self::None(_) => (),
        }
    }
}
