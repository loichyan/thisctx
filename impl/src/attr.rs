use syn::{parse::Nothing, Attribute, Error, Result};

#[derive(Default)]
pub struct Attrs<'a> {
    pub source: Option<&'a Attribute>,
    pub is_source: bool,
}

pub fn get(input: &[Attribute]) -> Result<Attrs> {
    let mut attrs = Attrs::default();

    for attr in input {
        if attr.path.is_ident("source") {
            if attrs.source.is_some() {
                require_empty_attribute(attr)?;
                return Err(Error::new_spanned(attr, "duplicate #[source] attribute"));
            }
            attrs.source = Some(attr);
        }
    }

    Ok(attrs)
}

fn require_empty_attribute(attr: &Attribute) -> Result<()> {
    syn::parse2::<Nothing>(attr.tokens.clone())?;
    Ok(())
}
