use crate::{
    ast::{Enum, Field, Input, Struct, Variant},
    attr::{Attrs, Suffix},
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{punctuated::Punctuated, DeriveInput, Fields, Ident, Member, Result, Token, Visibility};

const DEFAULT_SUFFIX: &str = "Context";

pub fn derive(node: &DeriveInput) -> Result<TokenStream> {
    let input = Input::from_syn(node)?;
    Ok(match input {
        Input::Struct(input) => impl_struct(input).to_token_stream(),
        Input::Enum(input) => impl_enum(input),
    })
}

pub fn impl_struct(input: Struct) -> Option<TokenStream> {
    macro_rules! attr {
        ($ident:ident) => {{
            input.attrs.thisctx.$ident.as_ref()
        }};
    }
    if input.attrs.is_transparent() {
        return None;
    }

    let ident = &input.original.ident;
    let error = input
        .attrs
        .thisctx
        .into
        .as_ref()
        .map(<_>::to_token_stream)
        .unwrap_or_else(|| ident.to_token_stream());
    let attr = quote_attr(input.attrs.thisctx.attr.iter());
    Some(
        Context {
            error: &error,
            vis: attr!(visibility).unwrap_or(&input.original.vis),
            ident,
            suffix: attr!(suffix),
            fields: &input.fields,
            original_fields: &input.data.fields,
            unit: attr!(unit).copied(),
            attr: &attr,
        }
        .impl_into_error(quote!(#ident)),
    )
}

pub fn impl_enum(input: Enum) -> TokenStream {
    input
        .variants
        .iter()
        .map(|variant| input.impl_variant(variant).to_token_stream())
        .collect()
}

impl<'a> Enum<'a> {
    fn impl_variant(&self, input: &Variant) -> Option<TokenStream> {
        macro_rules! attr {
            ($ident:ident) => {{
                input
                    .attrs
                    .thisctx
                    .$ident
                    .as_ref()
                    .or(self.attrs.thisctx.$ident.as_ref())
            }};
        }
        if input.attrs.is_transparent() {
            return None;
        }

        let enum_ident = &self.original.ident;
        let ident = &input.original.ident;
        let error = self
            .attrs
            .thisctx
            .into
            .as_ref()
            .map(<_>::to_token_stream)
            .unwrap_or_else(|| enum_ident.to_token_stream());
        let attr = quote_attr(
            input
                .attrs
                .thisctx
                .attr
                .iter()
                .chain(self.attrs.thisctx.attr.iter()),
        );
        Some(
            Context {
                error: &error,
                vis: attr!(visibility).unwrap_or(&self.original.vis),
                ident,
                suffix: attr!(suffix),
                fields: &input.fields,
                original_fields: &input.original.fields,
                unit: attr!(unit).copied(),
                attr: &attr,
            }
            .impl_into_error(quote!(#enum_ident::#ident)),
        )
    }
}

impl<'a> Attrs<'a> {
    fn is_transparent(&self) -> bool {
        self.error
            .as_ref()
            .and_then(|e| e.transparent.as_ref())
            .is_some()
    }
}

fn find_source_field(fields: &[Field]) -> usize {
    for (i, field) in fields.iter().enumerate() {
        if field.attrs.source.is_some() {
            return i;
        }
    }
    for (i, field) in fields.iter().enumerate() {
        match &field.original.ident {
            Some(ident) if ident == "source" => {
                return i;
            }
            _ => (),
        }
    }
    fields.len()
}

struct Context<'a> {
    error: &'a TokenStream,
    vis: &'a Visibility,
    ident: &'a Ident,
    suffix: Option<&'a Suffix>,
    fields: &'a [Field<'a>],
    original_fields: &'a Fields,
    unit: Option<bool>,
    attr: &'a TokenStream,
}

fn quote_attr<'a>(attrs: impl IntoIterator<Item = &'a TokenStream>) -> TokenStream {
    attrs.into_iter().map(|tokens| quote!(#[#tokens])).collect()
}

impl<'a> Context<'a> {
    fn impl_into_error(&self, expr_struct_path: TokenStream) -> TokenStream {
        type CommaPunctuated<T> = Punctuated<T, Token![,]>;

        let mut source_field_index = find_source_field(self.fields);
        let source_field = self.fields.get(source_field_index);
        let mut context_generics = CommaPunctuated::new();
        let mut context_generic_defaults = CommaPunctuated::new();
        let mut context_generic_bounds = CommaPunctuated::new();
        let mut context_struct_fields = CommaPunctuated::new();
        let mut expr_struct_fields = CommaPunctuated::new();
        let mut index = 0;
        for field in self.fields.iter() {
            if index == source_field_index {
                // Make `source_field_index` unreachable.
                source_field_index = self.fields.len();
                if let Some(ident) = &field.original.ident {
                    expr_struct_fields.push(quote!(#ident: source));
                } else {
                    expr_struct_fields.push(quote!(source));
                }
            } else {
                let generic = format_ident!("T{index}");
                let field_attr = quote_attr(field.attrs.thisctx.attr.iter());
                let field_vis = field.attrs.thisctx.visibility.as_ref().unwrap_or(self.vis);
                if let Some(ident) = &field.original.ident {
                    context_struct_fields.push(quote!(#field_attr #field_vis #ident: #generic));
                    expr_struct_fields.push(quote!(#ident: self.#ident.into()));
                } else {
                    let member = Member::Unnamed(index.into());
                    context_struct_fields.push(quote!(#field_attr #field_vis #generic));
                    expr_struct_fields.push(quote!(self.#member.into()));
                }
                let field_ty = &field.original.ty;
                context_generic_defaults.push(quote!(#generic = #field_ty));
                context_generic_bounds.push(quote!(#generic: ::core::convert::Into<#field_ty>));
                context_generics.push(generic);
                index += 1;
            }
        }
        let context_struct_body;
        let expr_struct_body;
        let should_context_unit_body =
            context_struct_fields.is_empty() && !matches!(self.unit, Some(false));
        match self.original_fields {
            Fields::Named(_) => {
                context_struct_body = if should_context_unit_body {
                    quote!(;)
                } else {
                    quote!({ #context_struct_fields })
                };
                expr_struct_body = quote!({ #expr_struct_fields });
            }
            Fields::Unnamed(_) => {
                context_struct_body = if should_context_unit_body {
                    quote!(;)
                } else {
                    quote!(( #context_struct_fields );)
                };
                expr_struct_body = quote!(( #expr_struct_fields ));
            }
            Fields::Unit => {
                context_struct_body = quote!(;);
                expr_struct_body = quote!();
            }
        }

        let context_attr = self.attr;
        let context_vis = self.vis;
        let context_ty = match self.suffix {
            Some(Suffix::Flag(flag)) if !flag => self.ident.clone(),
            Some(Suffix::Ident(suff)) => {
                format_ident!("{}{}", self.ident, suff, span = self.ident.span())
            }
            _ => format_ident!("{}{}", self.ident, DEFAULT_SUFFIX, span = self.ident.span()),
        };

        let error_ty = self.error;
        let source_ty;
        let impl_from_context_for_error;
        if let Some(field) = source_field {
            source_ty = field.original.ty.to_token_stream();
            impl_from_context_for_error = None;
        } else {
            source_ty = quote!(());
            impl_from_context_for_error = Some(quote!(
                impl<#context_generics> From<#context_ty<#context_generics>> for #error_ty
                where #context_generic_bounds
                {
                    #[inline]
                    fn from(context: #context_ty<#context_generics>) -> Self {
                        ::thisctx::IntoError::into_error(context, ())
                    }
                }
            ));
        }

        quote!(
            #context_attr
            #context_vis struct #context_ty<#context_generic_defaults>
            #context_struct_body

            impl<#context_generics> ::thisctx::IntoError for #context_ty<#context_generics>
            where #context_generic_bounds
            {
                type Error = #error_ty;
                type Source = #source_ty;

                #[inline]
                fn into_error(self, source: #source_ty) -> #error_ty {
                    #[allow(clippy::useless_conversion)]
                    <#error_ty as ::core::convert::From<_>>::from(
                        #expr_struct_path #expr_struct_body
                    )
                }
            }

            #impl_from_context_for_error
        )
    }
}
