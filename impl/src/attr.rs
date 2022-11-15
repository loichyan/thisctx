use proc_macro2::TokenStream;
use syn::{
    parenthesized,
    parse::{Nothing, Parse, ParseStream},
    token, Attribute, Error, Ident, LitBool, Result, Token, Visibility,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(visibility);
    custom_keyword!(suffix);
    custom_keyword!(unit);
    custom_keyword!(attr);
}

#[derive(Default)]
pub struct Attrs<'a> {
    pub thisctx: Thisctx,
    pub source: Option<&'a Attribute>,
    pub is_source: bool,
}

#[derive(Default)]
pub struct Thisctx {
    pub visibility: Option<Visibility>,
    pub suffix: Option<Suffix>,
    pub unit: Option<bool>,
    pub attr: Vec<TokenStream>,
}

pub enum Suffix {
    Flag(bool),
    Ident(Ident),
}

impl Parse for Suffix {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookhead = input.lookahead1();
        if lookhead.peek(LitBool) {
            input
                .parse::<LitBool>()
                .map(|flag| Suffix::Flag(flag.value))
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
        macro_rules! check_dup {
            ($attr:ident) => {{
                let kw = input.parse::<kw::$attr>()?;
                if attrs.$attr.is_some() {
                    return Err(Error::new_spanned(
                        kw,
                        concat!("duplicate #[thisctx(", stringify!($attr), ")] attribute"),
                    ));
                }
            }};
        }

        loop {
            if input.is_empty() {
                break;
            }
            let lookhead = input.lookahead1();
            if lookhead.peek(kw::visibility) {
                check_dup!(visibility);
                attrs.visibility = parse_thisctx_arg(input)?;
            } else if lookhead.peek(kw::suffix) {
                check_dup!(suffix);
                attrs.suffix = parse_thisctx_arg(input)?;
            } else if lookhead.peek(kw::unit) {
                check_dup!(unit);
                attrs.unit = parse_thisctx_arg::<LitBool>(input)?.map(|flag| flag.value);
            } else if lookhead.peek(kw::attr) {
                input.parse::<kw::attr>()?;
                attrs.attr.extend(parse_thisctx_arg(input)?);
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
