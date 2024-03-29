use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Nothing, Parse, ParseStream},
    token, Attribute, Error, Ident, LitBool, LitStr, Result, Token, Type, Visibility,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(attr);
    custom_keyword!(generic);
    custom_keyword!(into);
    custom_keyword!(module);
    custom_keyword!(skip);
    custom_keyword!(suffix);
    custom_keyword!(transparent);
    custom_keyword!(unit);
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

fn parse_thisctx_attribute(options: &mut AttrThisctx, original: &Attribute) -> Result<()> {
    original.parse_args_with(|input: ParseStream| {
        macro_rules! check_dup {
            ($opt:ident) => {
                check_dup!($opt as kw::$opt)
            };
            ($opt:ident as $kw:ty) => {{
                let kw = input.parse::<$kw>()?;
                if options.$opt.is_some() {
                    return Err(Error::new_spanned(
                        kw,
                        concat!("duplicate #[thisctx(", stringify!($opt), ")] option"),
                    ));
                }
                kw
            }};
        }

        loop {
            if input.is_empty() {
                break;
            }
            let lookhead = input.lookahead1();

            macro_rules! parse_opts {
                () => {
                    return Err(lookhead.error());
                };
                ($opt:ident = $kw:ident, $($rest:tt)*) => {
                    parse_opts!($opt @= $kw(ParseThisctxOpt::parse(input)?), $($rest)*);
                };
                ($opt:ident ~= $kw:ident, $($rest:tt)*) => {
                    parse_opts!($opt @= $kw((!<bool as ParseThisctxOpt>::parse(input)?).into()), $($rest)*);
                };
                ($opt:ident @= $kw:ident($val:expr), $($rest:tt)*) => {
                    syn::custom_keyword!($kw);
                    if lookhead.peek($kw) {
                        check_dup!($opt as $kw);
                        options.$opt = Some($val);
                    } else {
                        parse_opts!($($rest)*);
                    }
                };
                ($opt:ident += $kw:ident, $($rest:tt)*) => {
                    syn::custom_keyword!($kw);
                    if lookhead.peek($kw) {
                        input.parse::<$kw>()?;
                        options.$opt.push(parse_thisctx_opt(input, true)?.unwrap());
                    } else {
                        parse_opts!($($rest)*);
                    }
                };
                ($opt:ident #= $kw:ident, $($rest:tt)*) => {
                    syn::custom_keyword!($kw);
                    if lookhead.peek($kw) {
                        options.$opt.push(input.parse::<syn::Meta>()?.into_token_stream());
                    } else {
                        parse_opts!($($rest)*);
                    }
                };
            }

            if lookhead.peek(Token![pub]) {
                options.visibility = Some(check_dup!(visibility as Visibility));
            } else {
                parse_opts! {
                    attr       += attr,
                    generic     = generic,
                    into       += into,
                    module      = module,
                    skip        = skip,
                    suffix      = suffix,
                    unit        = unit,
                    visibility  = visibility,
                    // Reversed options
                    generic    ~= no_generic,
                    module     ~= no_module,
                    skip       ~= no_skip,
                    suffix     ~= no_suffix,
                    unit       ~= no_unit,
                    // Attribute shortcut
                    attr       #= cfg,
                    attr       #= cfg_attr,
                    attr       #= derive,
                    attr       #= doc,
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(())
    })
}

trait ParseThisctxOpt: Sized {
    fn parse(input: ParseStream) -> Result<Self>;
}

impl ParseThisctxOpt for Visibility {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_thisctx_opt(input, true).map(Option::unwrap)
    }
}

impl ParseThisctxOpt for FlagOrIdent {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(parse_thisctx_opt(input, false)?.unwrap_or(true.into()))
    }
}

impl ParseThisctxOpt for bool {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(parse_thisctx_opt::<LitBool>(input, false)?
            .map(|flag| flag.value)
            .unwrap_or(true))
    }
}

fn parse_thisctx_opt<T: Parse>(input: ParseStream, required: bool) -> Result<Option<T>> {
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
