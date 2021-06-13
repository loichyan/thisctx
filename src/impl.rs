use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{braced, Attribute, Field, FieldsNamed, Ident, Token};

pub struct ThisCtx(EnumDef);

impl Parse for ThisCtx {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner = input.parse()?;
        Ok(Self(inner))
    }
}

impl ToTokens for ThisCtx {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens)
    }
}

mod kw {
    use syn::custom_keyword;
    custom_keyword!(source);
    custom_keyword!(context);
}

struct EnumDef {
    attr: Vec<Attribute>,
    name: Ident,
    variants: Punctuated<VariantDef, Token![,]>,
}

impl Parse for EnumDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr = input.call(Attribute::parse_outer)?;
        input.parse::<Token![enum]>()?;
        let name = input.parse()?;
        let brace;
        braced!(brace in input);
        let variants = brace.parse_terminated(VariantDef::parse)?;
        Ok(Self {
            attr,
            name,
            variants,
        })
    }
}

impl ToTokens for EnumDef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            attr,
            name,
            variants,
        } = self;
        let ctx_defs = variants.iter().map(VariantDef::to_context_def);
        tokens.extend(quote! (
            #(#attr)*
            enum #name {
                #variants
            }
            #(#ctx_defs)*
        ))
    }
}

struct VariantDef {
    attr: Vec<Attribute>,
    name: Ident,
    body: VariantBody,
}

impl VariantDef {
    fn to_context_def(&self) -> TokenStream {
        let Self { name, body, .. } = self;
        match body {
            VariantBody::Unit => quote!(struct #name;),
            VariantBody::Struct { ctx, .. } => match ctx {
                Some(ctx) => {
                    let attr = &ctx.attr;
                    let body = &ctx.body;
                    quote!(#(#attr)* struct #name #body)
                }
                None => quote!(struct #name;),
            },
        }
    }
}

impl Parse for VariantDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let attr = input.call(Attribute::parse_outer)?;
        let name = input.parse()?;
        let body = input.parse()?;
        Ok(Self { attr, name, body })
    }
}

impl ToTokens for VariantDef {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { attr, name, body } = self;
        let body = match body {
            VariantBody::Unit => quote!(),
            VariantBody::Struct { src, ctx } => {
                let src = src.as_ref().map(|SrcField(src)| quote!(#src));
                let ctx = ctx
                    .as_ref()
                    .map(|CtxField { name: ctx, .. }| quote!(#ctx: #name));
                let fields = src
                    .into_iter()
                    .chain(ctx)
                    .collect::<Punctuated<_, Token![,]>>();
                quote!({ #fields })
            }
        };
        tokens.extend(quote!(#(#attr)* #name #body))
    }
}

enum VariantBody {
    Unit,
    Struct {
        src: Option<SrcField>,
        ctx: Option<CtxField>,
    },
}

impl Parse for VariantBody {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![,]) {
            Ok(Self::Unit)
        } else {
            let mut src = None;
            let mut ctx = None;

            let brace;
            braced!(brace in input);
            loop {
                if brace.is_empty() {
                    break;
                }
                brace.parse::<Token![@]>()?;
                let lookhead = brace.lookahead1();

                if lookhead.peek(kw::source) {
                    if src.is_some() {
                        return Err(brace.error("multi source provided"));
                    }
                    brace.parse::<kw::source>()?;
                    let inner = brace.parse()?;
                    src = Some(inner);
                } else if lookhead.peek(kw::context) {
                    if ctx.is_some() {
                        return Err(brace.error("multi context provided"));
                    }
                    brace.parse::<kw::context>()?;
                    let inner = brace.parse()?;
                    ctx = Some(inner);
                } else {
                    return Err(lookhead.error());
                }

                if brace.is_empty() {
                    break;
                }
                brace.parse::<Token![,]>()?;
            }

            Ok(Self::Struct { src, ctx })
        }
    }
}

struct SrcField(Field);

impl Parse for SrcField {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner = Field::parse_named(input)?;
        Ok(Self(inner))
    }
}

struct CtxField {
    name: Ident,
    attr: Vec<Attribute>,
    body: FieldsNamed,
}

impl Parse for CtxField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let attr = input.call(Attribute::parse_outer)?;
        input.parse::<Token![struct]>()?;
        let body = input.parse()?;
        Ok(Self { name, attr, body })
    }
}
