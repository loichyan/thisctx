use proc_macro2::TokenStream;
use syn::{
    parenthesized,
    parse::{Nothing, Parse, ParseStream},
    token, Attribute, Error, Ident, LitBool, LitStr, Result, Token, Type, Visibility,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(attr);
    custom_keyword!(generic);
    custom_keyword!(no_generic);
    custom_keyword!(into);
    custom_keyword!(module);
    custom_keyword!(no_module);
    custom_keyword!(skip);
    custom_keyword!(no_skip);
    custom_keyword!(suffix);
    custom_keyword!(no_suffix);
    custom_keyword!(transparent);
    custom_keyword!(unit);
    custom_keyword!(no_unit);
    custom_keyword!(visibility);
}

#[derive(Default)]
pub struct Attrs<'a> {
    pub thisctx: AttrThisctx,
    pub source: Option<&'a Attribute>,
    pub error: Option<AttrError<'a>>,
}

#[derive(Default)]
pub struct AttrThisctx {
    pub attr: Vec<TokenStream>,
    pub generic: Option<bool>,
    pub into: Vec<Type>,
    pub module: Option<FlagOrIdent>,
    pub skip: Option<bool>,
    pub suffix: Option<FlagOrIdent>,
    pub unit: Option<bool>,
    pub visibility: Option<Visibility>,
}

#[derive(Default)]
pub struct AttrError<'a> {
    pub transparent: Option<&'a Attribute>,
}

pub enum FlagOrIdent {
    Flag(bool),
    Ident(Ident),
}

impl From<bool> for FlagOrIdent {
    fn from(value: bool) -> Self {
        Self::Flag(value)
    }
}

impl Parse for FlagOrIdent {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookhead = input.lookahead1();
        if lookhead.peek(LitBool) {
            input.parse::<LitBool>().map(|flag| flag.value.into())
        } else if lookhead.peek(Ident) {
            input.parse().map(FlagOrIdent::Ident)
        } else {
            Err(lookhead.error())
        }
    }
}

pub fn get(input: &[Attribute]) -> Result<Attrs> {
    let mut attrs = Attrs::default();

    for attr in input {
        macro_rules! check_dup {
            ($attr:ident) => {
                if attrs.$attr.is_some() {
                    return Err(Error::new_spanned(
                        attr,
                        concat!("duplicate #[", stringify!($attr), "] attribute"),
                    ));
                }
            };
        }

        if attr.path.is_ident("thisctx") {
            parse_thisctx_attribute(&mut attrs.thisctx, attr)?;
        } else if attr.path.is_ident("source") {
            require_empty_attribute(attr)?;
            check_dup!(source);
            attrs.source = Some(attr);
        } else if attr.path.is_ident("error") {
            check_dup!(error);
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
            ($attr:ident) => {
                check_dup!($attr, kw::$attr)
            };
            ($attr:ident, $kw:ty) => {{
                let kw = input.parse::<$kw>()?;
                if attrs.$attr.is_some() {
                    return Err(Error::new_spanned(
                        kw,
                        concat!("duplicate #[thisctx(", stringify!($attr), ")] option"),
                    ));
                }
                kw
            }};
        }

        macro_rules! parse_bool {
            ($attr:ident) => {
                check_dup!($attr);
                attrs.$attr = Some(parse_bool(input)?.into());
            };
            ($attr:ident, $no_attr:ident) => {
                check_dup!($attr, kw::$no_attr);
                attrs.$attr = Some((!parse_bool(input)?).into());
            };
        }

        loop {
            if input.is_empty() {
                break;
            }
            let lookhead = input.lookahead1();
            if lookhead.peek(kw::attr) {
                input.parse::<kw::attr>()?;
                attrs.attr.push(parse_thisctx_arg(input, true)?.unwrap());
            } else if lookhead.peek(kw::generic) {
                parse_bool!(generic);
            } else if lookhead.peek(kw::no_generic) {
                parse_bool!(generic, no_generic);
            } else if lookhead.peek(kw::into) {
                input.parse::<kw::into>()?;
                attrs.into.push(parse_thisctx_arg(input, true)?.unwrap());
            } else if lookhead.peek(kw::module) {
                check_dup!(module);
                attrs.module = Some(parse_flag_or_ident(input)?);
            } else if lookhead.peek(kw::no_module) {
                parse_bool!(module, no_module);
            } else if lookhead.peek(kw::skip) {
                parse_bool!(skip);
            } else if lookhead.peek(kw::no_skip) {
                parse_bool!(skip, no_skip);
            } else if lookhead.peek(kw::suffix) {
                check_dup!(suffix);
                attrs.suffix = Some(parse_flag_or_ident(input)?);
            } else if lookhead.peek(kw::no_suffix) {
                parse_bool!(suffix, no_suffix);
            } else if lookhead.peek(kw::unit) {
                parse_bool!(unit);
            } else if lookhead.peek(kw::no_unit) {
                parse_bool!(unit, no_unit);
            } else if lookhead.peek(kw::visibility) {
                check_dup!(visibility);
                attrs.visibility = parse_thisctx_arg(input, true)?;
            } else if lookhead.peek(Token![pub]) {
                attrs.visibility = Some(check_dup!(visibility, Visibility));
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

fn parse_flag_or_ident(input: ParseStream) -> Result<FlagOrIdent> {
    Ok(parse_thisctx_arg(input, false)?.unwrap_or(true.into()))
}

fn parse_bool(input: ParseStream) -> Result<bool> {
    Ok(parse_thisctx_arg::<LitBool>(input, false)?
        .map(|flag| flag.value)
        .unwrap_or(true))
}

fn parse_thisctx_arg<T: Parse>(input: ParseStream, required: bool) -> Result<Option<T>> {
    if input.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        let s = input.parse::<LitStr>()?;
        s.parse().map(Some)
    } else if !required && !input.peek(token::Paren) {
        Ok(None)
    } else {
        let content;
        parenthesized!(content in input);
        content.parse().map(Some)
    }
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
