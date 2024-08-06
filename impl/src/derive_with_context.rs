use std::collections::BTreeMap;
use std::ops;

use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{DeriveInput, Field, Fields, GenericParam, Generics, Visibility};

use crate::attrs::Attrs;
use crate::util::QuoteWith;

struct RT;
impl ToTokens for RT {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // ::thisctx::private
        NewToken![::].to_tokens(tokens);
        NewIdent![thisctx].to_tokens(tokens);
        NewToken![::].to_tokens(tokens);
        NewIdent![private].to_tokens(tokens);
    }
}

// Some notes for reviewers:
//
// 1. Code generations are commented with eye-catching banners.
// 2. Attribute inheritances are a bit scattered, but they are all marked with
//    "inherit #[thisctx(...)]".

pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let attrs = crate::attrs::parse_container(&input)?;
    let vis = attrs.resolve_vis(&input.vis);

    let mut global = GlobalData::default();
    match &input.data {
        syn::Data::Struct(s) => {
            ContextInfo {
                input: &input,
                name: &input.ident,
                fields: &s.fields,
                parent_attrs: None,
                attrs: &attrs,
                vis,
            }
            .expand(&mut global)?;
        }
        syn::Data::Enum(e) => {
            for variant in e.variants.iter() {
                let v_attrs = crate::attrs::parse_variant(variant)?;
                // inherit #[thisctx(skip)]
                if v_attrs.skip.or(attrs.skip).unwrap_or(false) {
                    continue;
                }
                ContextInfo {
                    input: &input,
                    name: &variant.ident,
                    fields: &variant.fields,
                    parent_attrs: Some(&attrs),
                    attrs: &v_attrs,
                    // inherit #[thisctx(vis)]
                    vis: v_attrs.resolve_vis(vis),
                }
                .expand(&mut global)?;
            }
        }
        syn::Data::Union(_) => {
            return Err(syn::Error::new(
                input.ident.span(),
                "union is not supported",
            ))
        }
    }
    let GlobalData {
        optional_fields,
        mut output,
    } = global;

    /* -------------------------- *
     * generate WithOptional impl *
     * -------------------------- */

    // impl WithOptional<#ty> for #input
    //                   ^^^ type is identified by #[thisctx(optional = <id>)]
    for fields in optional_fields.values() {
        let ty = &fields[0].field.ty;
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

        let input_name = &input.ident;
        let impl_body = to_with_optional_body(&input, fields);

        output.extend(quote!(
            impl #impl_generics #RT::WithOptional<<#ty as #RT::Optional>::Inner>
            for #input_name #ty_generics #where_clause {
                #[allow(irrefutable_let_patterns)]
                fn with_optional(
                    &mut self,
                    __value: <#ty as #RT::Optional>::Inner,
                ) -> #RT::Option<<#ty as #RT::Optional>::Inner> {
                    #impl_body
                }
            }
        ));
    }

    /* --------------------- *
     * generate final output *
     * --------------------- */

    Ok(if let Some(module) = &attrs.module {
        quote!(#vis mod #module { use super::*; #output })
    } else {
        output
    })
}

#[derive(Default)]
struct GlobalData<'a> {
    optional_fields: BTreeMap<Ident, Vec<OptionalField<'a>>>,
    output: TokenStream,
}

struct OptionalField<'a> {
    parent: &'a Ident,
    field: &'a Field,
    index: usize,
}

struct ContextInfo<'i, 'a> {
    input: &'i DeriveInput,
    name: &'i Ident,
    fields: &'i Fields,
    parent_attrs: Option<&'a Attrs>,
    attrs: &'a Attrs,
    vis: &'a Visibility,
}

impl<'i> ContextInfo<'i, '_> {
    fn expand(&self, global: &mut GlobalData<'i>) -> syn::Result<()> {
        let Self {
            input,
            name,
            fields,
            parent_attrs,
            attrs,
            vis,
        } = self;
        let fields_info = self.parse_fields_info(global)?;

        /* ------------------------ *
         * generate names and types *
         * ------------------------ */

        let orig_name = *name;
        let name = attrs
            .rename(orig_name)
            // inherit #[thisctx(prefix)]
            // inherit #[thisctx(suffix)]
            .or_else(|| parent_attrs.and_then(|a| a.rename(orig_name)))
            .unwrap_or_else(|| orig_name.clone());
        if parent_attrs.map_or(true, |a| a.module.is_none()) && name == input.ident {
            return Err(syn::Error::new(
                self.span(),
                format!("name conflicts with `{}`", input.ident),
            ));
        }

        // IntoError::Target
        let remote = attrs
            .remote
            .as_ref()
            // inherit #[thisctx(remote)]
            .or_else(|| parent_attrs.and_then(|a| a.remote.as_ref()));
        let target = QuoteWith(|tokens| {
            if let Some(remote) = remote {
                // change IntoError::Target to the specified remote type
                remote.to_tokens(tokens);
            } else {
                input.ident.to_tokens(tokens);
                input.generics.split_for_impl().1.to_tokens(tokens);
            }
        });

        // IntoError::Source
        let source = QuoteWith(|tokens| {
            if let Some(i) = fields_info.source_field {
                // use the specified source type
                fields_info[i].ty.to_tokens(tokens);
            } else {
                tokens.extend(quote!(#RT::NoneSource));
            }
        });

        // magic generics
        let magic_params = fields_info.to_generic_params(false);
        let magic_bounds = fields_info.to_generic_bounds();

        /* ------------------- *
         * generate definition *
         * ------------------- */
        {
            let outer_attrs = attrs
                .to_outer_attrs()
                // inherit #[thisctx(attr)]
                .or_else(|| parent_attrs.and_then(Attrs::to_outer_attrs));

            let magic_defs = fields_info.to_generic_params(
                // Disable generic defaults if there are const parameters:
                //
                // > rustc: generic parameters with a default must be trailing
                // > using type defaults and const parameters in the same parameter
                // > list is currently not permitted
                input.generics.const_params().count() == 0,
            );
            let (def_params, def_kst_params) = split_generic_params(&input.generics, true, true);

            let def_fields = fields_info.to_def().into_token_stream();
            let def_body = Group::new(
                if def_fields.is_empty() {
                    // Emit a unit struct if possible,
                    Delimiter::None
                } else {
                    // otherwise, use its original delimiter
                    match fields {
                        Fields::Named(_) => Delimiter::Brace,
                        Fields::Unnamed(_) => Delimiter::Parenthesis,
                        Fields::Unit => Delimiter::None,
                    }
                },
                def_fields,
            );

            let where_clause = input.generics.where_clause.as_ref();
            let def_body_with_where_clause = QuoteWith(|tokens| {
                if def_body.delimiter() == Delimiter::Brace {
                    // struct Foo <...> where ... {...}
                    //                  ^^^^^^^^^ before body
                    where_clause.to_tokens(tokens);
                    def_body.to_tokens(tokens);
                } else {
                    // struct Foo <...> (...) where ... ; <- semi is required
                    //                        ^^^^^^^^^ after body
                    def_body.to_tokens(tokens);
                    where_clause.to_tokens(tokens);
                    NewToken![;].to_tokens(tokens);
                }
            });

            global.output.extend(quote!(
                #[allow(non_camel_case_types)] #outer_attrs
                #vis struct #name<#def_params #magic_defs #def_kst_params>
                #def_body_with_where_clause
            ));
        }

        /* ----------------------- *
         * generate IntoError impl *
         * ----------------------- */
        {
            let (impl_params, impl_kst_params) = split_generic_params(&input.generics, true, false);
            let (ty_params, ty_kst_params) = split_generic_params(&input.generics, false, false);
            let geneirc_bounds = to_generic_bounds(&input.generics);

            let variant_prefix = to_variant_prefix(input);
            let into_error_body = to_constructor(self.input, &fields_info, "__source");

            global.output.extend(quote!(
                #[allow(non_camel_case_types)]
                impl<#impl_params #magic_params #impl_kst_params> #RT::IntoError
                for #name<#ty_params #magic_params #ty_kst_params>
                where #geneirc_bounds #magic_bounds {
                    type Target = #target;
                    type Source = #source;
                    fn into_error(self, __source: #source) -> #target {
                        #RT::Into::<#target>::into(
                            #variant_prefix #orig_name #into_error_body
                        )
                    }
                }
            ));
        }

        /* -------------------------- *
         * generate From<Source> impl *
         * -------------------------- */

        if let Some(i) = fields_info.from_field {
            let from_ty = &fields_info[i].ty;
            let (impl_generics, _, where_clause) = input.generics.split_for_impl();

            let variant_prefix = to_variant_prefix(input);
            let from_body = to_constructor(self.input, &fields_info, "__value");

            global.output.extend(quote!(
                #[allow(non_camel_case_types)]
                impl #impl_generics #RT::From<#from_ty> for #target
                #where_clause {
                    fn from(__value: #from_ty) -> #target {
                        #RT::Into::<#target>::into(
                            #variant_prefix #orig_name #from_body
                        )
                    }
                }
            ));
        }
        Ok(())
    }

    fn parse_fields_info(&self, global: &mut GlobalData<'i>) -> syn::Result<FieldsInfo> {
        let Self {
            attrs,
            parent_attrs,
            ..
        } = self;
        let mut field_infos = Vec::with_capacity(self.fields.len());

        // 1st-pass: find source filed
        let mut source_field = None;
        let mut from_field = None;
        let mut field_named_source = None;
        let mut len = 0;
        let mut optionals_count = 0;
        for (i, field) in self.fields.iter().enumerate() {
            len += 1;
            let f_attrs = crate::attrs::parse_field(field)?;

            // check source field
            if f_attrs.source {
                if source_field.is_some() {
                    return Err(syn::Error::new(self.span(), "duplicate source fields"));
                }
                source_field = Some(i);
            }
            if field.ident.as_ref().map_or(false, |i| i == "source") {
                field_named_source = Some(i);
            }

            // check from field
            if f_attrs.from {
                if from_field.is_some() {
                    return Err(syn::Error::new(self.span(), "duplicate from fields"));
                }
                from_field = Some(i);
            }

            // collect optional fields
            if let Some(optional) = &f_attrs.optional {
                optionals_count += 1;
                let id = optional
                    .as_ref()
                    .or_else(|| field.ident.as_ref())
                    // this should have been checked during parsing
                    .unwrap_or_else(|| unreachable!())
                    .clone();
                global
                    .optional_fields
                    .entry(id)
                    .or_default()
                    .push(OptionalField {
                        parent: self.name,
                        field,
                        index: i,
                    });
            }

            field_infos.push(FieldInfo {
                i: field,
                attrs: f_attrs,
                parent_vis: self.vis,
                generic: None,
            });
        }
        if attrs.transparent {
            if len != 1 {
                return Err(syn::Error::new(
                    self.span(),
                    "a transparent context must have exactly 1 field",
                ));
            }
            // A transparent error should have only 1 field, and that field
            // becomes the source.
            source_field = Some(0);
            field_infos[0].attrs.source = true;
        } else if let Some(i) = from_field {
            if (len - optionals_count) != 1 {
                return Err(syn::Error::new(
                    self.span(),
                    "`from` requires exactly 1 field (excluding optional fields)",
                ));
            }
            // From attributes always implies that the same field is source.
            source_field = from_field;
            field_infos[i].attrs.source = true;
        } else if let (None, Some(i)) = (source_field, field_named_source) {
            // Attributes take precedence over the field named "source".
            source_field = field_named_source;
            field_infos[i].attrs.source = true;
        }

        // 2nd-pass: add generics
        let parent_magic = attrs
            .magic
            // inherit #[thisctx(magic)]
            .or_else(|| parent_attrs.and_then(|a| a.magic));
        for (i, f) in field_infos.iter_mut().enumerate() {
            if f.attrs.is_excluded() {
                continue;
            }

            if f.attrs
                .magic
                .or(parent_magic)
                .unwrap_or_else(|| crate::infer::is_in_magic_whitelist(&f.ty))
            {
                f.generic = Some(match &f.ident {
                    Some(i) => format_ident!("T_{}", i, span = i.span()),
                    None => format_ident!("T_{}", i),
                });
            } else {
                f.generic = None;
            }
        }

        Ok(FieldsInfo {
            i: field_infos,
            source_field,
            from_field,
        })
    }

    fn span(&self) -> Span {
        self.name.span()
    }
}

fn to_constructor<'a>(
    _input: &'a DeriveInput,
    fields: &'a FieldsInfo,
    source: &'static str,
) -> impl 'a + ToTokens {
    // Tuple structs can be constructed using indices:
    //
    // let my_tuple = MyTuple {
    //     0: "field_0",
    //     1: "field_1",
    //     ..
    // };
    let fields = QuoteWith(move |tokens| {
        let mut shift = 0usize;
        for (i, f) in fields.iter().enumerate() {
            to_member(f, i).to_tokens(tokens);
            NewToken![:].to_tokens(tokens);
            let ty = &f.ty;
            tokens.extend(if f.attrs.source {
                shift += 1;
                Ident::new(source, Span::call_site()).into_token_stream()
            } else if f.attrs.optional.is_some() {
                shift += 1;
                quote!(<#ty as #RT::Default>::default())
            } else {
                // shift excluded fields to get the correct member index
                let member = to_member(f, i - shift);
                // Into::into works on both generic and non-generic fields
                quote!(#RT::Into::<#ty>::into(self.#member))
            });
            NewToken![,].to_tokens(tokens);
        }
    });
    Group::new(Delimiter::Brace, fields.into_token_stream())
}

fn to_with_optional_body<'a>(
    input: &'a DeriveInput,
    fields: &'a [OptionalField],
) -> impl 'a + ToTokens {
    // Tuple structs can be deconstructed using indices:
    //
    // let MyTuple {
    //     0: field_0,
    //     1: field_1,
    //     ..
    // } = my_tuple;
    let variant_prefix = to_variant_prefix(input);
    QuoteWith(move |tokens| {
        for OptionalField {
            parent,
            field,
            index,
        } in fields
        {
            let member = to_member(field, *index);
            tokens.extend(quote!(
                if let #variant_prefix #parent {
                    #member: __self,
                    ..
                } = self {
                    return #RT::Optional::set(__self, __value);
                }
            ));
        }
        tokens.extend(quote!(return #RT::Option::Some(__value)));
    })
}

struct FieldsInfo<'a> {
    i: Vec<FieldInfo<'a>>,
    source_field: Option<usize>,
    from_field: Option<usize>,
}

impl<'a> ops::Deref for FieldsInfo<'a> {
    type Target = [FieldInfo<'a>];

    fn deref(&self) -> &Self::Target {
        &self.i
    }
}

impl FieldsInfo<'_> {
    fn to_def(&self) -> impl '_ + ToTokens {
        QuoteWith(move |tokens| {
            for f in self.iter() {
                if f.attrs.is_excluded() {
                    continue;
                }
                f.attrs.to_outer_attrs().to_tokens(tokens);
                f.parent_vis.to_tokens(tokens);
                f.ident.to_tokens(tokens);
                f.colon_token.to_tokens(tokens);
                if let Some(g) = &f.generic {
                    // replace the original type with the generic identifier
                    g.to_tokens(tokens);
                } else {
                    f.ty.to_tokens(tokens);
                }
                NewToken![,].to_tokens(tokens);
            }
        })
    }

    fn to_generic_params(&self, with_defaults: bool) -> impl '_ + ToTokens {
        QuoteWith(move |tokens| {
            for f in self.iter() {
                let ty = &f.ty;
                if let Some(g) = &f.generic {
                    g.to_tokens(tokens);
                    if with_defaults {
                        // With defaults, the generated context can be used
                        // without generics:
                        //
                        // MyError<T_field1 = String> {
                        //     field1: T_field1,
                        // }
                        //
                        // type Alias = MyError;
                        NewToken![=].to_tokens(tokens);
                        ty.to_tokens(tokens);
                    }
                    NewToken![,].to_tokens(tokens);
                }
            }
        })
    }

    fn to_generic_bounds(&self) -> impl '_ + ToTokens {
        QuoteWith(move |tokens| {
            for f in self.iter() {
                if let Some(g) = &f.generic {
                    let ty = &f.ty;
                    tokens.extend(quote!(#g: #RT::Into::<#ty>,));
                }
            }
        })
    }
}

struct FieldInfo<'a> {
    i: &'a Field,
    attrs: Attrs,
    parent_vis: &'a Visibility,
    generic: Option<Ident>,
}

impl ops::Deref for FieldInfo<'_> {
    type Target = Field;

    fn deref(&self) -> &Self::Target {
        self.i
    }
}

impl Attrs {
    fn resolve_vis<'a>(&'a self, orig: &'a Visibility) -> &Visibility {
        // #[thisctx(vis)] is preferred, otherwise fallback to the original one
        self.vis.as_ref().unwrap_or(orig)
    }

    fn rename(&self, name: &Ident) -> Option<Ident> {
        Some(match (&self.rename, &self.prefix, &self.suffix) {
            (Some(r), ..) => r.clone(),
            (_, Some(p), Some(s)) => format_ident!("{}{}{}", p, name, s, span = name.span()),
            (_, Some(p), None) => format_ident!("{}{}", p, name, span = name.span()),
            (_, None, Some(s)) => format_ident!("{}{}", name, s, span = name.span()),
            _ => return None,
        })
    }

    fn is_excluded(&self) -> bool {
        // Source field and optional fields are excluded from the generated
        // context fields.
        self.source || self.optional.is_some()
    }

    fn to_outer_attrs(&self) -> Option<impl '_ + ToTokens> {
        if self.attr.is_empty() {
            None
        } else {
            Some(QuoteWith(move |tokens| {
                for a in self.attr.iter() {
                    NewToken![#].to_tokens(tokens);
                    tokens.extend(Group::new(Delimiter::Bracket, a.clone()).into_token_stream());
                }
            }))
        }
    }
}

fn split_generic_params(
    generics: &Generics,
    with_bounds: bool,
    with_defaults: bool,
) -> (impl '_ + ToTokens, impl '_ + ToTokens) {
    // Split const parameters and non-const parameters:
    //
    // > rustc: type parameters must be declared prior to const parameters
    let non_kst_params = QuoteWith(move |tokens| {
        for param in generics.params.iter() {
            match param {
                GenericParam::Lifetime(l) => {
                    l.lifetime.to_tokens(tokens);
                    if with_bounds {
                        l.colon_token.to_tokens(tokens);
                        l.bounds.to_tokens(tokens);
                    }
                }
                GenericParam::Type(t) => {
                    t.ident.to_tokens(tokens);
                    if with_bounds {
                        t.colon_token.to_tokens(tokens);
                        t.bounds.to_tokens(tokens);
                    }
                    if with_defaults {
                        t.eq_token.to_tokens(tokens);
                        t.default.to_tokens(tokens);
                    }
                }
                _ => continue,
            }
            NewToken![,].to_tokens(tokens);
        }
    });
    let kst_params = QuoteWith(move |tokens| {
        for param in generics.params.iter() {
            match param {
                GenericParam::Const(k) => {
                    if with_bounds || with_defaults {
                        k.const_token.to_tokens(tokens);
                        k.ident.to_tokens(tokens);
                        k.colon_token.to_tokens(tokens);
                        k.ty.to_tokens(tokens);
                    } else {
                        k.ident.to_tokens(tokens);
                    }
                    if with_defaults {
                        k.eq_token.to_tokens(tokens);
                        k.default.to_tokens(tokens);
                    }
                }
                _ => continue,
            }
            NewToken![,].to_tokens(tokens);
        }
    });
    (non_kst_params, kst_params)
}

fn to_generic_bounds(generics: &Generics) -> impl '_ + ToTokens {
    QuoteWith(move |tokens| {
        if let Some(where_clause) = &generics.where_clause {
            for pred in where_clause.predicates.iter() {
                pred.to_tokens(tokens);
                NewToken![,].to_tokens(tokens);
            }
        }
    })
}

fn to_variant_prefix(input: &DeriveInput) -> impl '_ + ToTokens {
    QuoteWith(move |tokens| {
        if matches!(input.data, syn::Data::Enum(_)) {
            // variant constructor requires enum prefix
            input.ident.to_tokens(tokens);
            NewToken![::].to_tokens(tokens);
        }
    })
}

fn to_member(f: &Field, index: usize) -> impl '_ + ToTokens {
    QuoteWith(move |tokens| {
        if let Some(i) = &f.ident {
            i.to_tokens(tokens);
        } else {
            syn::Index {
                index: index as u32,
                span: Span::call_site(),
            }
            .to_tokens(tokens);
        }
    })
}
