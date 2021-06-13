use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{braced, token, Attribute, FieldsNamed, FieldsUnnamed, Ident, Token, Type};

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
            name: err_name,
            variants,
        } = self;
        // enum definition
        tokens.extend(quote! (
            #(#attr)*
            enum #err_name {
                #variants
            }
        ));

        // context definition
        for variant in variants {
            let VariantDef {
                name: var_name,
                body,
                ..
            } = variant;
            let (def, imp) = match body {
                VariantBody::Unit => {
                    let def = quote!(struct #var_name;);
                    let imp = quote!(
                        impl thisctx::private::IntoError for #var_name {
                            type Error = #err_name;
                            type Source = thisctx::private::NoneError;

                            fn into_error(self, _: Self::Source) -> Self::Error {
                                Self::Error::#var_name
                            }
                        }
                    );
                    (def, imp)
                }
                VariantBody::Struct { ctx, src } => {
                    let (def, ctx_field) = match ctx {
                        Some(CtxField { attr, body, name }) => {
                            let def = match body {
                                CtxBody::Struct(body) => quote!(#(#attr)* struct #var_name #body),
                                CtxBody::Tuple(body) => quote!(#(#attr)* struct #var_name #body;),
                            };
                            (def, quote!(#name: self))
                        }
                        None => (quote!(struct #var_name;), quote!()),
                    };
                    let imp_tail = match src {
                        Some(SrcField { name, ty }) => quote!(
                            type Source = #ty;

                            fn into_error(self, source: Self::Source) -> Self::Error {
                                let #name = source;
                                Self::Error::#var_name { #name, #ctx_field }
                            }
                        ),
                        None => quote!(
                            type Source = thisctx::private::NoneError;

                            fn into_error(self, _: Self::Source) -> Self::Error {
                                Self::Error::#var_name { #ctx_field }
                            }
                        ),
                    };
                    let imp = quote!(
                        impl thisctx::private::IntoError for #var_name {
                            type Error = #err_name;
                            #imp_tail
                        }
                    );
                    (def, imp)
                }
            };
            tokens.extend(def);
            tokens.extend(imp);
        }
    }
}

struct VariantDef {
    attr: Vec<Attribute>,
    name: Ident,
    body: VariantBody,
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
        let def = match body {
            VariantBody::Unit => quote!(#(#attr)* #name),
            VariantBody::Struct { src, ctx } => {
                let src = src.as_ref().map(|src| quote!(#src));
                let ctx = ctx
                    .as_ref()
                    .map(|CtxField { name: ctx, .. }| quote!(#ctx: #name));
                let fields = src.into_iter().chain(ctx);
                quote!(#(#attr)* #name { #(#fields,)* })
            }
        };
        tokens.extend(def)
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

struct SrcField {
    name: Ident,
    ty: Type,
}

impl Parse for SrcField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;
        let ty = input.parse()?;
        Ok(Self { name, ty })
    }
}

impl ToTokens for SrcField {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { name, ty } = self;
        tokens.extend(quote!(#name: #ty));
    }
}

struct CtxField {
    name: Ident,
    attr: Vec<Attribute>,
    body: CtxBody,
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

enum CtxBody {
    Struct(FieldsNamed),
    Tuple(FieldsUnnamed),
}

impl Parse for CtxBody {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookhead = input.lookahead1();
        if lookhead.peek(token::Brace) {
            let inner = input.parse()?;
            Ok(Self::Struct(inner))
        } else if lookhead.peek(token::Paren) {
            let inner = input.parse()?;
            Ok(Self::Tuple(inner))
        } else {
            Err(lookhead.error())
        }
    }
}
