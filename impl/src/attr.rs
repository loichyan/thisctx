use proc_macro2::TokenStream;
use syn::{
    parenthesized,
    parse::{Nothing, Parse, ParseStream},
    token, Attribute, Error, Ident, LitBool, Result, Token, Type, Visibility,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(visibility);
    custom_keyword!(suffix);
    custom_keyword!(unit);
    custom_keyword!(attr);
    custom_keyword!(into);
    custom_keyword!(transparent);
    custom_keyword!(generic);
    custom_keyword!(context);
    custom_keyword!(module);
}

#[derive(Default)]
pub struct Attrs<'a> {
    pub thisctx: AttrThisctx,
    pub source: Option<&'a Attribute>,
    pub error: Option<AttrError<'a>>,
}

#[derive(Default)]
pub struct AttrThisctx {
    pub visibility: Option<Visibility>,
    pub suffix: Option<Suffix>,
    pub unit: Option<bool>,
    pub attr: Vec<TokenStream>,
    pub into: Vec<Type>,
    pub generic: Option<bool>,
    pub context: Option<bool>,
    pub module: Option<Ident>,
}

#[derive(Default)]
pub struct AttrError<'a> {
    pub transparent: Option<&'a Attribute>,
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

    macro_rules! check_dup {
        ($original:expr, $attr:ident) => {
            if attrs.$attr.is_some() {
                return Err(Error::new_spanned(
                    $original,
                    concat!("duplicate #[", stringify!($attr), "] attribute"),
                ));
            }
        };
    }

    for attr in input {
        if attr.path.is_ident("thisctx") {
            parse_thisctx_attribute(&mut attrs.thisctx, attr)?;
        } else if attr.path.is_ident("source") {
            require_empty_attribute(attr)?;
            check_dup!(attr, source);
            attrs.source = Some(attr);
        } else if attr.path.is_ident("error") {
            check_dup!(attr, error);
            attrs.error = Some(parse_error_attribute(attr)?);
        }
    }
    Ok(attrs)
}

fn require_empty_attribute(attr: &Attribute) -> Result<()> {
    syn::parse2::<Nothing>(attr.tokens.clone())?;
    Ok(())
}

fn parse_thisctx_attribute(attrs: &mut AttrThisctx, original: &Attribute) -> Result<()> {
    original.parse_args_with(|input: ParseStream| {
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
                attrs.unit = parse_bool(input)?;
            } else if lookhead.peek(kw::attr) {
                input.parse::<kw::attr>()?;
                attrs.attr.extend(parse_thisctx_arg(input)?);
            } else if lookhead.peek(kw::into) {
                input.parse::<kw::into>()?;
                attrs.into.extend(parse_thisctx_arg(input)?);
            } else if lookhead.peek(kw::generic) {
                check_dup!(generic);
                attrs.generic = parse_bool(input)?;
            } else if lookhead.peek(kw::context) {
                check_dup!(context);
                attrs.context = parse_bool(input)?;
            } else if lookhead.peek(kw::module) {
                check_dup!(module);
                attrs.module = parse_thisctx_arg(input)?;
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

fn parse_bool(input: ParseStream) -> Result<Option<bool>> {
    Ok(parse_thisctx_arg::<LitBool>(input)?.map(|flag| flag.value))
}

// TODO: support `attr("...")` and `attr = ...`
fn parse_thisctx_arg<T: Parse>(input: ParseStream) -> Result<Option<T>> {
    Ok(if input.peek(token::Paren) {
        let content;
        parenthesized!(content in input);
        Some(content.parse()?)
    } else {
        None
    })
}

fn parse_error_attribute(attr: &Attribute) -> Result<AttrError> {
    attr.parse_args_with(|input: ParseStream| {
        let mut error = AttrError::default();
        if input.peek(kw::transparent) {
            input.parse::<kw::transparent>()?;
            error.transparent = Some(attr);
        } else {
            input.parse::<TokenStream>()?;
        }
        Ok(error)
    })
}
