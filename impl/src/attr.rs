use syn::{
    parenthesized,
    parse::{Nothing, Parse, ParseStream},
    token, Attribute, Error, Ident, LitBool, Result, Token, Visibility,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(visibility);
    custom_keyword!(suffix);
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
    pub suffix: Option<Suffix>,
}

pub enum Suffix {
    Flag(LitBool),
    Ident(Ident),
}

impl Parse for Suffix {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookhead = input.lookahead1();
        if lookhead.peek(LitBool) {
            input.parse().map(Suffix::Flag)
        } else if lookhead.peek(Ident) {
            input.parse().map(Suffix::Ident)
        } else {
            Err(lookhead.error())
        }
    }
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
                attrs.vis = parse_thisctx_arg(input)?;
            } else if lookhead.peek(kw::suffix) {
                let kw = input.parse::<kw::suffix>()?;
                if attrs.vis.is_some() {
                    return Err(Error::new_spanned(
                        kw,
                        "duplicate #[thisctx(suffix)] attribute",
                    ));
                }
                attrs.suffix = parse_thisctx_arg(input)?;
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

fn parse_thisctx_arg<T: Parse>(input: ParseStream) -> Result<Option<T>> {
    Ok(if input.peek(token::Paren) {
        let content;
        parenthesized!(content in input);
        Some(content.parse()?)
    } else {
        None
    })
}
