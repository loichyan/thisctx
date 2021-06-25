use crate::utils::{
    AngleBracket, Attributes, Punctuated, StructBodySurround, TokensWith, WithSurround,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::str::FromStr;
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_quote, token, Field, Fields, FieldsNamed, FieldsUnnamed};
use syn::{Ident, Token, Type, Visibility};

pub struct ContextFeildInput {
    pub attrs: Attributes,
    pub name: Option<Ident>,
    pub colon_token: Option<Token![:]>,
    pub body: ContextInput,
}

impl ContextFeildInput {
    pub fn parse(input: ParseStream, attrs: Attributes, named: bool) -> Result<Self> {
        let (name, colon_token) = if named {
            (Some(input.parse()?), Some(input.parse()?))
        } else {
            (None, None)
        };
        let body = input.parse()?;
        Ok(Self {
            attrs,
            name,
            colon_token,
            body,
        })
    }
}

pub struct ContextInput {
    pub attrs: Attributes,
    pub struct_token: Token![struct],
    pub fields: Fields,
}

impl Parse for ContextInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.parse()?;
        let struct_token = input.parse()?;
        let body = if input.peek(token::Brace) {
            Fields::Named(FieldsNamed::parse(input)?)
        } else if input.peek(token::Paren) {
            Fields::Unnamed(FieldsUnnamed::parse(input)?)
        } else {
            Fields::Unit
        };
        Ok(Self {
            attrs,
            struct_token,
            fields: body,
        })
    }
}

pub struct ContextField {
    pub attrs: Attributes,
    pub name: Option<Ident>,
    pub colon_token: Option<Token![:]>,
    pub body: Context,
}

impl From<ContextFeildInput> for ContextField {
    fn from(t: ContextFeildInput) -> Self {
        let ContextFeildInput {
            attrs,
            name,
            colon_token,
            body,
        } = t;
        let body = Context::from(body);
        Self {
            attrs,
            name,
            colon_token,
            body,
        }
    }
}

pub struct Context {
    pub attrs: Attributes,
    pub struct_token: Token![struct],
    pub body: ContextBody,
}

impl Context {
    pub fn to_struct_def(&self, vis: &Visibility, name: &Ident) -> TokenStream {
        let Self {
            attrs,
            struct_token,
            body,
        } = self;
        let generic = body.map_fields_to_generic(ContextBody::GENERIC_BOUNDED_F);
        let struct_body = body.map_fields(ContextBody::STRUCT_BODY_DEF_F);
        let semi = match struct_body.surround {
            StructBodySurround::Brace(..) => None,
            StructBodySurround::Paren(..) | StructBodySurround::None => Some(quote!(;)),
        };
        quote!(#attrs #vis #struct_token #name #generic #struct_body #semi)
    }
}

impl From<ContextInput> for Context {
    fn from(t: ContextInput) -> Self {
        let ContextInput {
            attrs,
            struct_token,
            fields,
        } = t;
        let body = ContextBody::from(fields);
        Self {
            attrs,
            struct_token,
            body,
        }
    }
}

pub struct ContextBody(WithSurround<ContextBodyFields, StructBodySurround>);

impl ContextBody {
    pub const STRUCT_BODY_DEF_F: fn(&ContextBodyField) -> TokenStream =
        |ContextBodyField {
             generic,
             attrs,
             vis,
             ident,
             colon_token,
             ..
         }| quote!(#attrs #vis #ident #colon_token #generic);

    pub const STRUCT_BODY_CONVERTED_FROM_F: fn(
        from: &TokenStream,
        &ContextBodyField,
    ) -> TokenStream = |from,
                        ContextBodyField {
                            ident, colon_token, ..
                        }| {
        let from_field = match ident {
            FieldIdent::Some(ident) => quote!(#ident),
            FieldIdent::None(idx) => TokenStream::from_str(&idx.to_string()).unwrap(),
        };
        let from = from.into_token_stream();
        quote!(#ident #colon_token #from.#from_field.into())
    };

    pub fn map_fields<F>(&self, f: F) -> WithSurround<TokenStream, StructBodySurround>
    where
        F: FnMut(&ContextBodyField) -> TokenStream,
    {
        let content =
            TokensWith::new(|tokens| self.0.content.0.to_token_stream_with(f).to_tokens(tokens))
                .into_token_stream();
        let surround = self.0.surround;
        WithSurround { content, surround }
    }

    pub const GENERIC_TY_F: fn(&ContextBodyField) -> TokenStream =
        |ContextBodyField { ty, .. }| quote!(#ty);

    pub const GENERIC_NAME_F: fn(&ContextBodyField) -> TokenStream =
        |ContextBodyField { generic, .. }| quote!(#generic);

    pub const GENERIC_BOUNDED_F: fn(&ContextBodyField) -> TokenStream =
        |ContextBodyField { generic, ty, .. }| quote!(#generic: Into<#ty>);

    pub fn map_fields_to_generic<F>(&self, f: F) -> Option<WithSurround<TokenStream, AngleBracket>>
    where
        F: FnMut(&ContextBodyField) -> TokenStream,
    {
        if let StructBodySurround::None = self.0.surround {
            return None;
        }
        let WithSurround { content, .. } = self.map_fields(f);
        Some(WithSurround {
            surround: AngleBracket(parse_quote!(<), parse_quote!(>)),
            content,
        })
    }
}

impl From<Fields> for ContextBody {
    fn from(t: Fields) -> Self {
        match t {
            Fields::Named(FieldsNamed { brace_token, named }) => {
                let surround = StructBodySurround::brace(brace_token);
                let content = ContextBodyFields::from(named);
                Self(WithSurround { surround, content })
            }
            Fields::Unnamed(FieldsUnnamed {
                paren_token,
                unnamed,
            }) => {
                let surround = StructBodySurround::paren(paren_token);
                let content = ContextBodyFields::from(unnamed);
                Self(WithSurround { surround, content })
            }
            Fields::Unit => {
                let surround = StructBodySurround::None;
                let content = ContextBodyFields(Punctuated::new());
                Self(WithSurround { surround, content })
            }
        }
    }
}

pub struct ContextBodyFields(Punctuated<ContextBodyField, Token![,]>);

impl From<syn::punctuated::Punctuated<Field, Token![,]>> for ContextBodyFields {
    fn from(t: syn::punctuated::Punctuated<Field, Token![,]>) -> Self {
        let inner = t
            .into_pairs()
            .enumerate()
            .map(|(idx, pair)| {
                let (
                    Field {
                        vis,
                        attrs,
                        ident,
                        colon_token,
                        ty,
                    },
                    punct,
                ) = pair.into_tuple();
                let generic = syn::parse_str(&format!("T{}", idx)).unwrap();
                let attrs = Attributes(attrs);
                let ident = match ident {
                    Some(ident) => FieldIdent::Some(ident),
                    None => FieldIdent::None(idx),
                };
                let field = ContextBodyField {
                    generic,
                    vis,
                    attrs,
                    ident,
                    colon_token,
                    ty,
                };
                match punct {
                    Some(punct) => syn::punctuated::Pair::Punctuated(field, punct),
                    None => syn::punctuated::Pair::End(field),
                }
            })
            .collect();
        Self(Punctuated(inner))
    }
}

pub struct ContextBodyField {
    pub generic: Type,
    pub attrs: Attributes,
    pub vis: Visibility,
    pub ident: FieldIdent,
    pub colon_token: Option<Token![:]>,
    pub ty: Type,
}

pub enum FieldIdent {
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
