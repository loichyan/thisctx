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
        self.0.to_tokens(tokens);
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
            attr: enum_attr,
            name: enum_name,
            variants,
        } = self;
        // enum definition
        quote! (
            #(#enum_attr)*
            enum #enum_name {
                #variants
            }
        )
        .to_tokens(tokens);

        // contexts struct definition
        for variant in variants {
            let VariantDef {
                name: variant_name,
                body: variant_body,
                ..
            } = variant;
            let (ctx_def, ctx_impl) = match variant_body {
                // unit variant
                VariantBody::Unit => {
                    let ctx_def = quote!(struct #variant_name;);
                    let ctx_impl = quote!(
                        impl thisctx::private::IntoError for #variant_name {
                            type Error = #enum_name;
                            type Source = thisctx::private::NoneError;

                            fn into_error(self, _: Self::Source) -> Self::Error {
                                Self::Error::#variant_name
                            }
                        }
                    );
                    (ctx_def, ctx_impl)
                }
                // struct variant
                VariantBody::Struct {
                    ctx: ctx_field,
                    src: src_field,
                } => {
                    // context definition, field-value pair
                    let (ctx_def, ctx_field_val) = match ctx_field {
                        Some(CtxField {
                            name: ctx_name,
                            attr: ctx_attr,
                            body: ctx_body,
                        }) => {
                            let ctx_def = match ctx_body {
                                CtxBody::Struct(ctx_body) => {
                                    quote!(#(#ctx_attr)* struct #variant_name #ctx_body)
                                }
                                CtxBody::Tuple(ctx_body) => {
                                    quote!(#(#ctx_attr)* struct #variant_name #ctx_body;)
                                }
                            };
                            (ctx_def, quote!(#ctx_name: self,))
                        }
                        None => (quote!(struct #variant_name;), quote!()),
                    };
                    // source type, parameter name, field-value pair
                    let (src_ty, src_param_name, src_field_val) = match src_field {
                        Some(SrcField {
                            name: src_name,
                            ty: src_ty,
                        }) => (quote!(#src_ty), quote!(source), quote!(#src_name: source,)),
                        None => (quote!(thisctx::private::NoneError), quote!(_), quote!()),
                    };
                    let ctx_impl = quote!(
                        impl thisctx::private::IntoError for #variant_name {
                            type Error = #enum_name;
                            type Source = #src_ty;

                            fn into_error(self, #src_param_name: Self::Source) -> Self::Error {
                                Self::Error::#variant_name { #ctx_field_val #src_field_val }
                            }
                        }
                    );
                    (ctx_def, ctx_impl)
                }
            };
            ctx_def.to_tokens(tokens);
            ctx_impl.to_tokens(tokens);
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
        def.to_tokens(tokens);
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
        quote!(#name: #ty).to_tokens(tokens);
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
