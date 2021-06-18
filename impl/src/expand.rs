use crate::utils::{
    custom_token, AngleBracket, Attributes, Brace, Punctuated, StructBodySurround, TokensWith,
    WithSurround,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use std::str::FromStr;
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
        self.0.to_tokens(tokens);
        self.0.to_context_def().to_tokens(tokens);
        self.0.to_impl_into_error().to_tokens(tokens);
        self.0.to_from_ctx_for_enum().to_tokens(tokens);
    }
}

struct Enum {
    attrs: Attributes,
    vis: Visibility,
    enum_token: token::Enum,
    name: Ident,
    body: EnumBody,
}

impl Enum {
    // TODO: reutrn Punctuated
    fn to_context_def(&self) -> TokenStream {
        TokensWith::new(|tokens| {
            self.body
                .0
                .content
                .iter()
                .for_each(|variant| variant.to_context_def(&self.vis).to_tokens(tokens))
        })
        .into_token_stream()
    }

    fn to_impl_into_error(&self) -> TokenStream {
        TokensWith::new(|tokens| {
            self.body
                .0
                .content
                .iter()
                .for_each(|variant| variant.to_impl_into_error(&self.name).to_tokens(tokens))
        })
        .into_token_stream()
    }

    fn to_from_ctx_for_enum(&self) -> TokenStream {
        TokensWith::new(|tokens| {
            self.body.0.content.iter().for_each(|variant| {
                variant
                    .to_impl_from_ctx_for_enum(&self.name)
                    .to_tokens(tokens)
            })
        })
        .into_token_stream()
    }
}

impl Parse for Enum {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.parse()?;
        let vis = input.parse()?;
        let enum_token = input.parse()?;
        let name = input.parse()?;
        let body = input.parse()?;
        Ok(Self {
            attrs,
            vis,
            enum_token,
            name,
            body,
        })
    }
}

impl ToTokens for Enum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            attrs,
            vis,
            enum_token,
            name,
            body,
        } = self;
        attrs.to_tokens(tokens);
        vis.to_tokens(tokens);
        enum_token.to_tokens(tokens);
        name.to_tokens(tokens);
        body.to_tokens(tokens);
    }
}

struct EnumBody(WithSurround<Punctuated<Variant, Token![,]>, Brace>);

impl Parse for EnumBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner = input.parse()?;
        Ok(Self(inner))
    }
}

impl ToTokens for EnumBody {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens)
    }
}

struct Variant {
    attrs: Attributes,
    name: Ident,
    body: VariantBody,
}

impl Variant {
    fn to_context_def(&self, vis: &Visibility) -> TokenStream {
        let Self { name, body, .. } = self;
        match body.0.content.ctx.as_ref() {
            Some(ctx) => ctx.anon_struct.to_struct_def(vis, name),
            None => quote!(#vis struct #name;),
        }
    }

    fn to_impl_into_error(&self, enum_name: &Ident) -> TokenStream {
        let Self {
            name: variant_name,
            body,
            ..
        } = self;
        let src_ty = body
            .get_src_ty()
            .map(|ty| quote!(#ty))
            .unwrap_or_else(|| quote!(#NONE_ERROR));
        let src_name = quote!(source);
        let expr_struct_body = body.map_fields(
            |SourceField {
                 name, colon_token, ..
             }| quote!(#name #colon_token #src_name),
            |ContextField {
                 name,
                 colon_token,
                 anon_struct,
                 ..
             }| {
                let from = quote!(self);
                let convert_struct_body = anon_struct
                    .body
                    .map_fields(|field| StructBody::STRUCT_BODY_CONVERTED_FROM_F(&from, field));
                quote!(#name #colon_token #variant_name #convert_struct_body)
            },
        );
        let generic_bounded = body.map_context_fields_to_generic(StructBody::GENERIC_BOUNDED_F);
        let generic_name = body.map_context_fields_to_generic(StructBody::GENERIC_NAME_F);
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

    // TODO: impl from source for enum
    fn to_impl_from_ctx_for_enum(&self, enum_name: &Ident) -> Option<TokenStream> {
        let Self {
            name: variant_name,
            body,
            ..
        } = self;
        if body.get_src_ty().is_some() {
            return None;
        }
        let ctx_name = quote!(context);
        let expr_struct_body = body.map_fields(
            |_| unreachable!("{} shouldn't have source field", variant_name),
            |ContextField {
                 name,
                 colon_token,
                 anon_struct,
                 ..
             }| {
                let convert_struct_body = anon_struct
                    .body
                    .map_fields(|field| StructBody::STRUCT_BODY_CONVERTED_FROM_F(&ctx_name, field));
                quote!(#name #colon_token #variant_name #convert_struct_body)
            },
        );
        let generic_bounded = body.map_context_fields_to_generic(StructBody::GENERIC_BOUNDED_F);
        let generic_name = body.map_context_fields_to_generic(StructBody::GENERIC_NAME_F);
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

impl ToTokens for Variant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            attrs,
            name: variant_name,
            body,
        } = self;
        let struct_body = body.map_fields(
            SourceField::to_token_stream,
            |ContextField {
                 attrs,
                 name,
                 colon_token,
                 anon_struct,
             }| {
                let generic = anon_struct
                    .body
                    .map_fields_to_generic(StructBody::GENERIC_TY_F);
                quote!(#attrs #name #colon_token #variant_name #generic)
            },
        );
        attrs.to_tokens(tokens);
        variant_name.to_tokens(tokens);
        struct_body.to_tokens(tokens);
    }
}

struct VariantBody(WithSurround<VariantFields, StructBodySurround>);

impl VariantBody {
    fn get_src_ty(&self) -> Option<&Type> {
        self.0.content.src.as_ref().map(|src| &src.ty)
    }

    fn map_context_fields_to_generic<F>(
        &self,
        f: F,
    ) -> Option<WithSurround<TokenStream, AngleBracket>>
    where
        F: FnMut(&GenericField) -> TokenStream,
    {
        self.0
            .content
            .ctx
            .as_ref()
            .map(|ctx| ctx.anon_struct.body.map_fields_to_generic(f))
    }

    fn map_fields<F1, F2>(
        &self,
        map_src: F1,
        map_ctx: F2,
    ) -> WithSurround<TokenStream, StructBodySurround>
    where
        F1: FnOnce(&SourceField) -> TokenStream,
        F2: FnOnce(&ContextField) -> TokenStream,
    {
        let src_mapped = self.0.content.src.as_ref().map(|src| map_src(src));
        let ctx_mapped = self.0.content.ctx.as_ref().map(|ctx| map_ctx(ctx));
        let content = src_mapped
            .into_iter()
            .chain(ctx_mapped)
            .collect::<Punctuated<_, Token![,]>>()
            .to_token_stream();
        let surround = self.0.surround;
        WithSurround { content, surround }
    }
}

impl Parse for VariantBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let (surround, content) = StructBodySurround::parse_with(
            input,
            |input| VariantFields::parse(true, input),
            |input| VariantFields::parse(false, input),
            |_| {
                Ok(VariantFields {
                    src: None,
                    ctx: None,
                })
            },
        )?;
        Ok(Self(WithSurround { surround, content }))
    }
}

// TODO: use Field-Like
struct VariantFields {
    src: Option<SourceField>,
    ctx: Option<ContextField>,
}

impl VariantFields {
    fn parse(named: bool, input: ParseStream) -> Result<Self> {
        let parse_name_colon_token = || -> Result<(Option<Ident>, Option<Token![:]>)> {
            match named {
                true => Ok((Some(input.parse()?), Some(input.parse()?))),
                false => Ok((None, None)),
            }
        };

        let mut src = None;
        let mut ctx = None;
        Punctuated::<_, Token![,]>::visit_parse_with(input, |input| {
            let attrs = input.parse()?;
            input.parse::<Token![@]>()?;
            let lookhead = input.lookahead1();
            if lookhead.peek(kw::source) {
                if src.is_some() {
                    return Err(input.error("too many sources"));
                }
                input.parse::<kw::source>()?;
                let (name, colon_token) = parse_name_colon_token()?;
                let ty = input.parse()?;
                src = Some(SourceField {
                    attrs,
                    name,
                    colon_token,
                    ty,
                });
            } else if lookhead.peek(kw::context) {
                if ctx.is_some() {
                    return Err(input.error("too many contextx"));
                }
                input.parse::<kw::context>()?;
                let (name, colon_token) = parse_name_colon_token()?;
                let anon_struct = input.parse()?;
                ctx = Some(ContextField {
                    attrs,
                    name,
                    colon_token,
                    anon_struct,
                });
            } else {
                return Err(lookhead.error());
            }
            Ok(())
        })?;

        Ok(Self { src, ctx })
    }
}

struct SourceField {
    attrs: Attributes,
    name: Option<Ident>,
    colon_token: Option<Token![:]>,
    ty: Type,
}

impl ToTokens for SourceField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            attrs,
            name,
            colon_token,
            ty,
        } = self;
        attrs.to_tokens(tokens);
        name.to_tokens(tokens);
        colon_token.to_tokens(tokens);
        ty.to_tokens(tokens);
    }
}

struct ContextField {
    attrs: Attributes,
    name: Option<Ident>,
    colon_token: Option<Token![:]>,
    anon_struct: AnonStruct,
}

struct AnonStruct {
    attrs: Attributes,
    struct_token: Token![struct],
    body: StructBody,
}

impl AnonStruct {
    fn to_struct_def(&self, vis: &Visibility, name: &Ident) -> TokenStream {
        let Self {
            attrs,
            struct_token,
            body,
        } = self;
        let generic = body.map_fields_to_generic(StructBody::GENERIC_BOUNDED_F);
        let struct_body = body.map_fields(StructBody::STRUCT_BODY_DEF_F);
        let semi = match struct_body.surround {
            StructBodySurround::Brace(..) => None,
            StructBodySurround::Paren(..) | StructBodySurround::None => Some(quote!(;)),
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

struct StructBody(WithSurround<GenericFields, StructBodySurround>);

impl StructBody {
    const STRUCT_BODY_DEF_F: fn(&GenericField) -> TokenStream =
        |GenericField {
             generic,
             attrs,
             vis,
             ident,
             colon_token,
             ..
         }| quote!(#attrs #vis #ident #colon_token #generic);

    const STRUCT_BODY_CONVERTED_FROM_F: fn(from: &TokenStream, &GenericField) -> TokenStream =
        |from,
         GenericField {
             ident, colon_token, ..
         }| {
            let from_field = match ident {
                FieldIdent::Some(ident) => quote!(#ident),
                FieldIdent::None(idx) => TokenStream::from_str(&idx.to_string()).unwrap(),
            };
            let from = from.into_token_stream();
            quote!(#ident #colon_token #from.#from_field.into())
        };

    // TODO: return Punctuated
    fn map_fields<F>(&self, f: F) -> WithSurround<TokenStream, StructBodySurround>
    where
        F: FnMut(&GenericField) -> TokenStream,
    {
        let content =
            TokensWith::new(|tokens| self.0.content.0.to_token_stream_with(f).to_tokens(tokens))
                .into_token_stream();
        let surround = self.0.surround;
        WithSurround { content, surround }
    }

    const GENERIC_TY_F: fn(&GenericField) -> TokenStream = |GenericField { ty, .. }| quote!(#ty);

    const GENERIC_NAME_F: fn(&GenericField) -> TokenStream =
        |GenericField { generic, .. }| quote!(#generic);

    const GENERIC_BOUNDED_F: fn(&GenericField) -> TokenStream =
        |GenericField { generic, ty, .. }| quote!(#generic: Into<#ty>);

    fn map_fields_to_generic<F>(&self, f: F) -> WithSurround<TokenStream, AngleBracket>
    where
        F: FnMut(&GenericField) -> TokenStream,
    {
        let WithSurround { content, .. } = self.map_fields(f);
        // TODO: no generic for unit struct
        WithSurround {
            surround: AngleBracket(parse_quote!(<), parse_quote!(>)),
            content,
        }
    }
}

impl Parse for StructBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let (surround, content) = StructBodySurround::parse_with(
            input,
            |input| GenericFields::parse(true, input),
            |input| GenericFields::parse(false, input),
            |_| Ok(GenericFields(Punctuated::new())),
        )?;
        Ok(Self(WithSurround { surround, content }))
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
                attrs: attrss,
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
    attrs: Attributes,
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
