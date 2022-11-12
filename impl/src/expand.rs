use crate::{
    ast::{Enum, Field, Input, Struct, Variant},
    attr::Suffix,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, DeriveInput, Fields, Ident, Member, Result, Token, Visibility};

const DEFAULT_SUFFIX: &str = "Context";

pub fn derive(node: &DeriveInput) -> Result<TokenStream> {
    let input = Input::from_syn(node)?;
    Ok(match input {
        Input::Struct(input) => impl_struct(input),
        Input::Enum(input) => impl_enum(input),
    })
}

pub fn impl_struct(input: Struct) -> TokenStream {
    let error = &input.original.ident;
    Context {
        error,
        vis: input
            .attrs
            .thisctx
            .vis
            .as_ref()
            .unwrap_or(&input.original.vis),
        ident: error,
        suffix: input.attrs.thisctx.suffix.as_ref(),
        fields: &input.fields,
        original_fields: &input.data.fields,
    }
    .impl_into_error(quote!(#error))
}

pub fn impl_enum(input: Enum) -> TokenStream {
    input
        .variants
        .iter()
        .map(|variant| input.impl_variant(variant))
        .collect()
}

impl<'a> Enum<'a> {
    fn impl_variant(&self, input: &Variant) -> TokenStream {
        let error = &self.original.ident;
        let ident = &input.original.ident;
        Context {
            error,
            vis: input
                .attrs
                .thisctx
                .vis
                .as_ref()
                .or(self.attrs.thisctx.vis.as_ref())
                .unwrap_or(&self.original.vis),
            ident,
            suffix: input
                .attrs
                .thisctx
                .suffix
                .as_ref()
                .or(self.attrs.thisctx.suffix.as_ref()),
            fields: &input.fields,
            original_fields: &input.original.fields,
        }
        .impl_into_error(quote!(#error::#ident))
    }
}

struct Context<'a> {
    error: &'a Ident,
    vis: &'a Visibility,
    ident: &'a Ident,
    suffix: Option<&'a Suffix>,
    fields: &'a [Field<'a>],
    original_fields: &'a Fields,
}

impl<'a> Context<'a> {
    fn impl_into_error(&self, expr_struct: TokenStream) -> TokenStream {
        let mut context_generics = Punctuated::<_, Token![,]>::new();
        let mut context_generic_defaults = Punctuated::<_, Token![,]>::new();
        let mut context_generic_bounds = Punctuated::<_, Token![,]>::new();
        let mut context_struct_fields = Punctuated::<_, Token![,]>::new();
        let mut expr_struct_fields = Punctuated::<_, Token![,]>::new();
        let mut index = 0;
        for field in self.fields.iter() {
            if field.is_source() {
                if let Some(ident) = &field.original.ident {
                    expr_struct_fields.push(quote!(#ident: source));
                } else {
                    expr_struct_fields.push(quote!(source));
                }
            } else {
                let generic = format_ident!("T{index}");
                if let Some(ident) = &field.original.ident {
                    context_struct_fields.push(quote!(#ident: #generic));
                    expr_struct_fields.push(quote!(#ident: self.#ident.into()));
                } else {
                    let member = Member::Unnamed(index.into());
                    context_struct_fields.push(quote!(#generic));
                    expr_struct_fields.push(quote!(self.#member.into()));
                }
                let field_ty = &field.original.ty;
                context_generic_defaults.push(quote!(#generic = #field_ty));
                context_generic_bounds.push(quote!(#generic: core::convert::Into<#field_ty>));
                context_generics.push(generic);
                index += 1;
            }
        }
        let context_struct_body;
        let expr_struct_body;
        match self.original_fields {
            Fields::Named(_) => {
                context_struct_body = quote!({ #context_struct_fields });
                expr_struct_body = quote!({ #expr_struct_fields });
            }
            Fields::Unnamed(_) => {
                context_struct_body = quote!(( #context_struct_fields ););
                expr_struct_body = quote!(( #expr_struct_fields ));
            }
            Fields::Unit => {
                context_struct_body = quote!(;);
                expr_struct_body = quote!();
            }
        }
        let source_field = self.fields.iter().find(|field| field.is_source());
        let source_ty = source_field
            .map(
                |Field {
                     original: syn::Field { ty, .. },
                     ..
                 }| quote!(#ty),
            )
            .unwrap_or_else(|| quote!(()));
        let context_vis = self.vis;
        let context_ty = match self.suffix {
            Some(Suffix::Flag(flag)) if !flag.value => self.ident.clone(),
            Some(Suffix::Ident(suff)) => {
                format_ident!("{}{}", self.ident, suff, span = self.ident.span())
            }
            _ => format_ident!("{}{}", self.ident, DEFAULT_SUFFIX, span = self.ident.span()),
        };
        let error_ty = self.error;
        quote!(
            #context_vis struct #context_ty<#context_generic_defaults> #context_struct_body

            impl<#context_generics> thisctx::IntoError for #context_ty <#context_generics>
            where #context_generic_bounds
            {
                type Error = #error_ty;
                type Source = #source_ty;

                #[inline]
                fn into_error(self, source: #source_ty) -> #error_ty {
                    #expr_struct #expr_struct_body
                }
            }
        )
    }
}
