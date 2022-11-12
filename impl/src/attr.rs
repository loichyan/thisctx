use syn::{
    parenthesized,
    parse::{Nothing, ParseStream},
    token, Attribute, Error, Result, Token, Visibility,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(visibility);
}

#[derive(Default)]
pub struct Attrs<'a> {
    pub thisctx: Thisctx,
    pub source: Option<&'a Attribute>,
    pub is_source: bool,
}

#[derive(Default)]
pub struct Thisctx {
    pub vis: Option<Visibility>,
}

pub fn get(input: &[Attribute]) -> Result<Attrs> {
    let mut attrs = Attrs::default();

    for attr in input {
        if attr.path.is_ident("thisctx") {
            parse_thisctx_attribute(&mut attrs.thisctx, attr)?;
        } else if attr.path.is_ident("source") {
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

fn parse_thisctx_attribute(attrs: &mut Thisctx, attr: &Attribute) -> Result<()> {
    attr.parse_args_with(|input: ParseStream| {
        loop {
            if input.is_empty() {
                break;
            }
            let lookhead = input.lookahead1();
            if lookhead.peek(kw::visibility) {
                let kw = input.parse::<kw::visibility>()?;
                if attrs.vis.is_some() {
                    return Err(Error::new_spanned(
                        kw,
                        "duplicate #[thisctx(visibility)] attribute",
                    ));
                }
                if input.peek(token::Paren) {
                    let content;
                    parenthesized!(content in input);
                    attrs.vis = Some(content.parse()?);
                }
            } else {
                return Err(lookhead.error());
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(())
    })
}
