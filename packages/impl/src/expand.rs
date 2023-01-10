use crate::{
    ast::{Enum, Field, Input, Struct, Variant},
    attr::{Attrs, Suffix},
    generics::{GenericName, TypeParamBound},
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{DeriveInput, Fields, Ident, Index, Result, Token, Type, Visibility};

macro_rules! quote_extend {
    ($tokens:expr=> $($tt:tt)*) => {{
        let mut _s = &mut *$tokens;
        ::quote::quote_each_token!(_s $($tt)*);
    }};
}

macro_rules! new_type_quote {
    ($name:ident=> $($tt:tt)*) => {
        #[allow(non_camel_case_types)]
        struct $name;
        impl ToTokens for $name {
            #[inline]
            fn to_tokens(&self, tokens: &mut ::proc_macro2::TokenStream) {
                quote_extend!(tokens=> $($tt)*);
            }
        }
    };
    ($name:ident($($var:ident: $ty:ident),*)=> $($tt:tt)*) => {
        struct $name<$($ty: ToTokens),*>($($ty),*);
        impl <$($ty: ToTokens),*> ToTokens for $name<$($ty),*> {
            #[inline]
            fn to_tokens(&self, tokens: &mut ::proc_macro2::TokenStream) {
                let Self($($var,)*) = self;
                quote_extend!(tokens=> $($tt)*);
            }
        }
    };
}

macro_rules! define_arg {
    ($($name:ident,)*) => {
        $(trait $name {
            #[allow(non_upper_case_globals)]
            const $name: bool;
        })*
    };
}

macro_rules! impl_arg {
    ($name:ident($($arg:ident = $val:expr,)*)) => {
        struct $name;
        $(impl $arg for $name {
            #[allow(non_upper_case_globals)]
            const $arg: bool = $val;
        })*

    };
}

define_arg! {
    EmitDefinition,
    EmitSelectedOnly,
    EmitDefault,
    EmitTrailingSemi,
}

impl_arg!(WithoutTrailingSemi(EmitTrailingSemi = false,));
impl_arg!(WithTrailingSemi(EmitTrailingSemi = true,));

new_type_quote!(ty_none_source=> ::thisctx::NoneSource);
new_type_quote!(i_source_var=> source);
new_type_quote!(t_from=> ::core::convert::From);
new_type_quote!(t_into=> ::core::convert::Into);
new_type_quote!(t_into_error=> ::thisctx::IntoError);
new_type_quote!(t_default=> ::core::default::Default);
new_type_quote!(QuoteLeadingColon2(a:T1)=> ::#a);
new_type_quote!(QuoteGeneric(a:T1)=> <#a>);

type GenericsAnalyzer<'a> = crate::generics::GenericsAnalyzer<'a, GenericBoundsContext>;

const DEFAULT_SUFFIX: &str = "Context";

pub fn derive(node: &DeriveInput) -> Result<TokenStream> {
    let input = Input::from_syn(node)?;
    Ok(match input {
        Input::Struct(input) => impl_struct(input),
        Input::Enum(input) => impl_enum(input),
    })
}

pub fn impl_struct(input: Struct) -> TokenStream {
    if matches!(input.attrs.context(), Some(false)) || input.attrs.is_transparent() {
        return TokenStream::default();
    }
    input.attrs.with_module(
        input.original,
        Context {
            input: input.original,
            variant: None,
            surround: Surround::from_fields(&input.data.fields),
            options: ContextOptions::from_attrs([&input.attrs].iter().map(<_>::clone)),
            ident: &input.original.ident,
            fields: &input.fields,
        }
        .to_token_stream(),
    )
}

pub fn impl_enum(input: Enum) -> TokenStream {
    let mut tokens = TokenStream::default();
    for variant in input.variants.iter() {
        input.impl_variant(variant, &mut tokens);
    }
    input.attrs.with_module(input.original, tokens)
}

impl<'a> Enum<'a> {
    #[allow(clippy::or_fun_call)]
    fn impl_variant(&self, variant: &Variant, tokens: &mut TokenStream) {
        if matches!(
            variant.attrs.context().or(self.attrs.context()),
            Some(false)
        ) || variant.attrs.is_transparent()
        {
            return;
        }
        Context {
            input: self.original,
            variant: Some(&variant.original.ident),
            surround: Surround::from_fields(&variant.original.fields),
            options: ContextOptions::from_attrs(
                [&self.attrs, &variant.attrs].iter().map(<_>::clone),
            ),
            ident: &variant.original.ident,
            fields: &variant.fields,
        }
        .to_tokens(tokens);
    }
}

impl<'a> Attrs<'a> {
    fn is_transparent(&self) -> bool {
        self.error
            .as_ref()
            .and_then(|e| e.transparent.as_ref())
            .is_some()
    }

    fn context(&self) -> Option<bool> {
        self.thisctx.context
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
}

impl ToTokens for Context<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.impl_all(tokens);
    }
}

#[derive(Default)]
struct ContextOptions<'a> {
    visibility: Option<&'a Visibility>,
    suffix: Option<&'a Suffix>,
    unit: Option<bool>,
    into: Vec<&'a Type>,
    attr: TokenStream,
    generic: Option<bool>,
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
                if let Some(attr) = $attrs.thisctx.$attr.as_ref() {
                    new.$attr = Some(attr);
                }
                update_options!($attrs=> $($rest)*);
            };
            ($attrs:expr=> *$attr:ident, $($rest:tt)*) => {
                if let Some(&attr) = $attrs.thisctx.$attr.as_ref() {
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
            QuoteAttrs(&attrs.thisctx.attr).to_tokens(&mut new.attr);
        }
        for attrs in attrs_iter {
            update_options!(attrs=>
                visibility,
                suffix,
                *unit,
                +into,
                *generic,
            );
        }

        new
    }
}

impl<'a> Context<'a> {
    fn find_source_field(&self) -> usize {
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

    fn impl_all(&self, tokens: &mut TokenStream) {
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
                    && !matches!(
                        field.attrs.thisctx.generic.or(self.options.generic),
                        Some(false),
                    );
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

        // Generate quote wrappers.
        let context_definition_generics = {
            impl_arg!(Arg(
                EmitDefinition = true,
                EmitSelectedOnly = true,
                EmitDefault = true,
            ));
            Quote2Types(
                QuoteAnalyzedGenerics(Arg, &generics_analyzer),
                QuoteGeneratedGenerics(Arg, &fields_analyzer),
            )
        };
        let context_generics = {
            impl_arg!(Arg(
                EmitDefinition = false,
                EmitSelectedOnly = true,
                EmitDefault = false,
            ));
            Quote2Types(
                QuoteAnalyzedGenerics(Arg, &generics_analyzer),
                QuoteGeneratedGenerics(Arg, &fields_analyzer),
            )
        };
        let constructor_generics = {
            impl_arg!(Arg(EmitDefinition = false, EmitSelectedOnly = false,));
            QuoteAnalyzedGenerics(Arg, &generics_analyzer)
        };
        let impl_generics = {
            impl_arg!(Arg(
                EmitDefinition = true,
                EmitSelectedOnly = false,
                EmitDefault = false,
            ));
            Quote2Types(
                QuoteAnalyzedGenerics(Arg, &generics_analyzer),
                QuoteGeneratedGenerics(Arg, &fields_analyzer),
            )
        };
        let impl_bounds = QuoteImplBounds(&generics_analyzer, &fields_analyzer);
        let context_fields = QuoteContextFields(&fields_analyzer);
        let constructor_fields = QuoteConstructorFeilds(&fields_analyzer);

        // Surround fields.
        let context_definition_body = if index == 0 && !matches!(self.options.unit, Some(false)) {
            Surround::None
        } else {
            self.surround
        }
        .quote(WithTrailingSemi, context_fields);
        let constructor_body = self.surround.quote(WithoutTrailingSemi, constructor_fields);

        // Generate type of context.
        let context_ident = {
            let ident = self.ident;
            let span = self.ident.span();
            match self.options.suffix {
                Some(Suffix::Flag(true)) | None => {
                    format_ident!("{}{}", ident, DEFAULT_SUFFIX, span = span)
                }
                Some(Suffix::Ident(suffix)) => format_ident!("{}{}", ident, suffix, span = span),
                Some(Suffix::Flag(false)) => self.ident.clone(),
            }
        };

        // Generate context definition.
        let context_attr = &self.options.attr;
        quote_extend!(tokens=>
            #[allow(non_camel_case_types)]
            #context_attr #context_vis
            struct #context_ident<#context_definition_generics>
            #context_definition_body
        );

        // Generate context type.
        let context_ty = Quote2Types(&context_ident, QuoteGeneric(&context_generics));

        // Create useful quote types.
        let constructor_ident = &self.input.ident;
        let constructor_ty = Quote2Types(&constructor_ident, QuoteGeneric(&constructor_generics));
        let constructor_ty_variant = self.variant.map(QuoteLeadingColon2);
        let has_source = source_ty.is_some();
        let source_ty = QuoteSourceType(source_ty);

        for error_ty in std::iter::once(Quote2Variants::Variant2(constructor_ty))
            .chain(self.options.into.iter().map(Quote2Variants::Variant1))
        {
            let into_error_ty = Quote2Types(t_into_error, QuoteGeneric(&error_ty));
            // Generate `impl IntoError for Context`.
            quote_extend!(tokens=>
                #[allow(non_camel_case_types)]
                impl<#impl_generics> #into_error_ty
                for #context_ty
                where #impl_bounds {
                    type Source = #source_ty;

                    #[inline]
                    fn into_error(self, #i_source_var: #source_ty) -> #error_ty {
                        #t_from::from(
                            #constructor_ident #constructor_ty_variant #constructor_body
                        )
                    }
                }
            );
            if !has_source {
                // Generate `impl From for Error` if no `source` is specified.
                quote_extend!(tokens=>
                    #[allow(non_camel_case_types)]
                    impl<#impl_generics>
                    #t_from<#context_ty>
                    for #error_ty
                    where #impl_bounds
                        #context_ty: #into_error_ty,
                        <#context_ty as #into_error_ty>::Source: #t_default,
                    {
                        #[inline]
                        fn from(context: #context_ty) -> Self {
                            #t_into_error::into_error(context, #t_default::default())
                        }
                    }
                );
            }
        }
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

    fn quote<A, T>(&self, arg: A, content: T) -> QuoteSurround<A, T> {
        QuoteSurround {
            arg,
            surround: *self,
            content,
        }
    }
}

struct QuoteImplBounds<'a>(&'a GenericsAnalyzer<'a>, &'a FieldsAnalyzer<'a>);
impl ToTokens for QuoteImplBounds<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, bounds) in self.0.bounds.iter() {
            if !bounds.params.is_empty() {
                let params = bounds.params.iter();
                quote_extend!(tokens=> #name: #(#params +)*,);
            }
        }
        for bounds in self.0.extra_bounds.iter() {
            quote_extend!(tokens=> #bounds,);
        }
        for (_, info) in self.1.iter() {
            if let FieldType::Generated(ty, original) = &info.ty {
                quote_extend!(tokens=> #ty: #t_into::<#original>,);
            }
        }
    }
}

struct QuoteSourceType<'a>(Option<&'a Type>);
impl ToTokens for QuoteSourceType<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(ty) = self.0 {
            ty.to_tokens(tokens);
        } else {
            quote_extend!(tokens=> #ty_none_source);
        }
    }
}

struct QuoteContextFields<'a>(&'a FieldsAnalyzer<'a>);
impl ToTokens for QuoteContextFields<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, info) in self.0 {
            if let FieldType::Source = info.ty {
                continue;
            }
            QuoteAttrs(&info.attrs.thisctx.attr).to_tokens(tokens);
            info.visibility.to_tokens(tokens);
            if let FieldName::Named(name) = name {
                quote_extend!(tokens=> #name:);
            }
            match &info.ty {
                FieldType::Original(ty) => ty.to_tokens(tokens),
                FieldType::Generated(ty, _) => ty.to_tokens(tokens),
                FieldType::Source => unreachable!(),
            }
            quote_extend!(tokens=> ,);
        }
    }
}

struct QuoteConstructorFeilds<'a>(&'a FieldsAnalyzer<'a>);
impl ToTokens for QuoteConstructorFeilds<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, info) in self.0 {
            if let FieldName::Named(name) = name {
                quote_extend!(tokens=> #name:);
            }
            match &info.ty {
                FieldType::Original(_) => quote_extend!(tokens=> self.#name),
                FieldType::Generated(..) => {
                    quote_extend!(tokens=> #t_into::into(self.#name));
                }
                FieldType::Source => quote_extend!(tokens=> #i_source_var),
            }
            quote_extend!(tokens=> ,);
        }
    }
}

struct QuoteGeneratedGenerics<'a, A>(A, &'a FieldsAnalyzer<'a>);
impl<A> ToTokens for QuoteGeneratedGenerics<'_, A>
where
    A: EmitDefault,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (_, info) in self.1 {
            if let FieldType::Generated(ty, original_ty) = &info.ty {
                quote_extend!(tokens=> #ty);
                if A::EmitDefault {
                    quote_extend!(tokens=> = #original_ty);
                }
                quote_extend!(tokens=> ,);
            }
        }
    }
}

struct QuoteAnalyzedGenerics<'a, A>(A, &'a GenericsAnalyzer<'a>);
impl<A> ToTokens for QuoteAnalyzedGenerics<'_, A>
where
    A: EmitDefinition + EmitSelectedOnly,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, bounds) in self.1.bounds.iter() {
            if A::EmitSelectedOnly && !bounds.context.selected {
                continue;
            }
            if A::EmitDefinition {
                if let Some(kst_ty) = bounds.const_ty {
                    quote_extend!(tokens=> const #name: #kst_ty,);
                    continue;
                }
            }
            quote_extend!(tokens=> #name,);
        }
    }
}

struct QuoteAttrs<'a>(&'a [TokenStream]);
impl ToTokens for QuoteAttrs<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for args in self.0 {
            quote_extend!(tokens=> #[#args]);
        }
    }
}

struct Quote2Types<T1, T2>(T1, T2);
impl<T1, T2> ToTokens for Quote2Types<T1, T2>
where
    T1: ToTokens,
    T2: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
        self.1.to_tokens(tokens);
    }
}

enum Quote2Variants<T1 = TokenStream, T2 = TokenStream> {
    Variant1(T1),
    Variant2(T2),
}
impl<T1: ToTokens, T2: ToTokens> ToTokens for Quote2Variants<T1, T2> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Quote2Variants::Variant1(t) => t.to_tokens(tokens),
            Quote2Variants::Variant2(t) => t.to_tokens(tokens),
        }
    }
}

struct QuoteSurround<A, T> {
    #[allow(dead_code)]
    arg: A,
    surround: Surround,
    content: T,
}
impl<A, T> ToTokens for QuoteSurround<A, T>
where
    A: EmitTrailingSemi,
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let content = &self.content;
        let semi = if A::EmitTrailingSemi {
            Some(<Token![;]>::default())
        } else {
            None
        };
        match self.surround {
            Surround::Brace => quote_extend!(tokens=> {#content}),
            Surround::Paren => quote_extend!(tokens=> (#content) #semi),
            Surround::None => semi.to_tokens(tokens),
        }
    }
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
