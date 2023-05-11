use crate::{
    ast::{Delimiter, Input},
    context::{ContextInfo, FieldType},
    generics::{ContainerGenerics, GenericBound, GenericName},
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{DeriveInput, Result, Visibility};

macro_rules! quote_to {
    ($tokens:expr=> $($tt:tt)*) => {{
        let mut tokens: &mut ::proc_macro2::TokenStream = $tokens;
        ::quote::quote_each_token!(tokens $($tt)*);
    }};
}

macro_rules! Tk {
    ($($tt:tt)*) => {{ <syn::Token![$($tt)*]>::default() }};
}

pub fn expand(input: &DeriveInput) -> Result<TokenStream> {
    let generics = ContainerGenerics::from_syn(&input.generics);
    let _input = Input::from_syn(&generics, input)?;
    let (module, contexts) = ContextInfo::collect_from(&_input);
    let expanded = contexts
        .into_iter()
        .map(|c| expand_context(&c))
        .collect::<TokenStream>();
    Ok(if let Some(module) = module {
        let vis = &input.vis;
        quote!(#vis mod #module { use super::*; #expanded })
    } else {
        expanded
    })
}

fn expand_context(context: &ContextInfo) -> TokenStream {
    let &ContextInfo {
        ref container_name,
        ref container_variant,
        container_delimiter,
        name: ref context_name,
        source_field,
        ..
    } = context;

    let context_ty = {
        let generics = QuoteGenerics(context).with_generated();
        QuoteWith(move |tokens| quote_to!(tokens=> #context_name::<#generics>))
    };
    let context_definition = {
        let ContextInfo { attr, vis, .. } = context;
        let generics = QuoteGenerics(context)
            .is_definition()
            .with_generated()
            .with_default_ty();
        let fileds = QuoteFields(context);
        QuoteWith(move |tokens| {
            let body = QuoteBody(context.delimiter, &fileds);
            quote_to!(tokens=> #(#[#attr])* #vis struct #context_name <#generics> #body);
        })
    };

    let constructor_ty = {
        let generics = QuoteGenerics(context).with_non_selected();
        QuoteWith(move |tokens| quote_to!(tokens=> #container_name::<#generics>))
    };
    let constructor_expr = {
        let colon2 = container_variant.map(|_| Tk![::]);
        let fields = QuoteFields(context).is_constructor();
        QuoteWith(move |tokens| {
            let body = QuoteBody(container_delimiter, &fields).is_expr();
            quote_to!(tokens=> #constructor_ty #colon2 #container_variant #body);
        })
    };

    let source_ty = QuoteWith(|tokens| {
        if let Some(ty) = source_field.map(|s| &s.ty) {
            quote_to!(tokens=> #ty);
        } else {
            quote_to!(tokens=> ::thisctx::private::NoneSource);
        }
    });

    let impl_generics = QuoteGenerics(context)
        .is_definition()
        .with_non_selected()
        .with_generated();
    let impl_bounds = QuoteBounds(context);

    let context_impls = QuoteWith(move |tokens| {
        std::iter::once(&constructor_ty as &dyn ToTokens)
            .chain(context.into.iter().map(|t| t as &dyn ToTokens))
            .for_each(|error_ty| {
                let into_error = QuoteWith(
                    |tokens| quote_to!(tokens=> ::thisctx::private::IntoError::<#error_ty>),
                );

                quote_to!(tokens=>
                    impl<#impl_generics> #into_error
                    for #context_ty
                    where #impl_bounds {
                        type Source = #source_ty;

                        #[inline]
                        fn into_error(self, source: #source_ty) -> #error_ty {
                            ::thisctx::private::Into::into(#constructor_expr)
                        }
                    }
                );

                if source_field.is_none() {
                    quote_to!(tokens=>
                        impl<#impl_generics> ::thisctx::private::From::<#context_ty>
                        for #error_ty
                        where #impl_bounds {
                            #[inline]
                            fn from(value: #context_ty) -> Self {
                                #into_error::into_error(value, ::thisctx::private::NoneSource)
                            }
                        }
                    )
                }
            });
    });

    quote!(
        #[allow(non_camel_case_types)] #context_definition
        #[allow(non_camel_case_types)] const _: () = { #context_impls };
    )
}

macro_rules! define_quote {
    (struct $name:ident [$($generics:tt)*] $(where [$($bounds:tt)*])? {
        $($arg:ident: $arg_ty:ty,)*
        $(+$flag:ident: bool,)*
    }) => {
        #[derive(Clone, Copy)]
        struct $name <$($generics)*>
        $(where $($bounds)*)? {
            $($arg: $arg_ty,)*
            $($flag: bool,)*
        }

        #[allow(non_snake_case)]
        fn $name<$($generics)*>($($arg: $arg_ty,)*) -> $name <$($generics)*>
        $(where $($bounds)*)? {
            $name { $($arg,)* $($flag: false,)* }
        }

        impl <$($generics)*> $name <$($generics)*>
        $(where $($bounds)*)? {
            $(fn $flag(self) -> Self {
                Self { $flag: true, ..self }
            })*
        }
    };
}

define_quote! {
    struct QuoteGenerics ['a] {
        context: &'a ContextInfo<'a>,
        +is_definition: bool,
        +with_non_selected: bool,
        +with_generated: bool,
        +with_default_ty: bool,
    }
}

impl<'a> ToTokens for QuoteGenerics<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let &Self {
            context,
            is_definition,
            with_non_selected,
            with_generated,
            with_default_ty,
        } = self;

        context
            .generics
            .iter()
            .filter(|(_, s)| with_non_selected || *s)
            .for_each(|(g, _)| match g.name {
                GenericName::Ident(name) => match g.const_ty {
                    Some(kst) if is_definition => quote_to!(tokens=> const #name: #kst,),
                    _ => quote_to!(tokens=> #name,),
                },
                GenericName::Lifetime(name) => quote_to!(tokens=> #name,),
            });

        if with_generated {
            context
                .fields
                .iter()
                .filter_map(|f| {
                    if let FieldType::Generated(name) = &f.ty {
                        Some((&f.original.ty, name))
                    } else {
                        None
                    }
                })
                .for_each(|(original_ty, name)| {
                    if with_default_ty {
                        quote_to!(tokens=> #name = #original_ty,);
                    } else {
                        quote_to!(tokens=> #name,);
                    }
                });
        }
    }
}

define_quote! {
    struct QuoteBounds ['a] {
        context: &'a ContextInfo<'a>,
    }
}

impl<'a> ToTokens for QuoteBounds<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let &Self { context } = self;

        context
            .generics
            .iter()
            .flat_map(|(g, _)| g.bounds.iter().map(|b| (g.name, b)))
            .for_each(|(name, b)| quote_to!(tokens=> #name: #b,));

        context
            .fields
            .iter()
            .filter_map(|f| {
                if let FieldType::Generated(name) = & f.ty {
                    Some((&f.original.ty, name))
                } else {
                    None
                }
            })
            .for_each(|(original_ty, name)| {
                quote_to!(tokens=> #name: ::thisctx::private::Into::<#original_ty>,)
            });

        let extra = &context.container_generics.extra_bounds;
        quote_to!(tokens=> #(#extra,)*);
    }
}

define_quote! {
    struct QuoteFields ['a] {
        context: &'a ContextInfo<'a>,
        +is_constructor: bool,
    }
}

impl<'a> ToTokens for QuoteFields<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let &Self {
            context,
            is_constructor,
        } = self;

        context.fields.iter().for_each(|f| {
            if !is_constructor && matches!(f.ty, FieldType::Source) {
                return;
            }
            let ident = f.original.ident.as_ref();
            let colon = ident.map(|_| Tk![:]);
            let vis = if !is_constructor {
                let vis = match &f.original.vis {
                    // Use context visibility if not specified.
                    Visibility::Inherited => self.context.vis,
                    others => others,
                };
                Some(f.attrs.vis.as_ref().unwrap_or(vis))
            } else {
                None
            };
            quote_to!(tokens=> #vis #ident #colon);
            let name = &f.name;
            match &f.ty {
                FieldType::Generated(ty) => {
                    if is_constructor {
                        quote_to!(tokens=> ::thisctx::private::Into::into(self.#name),);
                    } else {
                        quote_to!(tokens=> #ty,);
                    }
                }
                FieldType::Original => {
                    if is_constructor {
                        quote_to!(tokens=> self.#name,);
                    } else {
                        let ty = &f.original.ty;
                        quote_to!(tokens=> #ty,);
                    }
                }
                FieldType::Source => quote_to!(tokens=> source,),
            }
        });
    }
}

define_quote! {
    struct QuoteBody ['a] {
        delimiter: Delimiter,
        fields: &'a dyn ToTokens,
        +is_expr: bool,
    }
}

impl<'a> ToTokens for QuoteBody<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let &Self {
            delimiter,
            fields,
            is_expr,
        } = self;
        let semi = if !is_expr { Some(Tk![;]) } else { None };
        match delimiter {
            Delimiter::Paren => quote_to!(tokens=> ( #fields ) #semi),
            Delimiter::Brace => quote_to!(tokens=> { #fields }),
            Delimiter::None => quote_to!(tokens=> #fields #semi),
        }
    }
}

define_quote! {
    struct QuoteWith [F] where [F: Fn(&mut TokenStream)] {
        expand: F,
    }
}

impl<F> ToTokens for QuoteWith<F>
where
    F: Fn(&mut TokenStream),
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        (self.expand)(tokens);
    }
}

impl ToTokens for GenericName<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(t) => t.to_tokens(tokens),
            Self::Lifetime(l) => l.to_tokens(tokens),
        }
    }
}

impl ToTokens for GenericBound<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Trait(t) => t.to_tokens(tokens),
            Self::Lifetime(l) => l.to_tokens(tokens),
        }
    }
}
