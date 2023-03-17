use crate::{
    ast::{Enum, Field, Input, Struct, Variant},
    attr::{Attrs, Suffix},
    generics::{GenericName, TypeParamBound},
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{DeriveInput, Fields, Ident, Index, Result, Token, Type, Visibility};

macro_rules! new_type_quote {
    ($($name:ident($($tt:tt)*);)*) => {$(
        #[allow(non_camel_case_types)]
        struct $name;
        impl ToTokens for $name {
            #[inline]
            fn to_tokens(&self, tokens: &mut ::proc_macro2::TokenStream) {
                quote::quote!($($tt)*).to_tokens(tokens)
            }
        }
    )*};
}

new_type_quote!(
    // Types
    NONE_SOURCE  (::thisctx::NoneSource);

    // Identifiers
    I_SOURCE_VAR (source);

    // Traits
    T_DEFAULT    (::core::default::Default);
    T_FROM       (::core::convert::From);
    T_INTO       (::core::convert::Into);
    T_INTO_ERROR (::thisctx::IntoError);
);

type GenericsAnalyzer<'a> = crate::generics::GenericsAnalyzer<'a, GenericBoundsContext>;

const DEFAULT_SUFFIX: &str = "Context";

pub fn derive(node: &DeriveInput) -> Result<TokenStream> {
    let input = Input::from_syn(node)?;
    Ok(match input {
        Input::Struct(input) => impl_struct(input).unwrap_or_default(),
        Input::Enum(input) => impl_enum(input),
    })
}

pub fn impl_struct(input: Struct) -> Option<TokenStream> {
    if input.attrs.skip() == Some(true) {
        return None;
    }
    let mut options = ContextOptions::from_attrs([&input.attrs].iter().map(<_>::clone));
    if options.suffix.is_none() {
        options.suffix = Some(&Suffix::Flag(true));
    }
    Some(
        input.attrs.with_module(
            input.original,
            Context {
                input: input.original,
                variant: None,
                surround: Surround::from_fields(&input.data.fields),
                options,
                ident: &input.original.ident,
                fields: &input.fields,
                transparent: input.attrs.is_transparent(),
            }
            .impl_all(),
        ),
    )
}

pub fn impl_enum(input: Enum) -> TokenStream {
    let mut tokens = TokenStream::default();
    for variant in input.variants.iter() {
        if let Some(t) = input.impl_variant(variant) {
            tokens.extend(t);
        }
    }
    input.attrs.with_module(input.original, tokens)
}

impl<'a> Enum<'a> {
    fn impl_variant(&self, variant: &Variant) -> Option<TokenStream> {
        if variant.attrs.skip().or(self.attrs.skip()) == Some(true) {
            return None;
        }
        Some(
            Context {
                input: self.original,
                variant: Some(&variant.original.ident),
                surround: Surround::from_fields(&variant.original.fields),
                options: ContextOptions::from_attrs(
                    [&self.attrs, &variant.attrs].iter().map(<_>::clone),
                ),
                ident: &variant.original.ident,
                fields: &variant.fields,
                transparent: variant.attrs.is_transparent(),
            }
            .impl_all(),
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

    fn skip(&self) -> Option<bool> {
        self.thisctx.skip
    }

    fn with_module(&self, input: &DeriveInput, content: TokenStream) -> TokenStream {
        if content.is_empty() {
            return content;
        }
        let vis = self.thisctx.visibility.as_ref().unwrap_or(&input.vis);
        if let Some(module) = self.thisctx.module.as_ref() {
            quote!(#vis mod #module {
                use super::*;
                #content
            })
        } else {
            content
        }
    }
}

struct Context<'a> {
    input: &'a DeriveInput,
    variant: Option<&'a Ident>,
    surround: Surround,
    options: ContextOptions<'a>,
    ident: &'a Ident,
    fields: &'a [Field<'a>],
    transparent: bool,
}

#[derive(Default)]
struct ContextOptions<'a> {
    attr: TokenStream,
    generic: Option<bool>,
    into: Vec<&'a Type>,
    suffix: Option<&'a Suffix>,
    unit: Option<bool>,
    visibility: Option<&'a Visibility>,
}

#[derive(Default)]
struct GenericBoundsContext {
    selected: bool,
}

impl<'a> ContextOptions<'a> {
    fn from_attrs(attrs_iter: impl DoubleEndedIterator<Item = &'a Attrs<'a>> + Clone) -> Self {
        let mut new = ContextOptions::default();

        macro_rules! update_options {
            ($attrs:expr=> ) => {};
            ($attrs:expr=> $attr:ident, $($rest:tt)*) => {
                if let Some(attr) = $attrs.thisctx.$attr {
                    new.$attr = Some(attr);
                }
                update_options!($attrs=> $($rest)*);
            };
            ($attrs:expr=> &$attr:ident, $($rest:tt)*) => {
                if let Some(attr) = $attrs.thisctx.$attr.as_ref() {
                    new.$attr = Some(attr);
                }
                update_options!($attrs=> $($rest)*);
            };
            ($attrs:expr=> +$attr:ident, $($rest:tt)*) => {
                new.$attr.extend($attrs.thisctx.$attr.iter());
                update_options!($attrs=> $($rest)*);
            };
        }

        for attrs in attrs_iter.clone().rev() {
            let attrs = &attrs.thisctx.attr;
            quote!(#(#[#attrs])*).to_tokens(&mut new.attr);
        }
        for attrs in attrs_iter {
            update_options!(attrs=>
                generic,
                unit,
                &suffix,
                &visibility,
                +into,
            );
        }

        new
    }
}

impl<'a> Context<'a> {
    fn find_source_field(&self) -> usize {
        if self.transparent {
            return 0;
        }
        for (i, field) in self.fields.iter().enumerate() {
            if field.attrs.source.is_some() {
                return i;
            }
        }
        for (i, field) in self.fields.iter().enumerate() {
            match &field.original.ident {
                Some(ident) if ident == "source" => {
                    return i;
                }
                _ => (),
            }
        }
        self.fields.len()
    }

    fn impl_all(&self) -> TokenStream {
        // Analyze feilds of contexts.
        let context_vis = self.options.visibility.unwrap_or(&self.input.vis);
        let mut source_field_index = self.find_source_field();
        let mut source_ty = None;
        let mut fields_analyzer = FieldsAnalyzer::default();
        let mut generics_analyzer = GenericsAnalyzer::from_syn(&self.input.generics);
        let mut index = 0;
        for field in self.fields {
            let original_ty = &field.original.ty;
            let field_name = field
                .original
                .ident
                .as_ref()
                .map(FieldName::Named)
                .unwrap_or_else(|| FieldName::Unnamed(index.into()));
            let field_ty;
            // Check if it's a source field.
            if index == source_field_index {
                // Make index of source field unreachable.
                source_field_index = self.fields.len();
                source_ty = Some(original_ty);
                field_ty = FieldType::Source;
            } else {
                let mut generated = true;
                // Check if type of the field intersects with input generics.
                generics_analyzer.intersects(original_ty, |_, bounds| {
                    generated = false;
                    bounds.context.selected = true;
                });
                generated = generated
                    &&
                        field.attrs.thisctx.generic.or(self.options.generic)
                        !=
                        Some(false);
                field_ty = if generated {
                    // Generate a new type for conversion.
                    let generated = if let FieldName::Named(name) = field_name {
                        format_ident!("__T{}", name)
                    } else {
                        format_ident!("__T{}", index)
                    };
                    FieldType::Generated(generated, original_ty)
                } else {
                    FieldType::Original(original_ty)
                };
                // Increase index.
                index += 1;
            }
            fields_analyzer.push((
                field_name,
                FieldInfo {
                    visibility: field
                        .attrs
                        .thisctx
                        .visibility
                        .as_ref()
                        .unwrap_or(context_vis),
                    attrs: &field.attrs,
                    ty: field_ty,
                },
            ))
        }
        let has_source = source_ty.is_some();
        let is_unit = index == 0;

        // Generate type of context.
        let context_ident = {
            let ident = self.ident;
            let span = self.ident.span();
            match self.options.suffix {
                Some(Suffix::Flag(true)) => {
                    format_ident!("{}{}", ident, DEFAULT_SUFFIX, span = span)
                }
                Some(Suffix::Ident(suffix)) => format_ident!("{}{}", ident, suffix, span = span),
                _ => self.ident.clone(),
            }
        };

        // Generate context definition.
        let context_definition = {
            let attr = &self.options.attr;
            let generics1 = quote_analyzed_generics(&generics_analyzer, true, true);
            let generics2 = quote_generated_generics(&fields_analyzer, true);
            let fields = quote_context_fileds(&fields_analyzer);
            let body = if is_unit && self.options.unit!= Some(false) {
                Surround::None
            } else {
                self.surround
            }
            .quote(fields, true);
            quote!(
                #[allow(non_camel_case_types)]
                #attr #context_vis
                struct #context_ident<#generics1 #generics2>
                #body
            )
        };

        // Generate constructor expression.
        let constructor_ty = {
            let ident = &self.input.ident;
            let generics = quote_analyzed_generics(&generics_analyzer, false, false);
            quote!(#ident::<#generics>)
        };
        let constructor_expr = {
            let variant = &self.variant;
            let colon2 = variant.map(|_| <Token![::]>::default());
            let fields = quote_consturctor_fileds(&fields_analyzer);
            let body = self.surround.quote(fields, false);
            quote!(#constructor_ty #colon2 #variant #body)
        };

        // Generate context type, source type.
        let context_ty = {
            let generics1 = quote_analyzed_generics(&generics_analyzer, false, true);
            let generics2 = quote_generated_generics(&fields_analyzer, false);
            quote!(#context_ident::<#generics1 #generics2>)
        };
        let source_ty = source_ty
            .map(ToTokens::to_token_stream)
            .unwrap_or_else(|| NONE_SOURCE.to_token_stream());

        // Generate generics for `impl` block.
        let impl_generics = {
            let generics1 = quote_analyzed_generics(&generics_analyzer, true, false);
            let generics2 = quote_generated_generics(&fields_analyzer, false);
            quote!(#generics1 #generics2)
        };
        let impl_bounds = quote_impl_bounds(&generics_analyzer, &fields_analyzer);

        // Generate trait implementations.
        let context_impls = std::iter::once(constructor_ty)
            .chain(self.options.into.iter().map(ToTokens::to_token_stream))
            .map(|error_ty| {
                let into_error_ty = quote!(#T_INTO_ERROR::<#error_ty>);
                let impl_from_context = if !has_source {
                    // Generate `impl From for Error` if no `source` is specified.
                    Some(quote!(
                        #[allow(non_camel_case_types)]
                        impl<#impl_generics>
                        #T_FROM<#context_ty>
                        for #error_ty
                        where #impl_bounds
                            #context_ty: #into_error_ty,
                            <#context_ty as #into_error_ty>::Source: #T_DEFAULT,
                        {
                            #[inline]
                            fn from(context: #context_ty) -> Self {
                                #T_INTO_ERROR::into_error(context, #T_DEFAULT::default())
                            }
                        }
                    ))
                } else {
                    None
                };
                // Generate `impl IntoError for Context`.
                quote!(
                    #[allow(non_camel_case_types)]
                    impl<#impl_generics> #into_error_ty
                    for #context_ty
                    where #impl_bounds {
                        type Source = #source_ty;

                        #[inline]
                        fn into_error(self, #I_SOURCE_VAR: #source_ty) -> #error_ty {
                            #T_FROM::from(#constructor_expr)
                        }
                    }

                    #impl_from_context
                )
            });
        quote!(
            #context_definition
            #(#context_impls)*
        )
    }
}

type FieldsAnalyzer<'a> = Vec<(FieldName<'a>, FieldInfo<'a>)>;

enum FieldName<'a> {
    Named(&'a Ident),
    Unnamed(Index),
}

impl ToTokens for FieldName<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            FieldName::Named(t) => t.to_tokens(tokens),
            FieldName::Unnamed(t) => t.to_tokens(tokens),
        }
    }
}

struct FieldInfo<'a> {
    visibility: &'a Visibility,
    attrs: &'a Attrs<'a>,
    ty: FieldType<'a>,
}

enum FieldType<'a> {
    Generated(Ident, &'a Type),
    Original(&'a Type),
    Source,
}

#[derive(Clone, Copy)]
enum Surround {
    Paren,
    Brace,
    None,
}

impl Surround {
    fn from_fields(fields: &Fields) -> Self {
        match fields {
            Fields::Named(_) => Surround::Brace,
            Fields::Unnamed(_) => Surround::Paren,
            Fields::Unit => Surround::None,
        }
    }

    fn quote<T: ToTokens>(&self, content: T, trailing_semi: bool) -> TokenStream {
        let semi = if trailing_semi {
            Some(<Token![;]>::default())
        } else {
            None
        };
        match self {
            Surround::Brace => quote!({#content}),
            Surround::Paren => quote!((#content) #semi),
            Surround::None => semi.to_token_stream(),
        }
    }
}

fn quote_impl_bounds(generics: &GenericsAnalyzer, fields: &FieldsAnalyzer) -> TokenStream {
    let bounds = generics.bounds.iter().flat_map(|(name, bounds)| {
        if bounds.params.is_empty() {
            return None;
        }
        let params = bounds.params.iter();
        Some(quote!(#name: #(#params +)*))
    });
    let extra_bounds = generics.extra_bounds.iter().map(ToTokens::to_token_stream);
    let generated_bounds = fields.iter().flat_map(|(_, info)| {
        if let FieldType::Generated(ty, original) = &info.ty {
            Some(quote!(#ty: #T_INTO::<#original>))
        } else {
            None
        }
    });
    let all = bounds.chain(extra_bounds).chain(generated_bounds);
    quote!(#(#all,)*)
}

fn quote_context_fileds(analyzer: &FieldsAnalyzer) -> TokenStream {
    let fields = analyzer.iter().flat_map(|(name, info)| {
        if let FieldType::Source = info.ty {
            return None;
        }
        let attrs = &info.attrs.thisctx.attr;
        let vis = &info.visibility;
        let field = if let FieldName::Named(name) = name {
            Some(quote!(#name:))
        } else {
            None
        };
        let ty = match &info.ty {
            FieldType::Original(ty) => ty.to_token_stream(),
            FieldType::Generated(ty, _) => ty.to_token_stream(),
            FieldType::Source => unreachable!(),
        };
        Some(quote!(#(#[#attrs])* #vis #field #ty))
    });
    quote!(#(#fields,)*)
}

fn quote_consturctor_fileds(analyzer: &FieldsAnalyzer) -> TokenStream {
    let fields = analyzer.iter().map(|(name, info)| {
        let field = if let FieldName::Named(name) = name {
            Some(quote!(#name:))
        } else {
            None
        };
        let expr = match &info.ty {
            FieldType::Original(_) => quote!(self.#name),
            FieldType::Generated(..) => quote!(#T_INTO::into(self.#name)),
            FieldType::Source => I_SOURCE_VAR.to_token_stream(),
        };
        quote!(#field #expr)
    });
    quote!(#(#fields,)*)
}

fn quote_generated_generics(analyzer: &FieldsAnalyzer, emit_default: bool) -> TokenStream {
    let generics = analyzer.iter().flat_map(|(_, info)| {
        if let FieldType::Generated(ty, original_ty) = &info.ty {
            let default = if emit_default {
                Some(quote!(= #original_ty))
            } else {
                None
            };
            Some(quote!(#ty #default))
        } else {
            None
        }
    });
    quote!(#(#generics,)*)
}

fn quote_analyzed_generics(
    analyzer: &GenericsAnalyzer,
    emit_definition: bool,
    emit_selected_only: bool,
) -> TokenStream {
    let generics = analyzer.bounds.iter().flat_map(|(name, bounds)| {
        if emit_selected_only && !bounds.context.selected {
            return None;
        }
        if emit_definition {
            if let Some(kst_ty) = bounds.const_ty {
                return Some(quote!(const #name: #kst_ty));
            }
        }
        Some(quote!(#name))
    });
    quote!(#(#generics,)*)
}

impl ToTokens for GenericName<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            GenericName::Ident(t) => t.to_tokens(tokens),
            GenericName::Lifetime(t) => t.to_tokens(tokens),
        }
    }
}

impl ToTokens for TypeParamBound<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            TypeParamBound::Trait(t) => t.to_tokens(tokens),
            TypeParamBound::Lifetime(t) => t.to_tokens(tokens),
        }
    }
}
