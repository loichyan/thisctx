use crate::{
    ast::{Enum, Field, Input, Struct, Variant},
    attr::{Attrs, Suffix},
    generics::{GenericName, TypeParamBound},
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{token, DeriveInput, Fields, Ident, Index, Result, Token, Type, Visibility};

type GenericsAnalyzer<'a> = crate::generics::GenericsAnalyzer<'a, GenericBoundsContext>;
type GenericBounds<'a> = crate::generics::GenericBounds<'a, GenericBoundsContext>;

const DEFAULT_SUFFIX: &str = "Context";

macro_rules! quote_extend {
    ($tokens:expr=> $($tt:tt)*) => {{
        let mut _s = &mut *$tokens;
        ::quote::quote_each_token!(_s $($tt)*);
    }};
}

pub fn derive(node: &DeriveInput) -> Result<TokenStream> {
    let input = Input::from_syn(node)?;
    Ok(match input {
        Input::Struct(input) => impl_struct(input).to_token_stream(),
        Input::Enum(input) => impl_enum(input),
    })
}

pub fn impl_struct(input: Struct) -> Option<TokenStream> {
    if matches!(input.attrs.context(), Some(false)) || input.attrs.is_transparent() {
        return None;
    }
    Some(
        Context {
            input: input.original,
            variant: None,
            surround: Surround::from_fields(&input.data.fields),
            options: ContextOptions::from_attrs([&input.attrs].into_iter()),
            ident: &input.original.ident,
            fields: &input.fields,
        }
        .impl_all(),
    )
}

pub fn impl_enum(input: Enum) -> TokenStream {
    input
        .variants
        .iter()
        .flat_map(|variant| input.impl_variant(variant))
        .collect()
}

impl<'a> Enum<'a> {
    fn impl_variant(&self, input: &Variant) -> Option<TokenStream> {
        #[allow(clippy::or_fun_call)]
        if matches!(input.attrs.context().or(self.attrs.context()), Some(false))
            || input.attrs.is_transparent()
        {
            return None;
        }
        Some(
            Context {
                input: self.original,
                variant: Some(&input.original.ident),
                surround: Surround::from_fields(&input.original.fields),
                options: ContextOptions::from_attrs([&self.attrs, &input.attrs].into_iter()),
                ident: &input.original.ident,
                fields: &input.fields,
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

    fn context(&self) -> Option<bool> {
        self.thisctx.context
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

    fn impl_all(self) -> TokenStream {
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
                    // TODO: better generated name
                    FieldType::Generated(format_ident!("__T{index}"), original_ty)
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
        let context_definition_generics =
            QuoteContextDefinitionGenerics(&generics_analyzer, &fields_analyzer);
        let context_generics = QuoteContextGenerics(&generics_analyzer, &fields_analyzer);
        let context_fields = QuoteContextFields(&fields_analyzer);
        let constructor_generics = QuoteConstructorGenerics(&generics_analyzer);
        let constructor_fields = QuoteConstructorFeilds(&fields_analyzer);
        let impl_generics = QuoteImplGenerics(&generics_analyzer, &fields_analyzer);
        let impl_bounds = QuoteImplBounds(&generics_analyzer, &fields_analyzer);

        // Surround fields.
        let context_definition_body = (
            // Generate a unit body;
            if index == 0 && !matches!(self.options.unit, Some(false)) {
                Surround::None
            } else {
                self.surround
            }
        )
        .quote(context_fields, true);
        let constructor_body = self.surround.quote(constructor_fields, false);

        // Generate type of context.
        let context_ty = {
            let ident = self.ident;
            let span = self.ident.span();
            match self.options.suffix {
                Some(Suffix::Flag(true)) | None => {
                    format_ident!("{ident}{DEFAULT_SUFFIX}", span = span)
                }
                Some(Suffix::Ident(suffix)) => format_ident!("{ident}{suffix}", span = span),
                Some(Suffix::Flag(false)) => self.ident.clone(),
            }
        };

        // Generate consturctor path.
        let constructor_ident = &self.input.ident;
        let consturctor_path = quote!(#constructor_ident::<#constructor_generics>);

        // Generate context definition.
        let mut tokens = TokenStream::default();
        let context_attr = &self.options.attr;
        quote_extend!(&mut tokens=>
            #context_attr #context_vis
            struct #context_ty<#context_definition_generics>
            #context_definition_body
        );

        let source_ty = QuoteSourceType(source_ty);
        let constructor_type_variant = self.variant.map(QuoteWithColon2);
        for error_ty in std::iter::once(QuoteMultiTypes::Variant2(consturctor_path))
            .chain(self.options.into.iter().map(QuoteMultiTypes::Variant1))
        {
            // Generate `impl From for Error`.
            if source_ty.0.is_none() {
                quote_extend!(&mut tokens=>
                    impl<#impl_generics>
                    ::core::convert::From<#context_ty<#context_generics>>
                    for #error_ty
                    where #impl_bounds {
                        #[inline]
                        fn from(context: #context_ty<#context_generics>) -> Self {
                            ::thisctx::IntoError::into_error(context, ())
                        }
                    }
                );
            }

            // Generate definiton of context and impl `IntoError` for it.
            quote_extend!(&mut tokens=>
                impl<#impl_generics> ::thisctx::IntoError<#error_ty>
                for #context_ty<#context_generics>
                where #impl_bounds {
                    type Source = #source_ty;

                    #[inline]
                    fn into_error(self, source: #source_ty) -> #error_ty {
                        ::core::convert::From::from(
                            #constructor_ident #constructor_type_variant
                            #constructor_body
                        )
                    }
                }
            );
        }

        tokens
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
            Self::Named(t) => t.to_tokens(tokens),
            Self::Unnamed(t) => t.to_tokens(tokens),
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
            Fields::Named(_) => Self::Brace,
            Fields::Unnamed(_) => Self::Paren,
            Fields::Unit => Self::None,
        }
    }

    fn quote<T>(&self, content: T, semi: bool) -> QuoteSurround<T> {
        QuoteSurround {
            surround: *self,
            content,
            semi: if semi { Some(<_>::default()) } else { None },
        }
    }
}

struct QuoteImplGenerics<'a>(&'a GenericsAnalyzer<'a>, &'a FieldsAnalyzer<'a>);
impl ToTokens for QuoteImplGenerics<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, bounds) in self.0.bounds.iter() {
            let definition = QuoteGenericDefinition(name, bounds);
            quote_extend!(tokens=> #definition,);
        }
        QuoteGeneratedGenerics(self.1).to_tokens(tokens);
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
                quote_extend!(tokens=> #ty: ::core::convert::Into<#original>,);
            }
        }
    }
}

struct QuoteConstructorGenerics<'a>(&'a GenericsAnalyzer<'a>);
impl ToTokens for QuoteConstructorGenerics<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, _) in self.0.bounds.iter() {
            quote_extend!(tokens=> #name,);
        }
    }
}

struct QuoteContextDefinitionGenerics<'a>(&'a GenericsAnalyzer<'a>, &'a FieldsAnalyzer<'a>);
impl ToTokens for QuoteContextDefinitionGenerics<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        QuoteSelectedGenericDefinitions(self.0).to_tokens(tokens);
        for (_, info) in self.1 {
            if let FieldType::Generated(ty, original) = &info.ty {
                quote_extend!(tokens=> #ty = #original,);
            }
        }
    }
}

struct QuoteContextGenerics<'a>(&'a GenericsAnalyzer<'a>, &'a FieldsAnalyzer<'a>);
impl ToTokens for QuoteContextGenerics<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, bounds) in self.0.bounds.iter() {
            if bounds.context.selected {
                quote_extend!(tokens=> #name,);
            }
        }
        QuoteGeneratedGenerics(self.1).to_tokens(tokens);
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
                    quote_extend!(tokens=> ::core::convert::Into::into(self.#name));
                }
                FieldType::Source => quote_extend!(tokens=> source),
            }
            quote_extend!(tokens=> ,);
        }
    }
}

struct QuoteGeneratedGenerics<'a>(&'a FieldsAnalyzer<'a>);
impl ToTokens for QuoteGeneratedGenerics<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (_, info) in self.0 {
            if let FieldType::Generated(ty, _) = &info.ty {
                quote_extend!(tokens=> #ty,);
            }
        }
    }
}

struct QuoteSelectedGenericDefinitions<'a>(&'a GenericsAnalyzer<'a>);
impl ToTokens for QuoteSelectedGenericDefinitions<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for (name, bounds) in self.0.bounds.iter() {
            if bounds.context.selected {
                let definition = QuoteGenericDefinition(name, bounds);
                quote_extend!(tokens=> #definition,);
            }
        }
    }
}

struct QuoteGenericDefinition<'a>(GenericName<'a>, &'a GenericBounds<'a>);
impl ToTokens for QuoteGenericDefinition<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self(name, bounds) = self;
        if let Some(kst_ty) = bounds.const_ty {
            quote_extend!(tokens=> const #name: #kst_ty);
        } else {
            quote_extend!(tokens=> #name);
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

struct QuoteWithColon2<T>(T);
impl<T: ToTokens> ToTokens for QuoteWithColon2<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let content = &self.0;
        quote_extend!(tokens=> ::#content);
    }
}

enum QuoteMultiTypes<T1 = TokenStream, T2 = TokenStream> {
    Variant1(T1),
    Variant2(T2),
}
impl<T1: ToTokens, T2: ToTokens> ToTokens for QuoteMultiTypes<T1, T2> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            QuoteMultiTypes::Variant1(t) => t.to_tokens(tokens),
            QuoteMultiTypes::Variant2(t) => t.to_tokens(tokens),
        }
    }
}

struct QuoteSurround<T> {
    surround: Surround,
    content: T,
    semi: Option<Token![;]>,
}
impl<T: ToTokens> ToTokens for QuoteSurround<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let content = &self.content;
        let semi = &self.semi;
        match self.surround {
            Surround::Brace => quote_extend!(tokens=> {#content}),
            Surround::Paren => quote_extend!(tokens=> (#content) #semi),
            Surround::None => semi.to_tokens(tokens),
        }
    }
}

struct QuoteSourceType<'a>(Option<&'a Type>);
impl ToTokens for QuoteSourceType<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(ty) = self.0 {
            ty.to_tokens(tokens);
        } else {
            token::Paren::default().surround(tokens, |_| ());
        }
    }
}

impl ToTokens for GenericName<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(t) => t.to_tokens(tokens),
            Self::Lifetime(t) => t.to_tokens(tokens),
        }
    }
}

impl ToTokens for TypeParamBound<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Trait(t) => t.to_tokens(tokens),
            Self::Lifetime(t) => t.to_tokens(tokens),
        }
    }
}
