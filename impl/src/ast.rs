#![allow(dead_code)]

use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Fields, Result};

use crate::attr::{self, Attrs};

pub enum Input<'a> {
    Struct(Struct<'a>),
    Enum(Enum<'a>),
}

pub struct Struct<'a> {
    pub original: &'a DeriveInput,
    pub data: &'a DataStruct,
    pub attrs: Attrs<'a>,
    pub fields: Vec<Field<'a>>,
}

pub struct Enum<'a> {
    pub original: &'a DeriveInput,
    pub data: &'a DataEnum,
    pub attrs: Attrs<'a>,
    pub variants: Vec<Variant<'a>>,
}

pub struct Variant<'a> {
    pub original: &'a syn::Variant,
    pub attrs: Attrs<'a>,
    pub fields: Vec<Field<'a>>,
}

pub struct Field<'a> {
    pub original: &'a syn::Field,
    pub attrs: Attrs<'a>,
}

impl<'a> Input<'a> {
    pub fn from_syn(node: &'a DeriveInput) -> Result<Self> {
        match &node.data {
            Data::Struct(data) => Struct::from_syn(node, data).map(Input::Struct),
            Data::Enum(data) => Enum::from_syn(node, data).map(Input::Enum),
            Data::Union(_) => Err(Error::new_spanned(node, "unions are not supported")),
        }
    }
}

impl<'a> Struct<'a> {
    fn from_syn(node: &'a DeriveInput, data: &'a DataStruct) -> Result<Self> {
        let attrs = attr::get(&node.attrs)?;
        let fields = Field::from_syn_many(&data.fields)?;
        Ok(Struct {
            original: node,
            data,
            attrs,
            fields,
        })
    }
}

impl<'a> Enum<'a> {
    fn from_syn(node: &'a DeriveInput, data: &'a DataEnum) -> Result<Self> {
        let attrs = attr::get(&node.attrs)?;
        let variants = data
            .variants
            .iter()
            .map(Variant::from_syn)
            .collect::<Result<_>>()?;
        Ok(Enum {
            original: node,
            data,
            attrs,
            variants,
        })
    }
}

impl<'a> Variant<'a> {
    fn from_syn(node: &'a syn::Variant) -> Result<Self> {
        let attrs = attr::get(&node.attrs)?;
        Ok(Variant {
            original: node,
            attrs,
            fields: Field::from_syn_many(&node.fields)?,
        })
    }
}

impl<'a> Field<'a> {
    fn from_syn_many(fields: &'a Fields) -> Result<Vec<Self>> {
        fields.iter().map(Field::from_syn).collect()
    }

    fn from_syn(node: &'a syn::Field) -> Result<Self> {
        Ok(Field {
            original: node,
            attrs: attr::get(&node.attrs)?,
        })
    }
}
