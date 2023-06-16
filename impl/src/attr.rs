use proc_macro2::{Span, TokenStream};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token, Attribute, Error, Ident, LitBool, LitStr, Meta, Result, Token, Type, Visibility,
};

#[derive(Default)]
pub struct Attrs {
    // Attributes defined in thiserror
    pub error: Option<AttrError>,
    pub source: Option<AttrSource>,
    // Where options put in
    pub thisctx: Option<AttrThisctx>,
    // Repeatable options
    pub attr: Vec<Meta>,
    pub into: Vec<Type>,
    // Normal options
    pub vis: Option<Visibility>,
    // Flag options
    pub generic: Option<bool>,
    pub module: Option<FlagOrIdent>,
    pub skip: Option<bool>,
    pub suffix: Option<FlagOrIdent>,
    pub unit: Option<bool>,
}

#[derive(Clone, Copy)]
pub enum AttrError {
    Transparent,
    Others,
}

#[derive(Clone, Copy)]
pub struct AttrSource;

#[derive(Clone, Copy)]
pub struct AttrThisctx;

trait NamedValue: Sized {
    fn parse(input: ParseStream) -> Result<Self>;

    /// Returns an optional default value of this option.
    fn fallback() -> Option<Self> {
        None
    }
}

impl<T: Parse> NamedValue for T {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse()
    }
}

struct Named<V> {
    pub name: Ident,
    pub value: V,
}

impl<T> Named<T> {
    fn span(&self) -> Span {
        self.name.span()
    }
}

impl<V: NamedValue> Parse for Named<V> {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            name: input.parse()?,
            value: parse_value(input)?,
        })
    }
}

pub enum FlagOrIdent {
    Flag(bool),
    Ident(Ident),
}

impl NamedValue for FlagOrIdent {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitBool) {
            input.parse::<LitBool>().map(|t| t.value).map(Self::Flag)
        } else if lookahead.peek(Ident) {
            input.parse().map(Self::Ident)
        } else {
            Err(lookahead.error())
        }
    }

    fn fallback() -> Option<Self> {
        Some(Self::Flag(true))
    }
}

#[derive(Clone, Copy)]
struct Flag(bool);

impl std::ops::Not for Flag {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl From<Flag> for bool {
    fn from(val: Flag) -> Self {
        val.0
    }
}

impl NamedValue for Flag {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<LitBool>().map(|t| t.value).map(Self)
    }

    fn fallback() -> Option<Self> {
        Some(Self(true))
    }
}

impl From<Flag> for FlagOrIdent {
    fn from(val: Flag) -> Self {
        FlagOrIdent::Flag(val.0)
    }
}

/// Parses the value of an option or attribute.
///
/// Supports two syntaxes:
///
/// 1. Quoted literal: `= "..."`
/// 2. Surrounded tokens: `(...)`
fn parse_value<T: NamedValue>(input: ParseStream) -> Result<T> {
    let lookahead = input.lookahead1();
    if lookahead.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        input.parse::<LitStr>()?.parse_with(T::parse)
    } else if lookahead.peek(token::Paren) {
        let content;
        parenthesized!(content in input);
        T::parse(&content)
    } else if let Some(t) = T::fallback() {
        Ok(t)
    } else {
        Err(lookahead.error())
    }
}

macro_rules! Many {
    (@type $ty:ty) => { Vec<$ty> };
    (@update $self:ident, $node:ident, $field:ident, $val:ident) => {
        $self.$field.push($val);
    };
    ($($tt:tt)*) => {};
}

macro_rules! Unique {
    (@type $ty:ty) => { Option<$ty> };
    (@update $self:ident, $node:ident, $field:ident, $val:ident) => {
        if $self.$field.is_some() {
            return Err(syn::Error::new(
                $val.span(),
                concat!("option `thisctx.", stringify!($field), "` is duplicated"),
            ));
        }
        $self.$field = Some($val);
    };
    ($($tt:tt)*) => {};
}

macro_rules! conflicts_with {
    (@before:update $self:ident, $node:ident, $field:ident, $val:ident $(,$conflicts:ident)*) => {
        if false $(|| $self.$conflicts.is_some())* {
            return Err(syn::Error::new(
                $val.span(),
                concat!(
                    "option `thisctx.", stringify!($field), "` conflicts with",
                    $(" `thisctx.", stringify!($conflicts), "`",)*
                ),
            ));
        }
    };
    ($($tt:tt)*) => {};
}

macro_rules! used_in {
    (@parse $self:ident, $node:ident, $input:ident, $field:ident $(,$ty:ident)*) => {
        if [$(NodeType::$ty)*].find(&node) {
            return Err(syn::Error::new(
                $input.span(),
                concat!(
                    "option `thisctx.", stringify!($field), "` can only be used in",
                    $(" `", stringify!($ty), "`",)*
                ),
            ));
        }
    };
    ($($tt:tt)*) => {};
}

macro_rules! define_opts {
    ($(#[$attr:meta])* $vis:vis struct $name:ident {$(
        $(@$f_hook:path[$($args:tt)*])*
        $(#[$f_attr:meta])*
        $f_vis:vis $f_name:ident: $wrapper:path[$f_ty:ty],
    )*}) => {
        $(#[$attr])* $vis struct $name {
            pub pub_: Unique![@type Visibility],
            $($(#[$f_attr])* $f_vis $f_name: $wrapper![@type $f_ty],)*
        }

        impl $name {
            fn parse(&mut self, _: NodeType, input: syn::parse::ParseStream) -> syn::Result<()> {
                mod __kw {$(syn::custom_keyword!($f_name);)*}

                loop {
                    if input.is_empty() { break; }
                    let lookahead = input.lookahead1();

                    if false { unreachable!(); }
                    else if lookahead.peek(syn::token::Pub) {
                        let val = input.parse::<syn::Visibility>()?;
                        conflicts_with!(self, node, pub, val, vis, visibility);
                        Unique!(@update self, node, pub_, val);
                    }
                    $(else if lookahead.peek(__kw::$f_name) {
                        let val = input.parse::<$f_ty>()?;
                        $($f_hook!(@before:update self, node, $f_name, val, $($args)*);)*
                        $wrapper!(@update self, node, $f_name, val);
                    })*
                    else { return Err(lookahead.error()); }

                    if input.is_empty() { break; }
                    input.parse::<syn::token::Comma>()?;
                }

                Ok(())
            }
        }
    };
}

define_opts! {
    #[derive(Default)]
    struct Opts {
        pub attr:     Many[Named<Meta>],
        pub into:     Many[Named<Type>],
        pub cfg:      Many[Meta],
        pub cfg_attr: Many[Meta],
        pub derive:   Many[Meta],
        pub doc:      Many[Meta],

        @conflicts_with[visibility]
        pub vis:        Unique[Named<Visibility>],
        @conflicts_with[vis]
        pub visibility: Unique[Named<Visibility>],
        @conflicts_with[no_generic]
        pub generic:    Unique[Named<Flag>],
        @conflicts_with[no_module]
        pub module:     Unique[Named<FlagOrIdent>],
        @conflicts_with[no_skip]
        pub skip:       Unique[Named<Flag>],
        @conflicts_with[no_suffix]
        pub suffix:     Unique[Named<FlagOrIdent>],
        @conflicts_with[no_unit]
        pub unit:       Unique[Named<Flag>],

        @conflicts_with[generic]
        pub no_generic: Unique[Named<Flag>],
        @conflicts_with[module]
        pub no_module:  Unique[Named<Flag>],
        @conflicts_with[skip]
        pub no_skip:    Unique[Named<Flag>],
        @conflicts_with[suffix]
        pub no_suffix:  Unique[Named<Flag>],
        @conflicts_with[unit]
        pub no_unit:    Unique[Named<Flag>],
    }
}

#[derive(Clone, Copy)]
pub enum NodeType {
    Container,
    Variant,
    Field,
}

pub fn get(node: NodeType, input: &[Attribute]) -> Result<Attrs> {
    let mut attr;
    let mut path;
    let mut attrs = Attrs::default();
    let mut opts = Opts::default();

    macro_rules! no_duplicate {
        ($name:ident) => {
            if attrs.$name.is_some() {
                return Err(Error::new(
                    path.span(),
                    concat!("attribute `", stringify!($name), "` is duplicated"),
                ));
            }
        };
    }

    macro_rules! update_attrs {
        () => {};
        ($attr:ident + $opt:ident, $($rest:tt)*) => {
            attrs.$attr.extend(opts.$opt.into_iter().map(|t| t.value));
            update_attrs!($($rest)*);
        };
        ($attr:ident +# $opt:ident, $($rest:tt)*) => {
            attrs.$attr.extend(opts.$opt.into_iter());
            update_attrs!($($rest)*);
        };
        ($attr:ident = $opt:ident, $($rest:tt)*) => {
            attrs.$attr = opts.$opt.map(|t| t.value.into());
            update_attrs!($($rest)*);
        };
        ($attr:ident =! $opt:ident, $($rest:tt)*) => {
            attrs.$attr = opts.$opt.map(|t| (!t.value).into());
            update_attrs!($($rest)*);
        };
    }

    for t in input {
        attr = t;
        if let Some(t) = attr.path().get_ident() {
            path = t;
            if path == "thisctx" {
                attr.parse_args_with(|input: ParseStream| opts.parse(node, input))?;
            } else if path == "source" {
                no_duplicate!(source);
                attr.meta.require_path_only()?;
                attrs.source = Some(AttrSource);
            } else if path == "error" {
                no_duplicate!(error);
                attrs.error = Some(attr.parse_args_with(|input: ParseStream| {
                    syn::custom_keyword!(transparent);

                    if input.parse::<Option<transparent>>()?.is_some() {
                        Ok(AttrError::Transparent)
                    } else {
                        input.parse::<TokenStream>()?;
                        Ok(AttrError::Others)
                    }
                })?);
            }
        }
    }

    update_attrs! {
        attr + attr,
        into + into,
        attr +# cfg,
        attr +# cfg_attr,
        attr +# derive,
        attr +# doc,

        vis     = vis,
        vis     = visibility,
        generic = generic,
        module  = module,
        skip    = skip,
        suffix  = suffix,
        unit    = unit,

        generic =! no_generic,
        module  =! no_module,
        skip    =! no_skip,
        suffix  =! no_suffix,
        unit    =! no_unit,
    }

    Ok(attrs)
}
