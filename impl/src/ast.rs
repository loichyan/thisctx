use crate::{
    attr::{Attrs, Node},
    generics::{ContainerGenerics, GenericInfo},
};
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, Result};

pub enum Input<'a> {
    Struct(Struct<'a>),
    Enum(Enum<'a>),
}

pub struct Struct<'a> {
    pub original: &'a DeriveInput,
    pub data: &'a DataStruct,
    pub attrs: Attrs,
    pub generics: &'a ContainerGenerics<'a>,
    pub fields: Vec<Field<'a>>,
}

pub struct Enum<'a> {
    pub original: &'a DeriveInput,
    pub data: &'a DataEnum,
    pub attrs: Attrs,
    pub generics: &'a ContainerGenerics<'a>,
    pub variants: Vec<Variant<'a>>,
}

pub struct Variant<'a> {
    pub original: &'a syn::Variant,
    pub attrs: Attrs,
    pub fields: Vec<Field<'a>>,
}

pub struct Field<'a> {
    pub original: &'a syn::Field,
    pub attrs: Attrs,
    /// Container generics used by this field.
    pub generics: Vec<&'a GenericInfo<'a>>,
}

impl<'a> Input<'a> {
    pub fn from_syn(generics: &'a ContainerGenerics<'a>, input: &'a DeriveInput) -> Result<Self> {
        let attrs = crate::attr::get(Node::Container(input), &input.attrs)?;
        Ok(match &input.data {
            Data::Struct(data) => Self::Struct(Struct {
                original: input,
                data,
                attrs,
                generics,
                fields: Field::from_syn(generics, &data.fields)?,
            }),
            Data::Enum(data) => Self::Enum(Enum {
                original: input,
                data,
                attrs,
                generics,
                variants: data
                    .variants
                    .iter()
                    .map(|variant| {
                        Ok(Variant {
                            original: variant,
                            attrs: crate::attr::get(Node::Variant(variant), &variant.attrs)?,
                            fields: Field::from_syn(generics, &variant.fields)?,
                        })
                    })
                    .collect::<Result<_>>()?,
            }),
            Data::Union(_) => {
                return Err(syn::Error::new_spanned(input, "unions are not supported"))
            }
        })
    }
}

impl<'a> Field<'a> {
    fn from_syn(generics: &'a ContainerGenerics<'a>, fields: &'a syn::Fields) -> Result<Vec<Self>> {
        fields
            .iter()
            .map(|field| {
                Ok(Self {
                    original: field,
                    attrs: crate::attr::get(Node::Field(field), &field.attrs)?,
                    generics: generics.intersection(&field.ty),
                })
            })
            .collect()
    }
}

#[derive(Clone, Copy)]
pub enum Delimiter {
    Paren,
    Brace,
    None,
}

impl Delimiter {
    pub fn from_fields(value: &Fields) -> Self {
        match value {
            Fields::Named(_) => Self::Brace,
            Fields::Unnamed(_) => Self::Paren,
            Fields::Unit => Self::None,
        }
    }
}
