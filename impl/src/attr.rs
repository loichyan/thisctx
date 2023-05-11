use proc_macro2::TokenStream;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    token, Attribute, Error, Ident, LitBool, LitStr, Meta, Result, Token, Type, Visibility,
};

#[derive(Default)]
pub struct Attrs {
    // Attributes defined in thiserror
    pub error: Option<AttrError>,
    pub source: Option<AttrSource>,
    // Where options go to
    pub thisctx: Option<AttrThisctx>,
    // Options list
    pub attr: Vec<Meta>,
    pub generic: Option<bool>,
    pub into: Vec<Type>,
    pub module: Option<FlagOrIdent>,
    pub skip: Option<bool>,
    pub suffix: Option<FlagOrIdent>,
    pub unit: Option<bool>,
    pub vis: Option<Visibility>,
}

pub enum AttrError {
    Transparent,
    Others,
}

pub struct AttrSource;

pub struct AttrThisctx;

pub enum FlagOrIdent {
    Flag(bool),
    Ident(Ident),
}

/// Parses the value of an option or attribute.
///
/// Supports two syntaxes:
///
/// 1. Quoted literal: `= "..."`
/// 2. Surrounded tokens: `(...)`
fn parse_value<T>(
    input: ParseStream,
    parser: impl FnOnce(ParseStream) -> Result<T>,
    fallback: impl FnOnce() -> Option<T>,
) -> Result<T> {
    let lookahead = input.lookahead1();
    if lookahead.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        input.parse::<LitStr>()?.parse_with(parser)
    } else if lookahead.peek(token::Paren) {
        let content;
        parenthesized!(content in input);
        parser(&content)
    } else if let Some(t) = fallback() {
        Ok(t)
    } else {
        Err(lookahead.error())
    }
}

fn parse_any<T: Parse>(input: ParseStream) -> Result<T> {
    parse_value(input, T::parse, || None)
}

/// Parses a boolean value and returns negative if specified.
fn parse_bool(input: ParseStream, neg: bool) -> Result<bool> {
    parse_value(
        input,
        |input| Ok(input.parse::<LitBool>()?.value() ^ neg),
        || Some(true ^ neg),
    )
}

/// Similar to [`parse_bool`] and also accepts an identifier as a value.
fn parse_flag_or_ident(input: ParseStream, neg: bool) -> Result<FlagOrIdent> {
    parse_value(
        input,
        |input| {
            let lookahead = input.lookahead1();
            if lookahead.peek(LitBool) {
                Ok(FlagOrIdent::Flag(input.parse::<LitBool>()?.value ^ neg))
            // Identifiers in a negative option are meaningless.
            } else if !neg && lookahead.peek(Ident) {
                input.parse().map(FlagOrIdent::Ident)
            } else {
                Err(lookahead.error())
            }
        },
        || Some(FlagOrIdent::Flag(true ^ neg)),
    )
}

fn parse_options(input: ParseStream, opts: &mut Attrs) -> Result<()> {
    let mut lookahead;

    macro_rules! ensure_once {
        ($name:ident) => {
            if opts.$name.is_some() {
                return Err(Error::new(
                    input.span(),
                    format!("duplicate #[thisctx({})] option", stringify!($name)),
                ));
            }
        };
    }

    macro_rules! parse_opts {
        () => {
            return Err(lookahead.error());
        };
        // Option appears at most once
        ($opt:ident = $kw:ident use $parser:ident($($args:tt)*), $($rest:tt)*) => {
            syn::custom_keyword!($kw);
            if lookahead.peek($kw) {
                ensure_once!($opt);
                input.parse::<$kw>()?;
                opts.$opt = Some($parser(input, $($args)*)?);
            } else {
                parse_opts!($($rest)*);
            }
        };
        // Repeatable option
        ($opt:ident += $kw:ident use $parser:ident($($args:tt)*), $($rest:tt)*) => {
            syn::custom_keyword!($kw);
            if lookahead.peek($kw) {
                input.parse::<$kw>()?;
                opts.$opt.push($parser(input, $($args)*)?);
            } else {
                parse_opts!($($rest)*);
            }
        };
        // Attribute shortcut
        ($opt:ident #= $kw:ident, $($rest:tt)*) => {
            syn::custom_keyword!($kw);
            if lookahead.peek($kw) {
                opts.$opt.push(input.parse()?);
            } else {
                parse_opts!($($rest)*);
            }
        };
    }

    loop {
        if input.is_empty() {
            break;
        }
        lookahead = input.lookahead1();

        if lookahead.peek(Token![pub]) {
            ensure_once!(vis);
            opts.vis = Some(input.parse()?);
        } else {
            parse_opts! {
                attr   += attr       use parse_any(),
                attr   #= cfg,
                attr   #= cfg_attr,
                attr   #= derive,
                attr   #= doc,

                generic = generic    use parse_bool(false),
                generic = no_generic use parse_bool(true),

                into   += into       use parse_any(),

                module  = module     use parse_flag_or_ident(false),
                module  = no_module  use parse_flag_or_ident(true),

                skip    = skip       use parse_bool(false),
                skip    = no_skip    use parse_bool(true),

                suffix  = suffix     use parse_flag_or_ident(false),
                suffix  = no_suffix  use parse_flag_or_ident(true),

                unit    = unit       use parse_bool(false),
                unit    = no_unit    use parse_bool(true),

                vis     = vis        use parse_any(),
            }
        }

        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }

    Ok(())
}

fn parse_opt_error(input: ParseStream) -> Result<AttrError> {
    syn::custom_keyword!(transparent);

    if input.parse::<Option<transparent>>()?.is_some() {
        Ok(AttrError::Transparent)
    } else {
        input.parse::<TokenStream>()?;
        Ok(AttrError::Others)
    }
}

pub fn get(input: &[Attribute]) -> Result<Attrs> {
    let mut attrs = Attrs::default();
    let mut attr;
    let mut path;

    macro_rules! ensure_once {
        ($name:ident) => {
            if attrs.$name.is_some() {
                return Err(Error::new(
                    path.span(),
                    format!("duplicate #[{}] attribute", stringify!($name)),
                ));
            }
        };
    }

    for t in input {
        attr = t;
        if let Some(t) = attr.path().get_ident() {
            path = t;
            if path == "thisctx" {
                attr.parse_args_with(|input: ParseStream| parse_options(input, &mut attrs))?;
            } else if path == "source" {
                ensure_once!(source);
                attr.meta.require_path_only()?;
                attrs.source = Some(AttrSource);
            } else if path == "error" {
                ensure_once!(error);
                attrs.error = Some(attr.parse_args_with(parse_opt_error)?);
            }
        }
    }

    Ok(attrs)
}
