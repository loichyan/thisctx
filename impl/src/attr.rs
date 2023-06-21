use plap::{Arg, ArgAction, Args, DefaultFormatter, NamedArg, Parser, ParserContext};
use proc_macro2::{Span, TokenStream};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Attribute, Ident, Meta, Result, Token, Type, Visibility,
};

#[derive(Default)]
pub struct Attrs {
    // Attributes defined in thiserror
    pub error: Option<AttrError>,
    pub source: Option<AttrSource>,
    // Where options put in
    pub thisctx: Opts,
}

#[derive(Default)]
pub struct Opts {
    // All options
    pub attr: Vec<Meta>,
    pub into: Vec<Type>,
    pub vis: Option<Visibility>,
    pub generic: Option<bool>,
    pub module: Option<FlagOr<Ident>>,
    pub skip: Option<bool>,
    pub suffix: Option<FlagOr<Ident>>,
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

pub struct OptsParser {
    context: ParserContext,
    // Attribute options
    attr: Arg<Meta>,
    cfg: Arg<Meta>,
    cfg_attr: Arg<Meta>,
    derive: Arg<Meta>,
    doc: Arg<Meta>,
    // Visibility options
    pub_: Arg<Visibility>,
    vis: Arg<Visibility>,
    visibility: Arg<Visibility>,
    // Normal options
    into: Arg<Type>,
    generic: Arg<bool>,
    module: Arg<FlagOr<Ident>>,
    skip: Arg<bool>,
    suffix: Arg<FlagOr<Ident>>,
    unit: Arg<bool>,
    // Negative flag options
    no_generic: Arg<bool>,
    no_module: Arg<bool>,
    no_skip: Arg<bool>,
    no_suffix: Arg<bool>,
    no_unit: Arg<bool>,
}

pub enum FlagOr<T> {
    Flag(bool),
    Or(T),
}

impl<T> FlagOr<T> {
    fn parse_named(input: ParseStream) -> Result<Self>
    where
        T: Parse,
    {
        input.parse::<Ident>()?;
        if input.peek(Token![,]) || input.is_empty() {
            Ok(Self::Flag(true))
        } else {
            let (_, value) = NamedArg::parse_value(input)?;
            Ok(Self::Or(value))
        }
    }
}

impl Args for Opts {
    type Parser = OptsParser;
}

impl Parser for OptsParser {
    type Output = Opts;

    fn with_node(node: Span) -> Self {
        let mut context = ParserContext::builder()
            .node(node)
            .formatter(DefaultFormatter::builder().namespace("thisctx").build());
        OptsParser {
            attr: context.arg("attr").action(ArgAction::Append),
            cfg: context.arg("cfg").action(ArgAction::Append),
            cfg_attr: context.arg("cfg_attr").action(ArgAction::Append),
            derive: context.arg("derive").action(ArgAction::Append),
            doc: context.arg("doc").action(ArgAction::Append),
            pub_: context
                .arg("pub")
                .action(ArgAction::Set)
                .group("pub|vis|visibility"),
            vis: context
                .arg("vis")
                .action(ArgAction::Set)
                .group("pub|vis|visibility"),
            visibility: context
                .arg("visibility")
                .action(ArgAction::Set)
                .group("pub|vis|visibility"),
            into: context.arg("into").action(ArgAction::Append),
            generic: context.arg("generic").action(ArgAction::Set),
            module: context.arg("module").action(ArgAction::Set),
            skip: context.arg("skip").action(ArgAction::Set),
            suffix: context.arg("suffix").action(ArgAction::Set),
            unit: context.arg("unit").action(ArgAction::Set),
            no_generic: context
                .arg("no_generic")
                .action(ArgAction::Set)
                .conflicts_with("generic"),
            no_module: context
                .arg("no_module")
                .action(ArgAction::Set)
                .conflicts_with("module"),
            no_skip: context
                .arg("no_skip")
                .action(ArgAction::Set)
                .conflicts_with("skip"),
            no_suffix: context
                .arg("no_suffix")
                .action(ArgAction::Set)
                .conflicts_with("suffix"),
            no_unit: context
                .arg("no_unit")
                .action(ArgAction::Set)
                .conflicts_with("unit"),
            context: context.build(),
        }
    }

    fn context(&self) -> &ParserContext {
        &self.context
    }

    fn parse_once(&mut self, input: ParseStream) -> Result<bool> {
        let span = input.span();
        macro_rules! parse_opts {
            ($($name:ident as $parser:tt,)*) => {
                $(syn::custom_keyword!($name);)*
                if false {}
                $(else if input.peek($name) {
                    self.$name.add_value(span, parse_opts!(@parser $parser)(input)?);
                })* else {
                    return Ok(false);
                }
            };
            (@parser named) => {
                plap::ParseStreamExt::parse_named_arg
            };
            (@parser flag) => {
                plap::ParseStreamExt::parse_flag_arg
            };
            (@parser flag_or_named) => {
                FlagOr::parse_named
            };
            (@parser _) => {
                syn::parse::Parse::parse
            };
        }
        if input.peek(Token![pub]) {
            self.pub_.add_value(span, input.parse()?);
        } else {
            parse_opts!(
                attr as named,
                cfg as _,
                cfg_attr as _,
                derive as _,
                doc as _,
                vis as named,
                visibility as named,
                into as named,
                generic as flag,
                module as flag_or_named,
                skip as flag,
                suffix as flag_or_named,
                unit as flag,
                no_generic as flag,
                no_module as flag,
                no_skip as flag,
                no_suffix as flag,
                no_unit as flag,
            );
        }
        Ok(true)
    }

    fn finish(self) -> Result<Self::Output> {
        self.context.finish()?;
        Ok(Opts {
            attr: self
                .attr
                .into_values()
                .chain(self.cfg.into_values())
                .chain(self.cfg_attr.into_values())
                .chain(self.derive.into_values())
                .chain(self.doc.into_values())
                .collect(),
            into: self.into.into_values().collect(),
            vis: self
                .pub_
                .into_values()
                .next()
                .or_else(|| self.vis.into_option())
                .or_else(|| self.visibility.into_option()),
            generic: arg_flag(self.generic, self.no_generic),
            module: arg_flag_or(self.module, self.no_module),
            skip: arg_flag(self.skip, self.no_skip),
            suffix: arg_flag_or(self.suffix, self.no_suffix),
            unit: arg_flag(self.unit, self.no_unit),
        })
    }
}

fn arg_flag_or<T>(y: Arg<FlagOr<T>>, n: Arg<bool>) -> Option<FlagOr<T>> {
    if let Some(value) = y.into_option() {
        Some(value)
    } else if !n.is_empty() {
        Some(FlagOr::Flag(false))
    } else {
        None
    }
}

fn arg_flag(y: Arg<bool>, n: Arg<bool>) -> Option<bool> {
    if !y.is_empty() {
        Some(true)
    } else if !n.is_empty() {
        Some(false)
    } else {
        None
    }
}

pub enum Node<'a> {
    Container(&'a syn::DeriveInput),
    Variant(&'a syn::Variant),
    Field(&'a syn::Field),
}

pub fn get(node: Node, input: &[Attribute]) -> Result<Attrs> {
    let mut attr;
    let mut span;
    let mut attrs = Attrs::default();
    let mut opts = Opts::parser(match node {
        Node::Container(c) => c.ident.span(),
        Node::Variant(v) => v.ident.span(),
        Node::Field(f) => f.ident.as_ref().map_or_else(|| f.ty.span(), Ident::span),
    });

    macro_rules! no_duplicate {
        ($name:ident) => {
            if attrs.$name.is_some() {
                return Err(syn::Error::new(
                    span,
                    concat!("`", stringify!($name), "` is duplicated"),
                ));
            }
        };
    }

    macro_rules! unexpected_in {
        ($node:literal => $($name:ident),* $(,)?) => {{$(
            for &span in opts.$name.spans() {
                opts.context().error(syn::Error::new(
                    span,
                    concat!("`thisctx.", stringify!($name), "` is not allowed in ", $node),
                ));
            }
        )*}};
    }

    for t in input {
        attr = t;
        if let Some(path) = attr.path().get_ident() {
            span = path.span();
            if path == "thisctx" {
                attr.parse_args_with(|input: ParseStream| opts.parse(input))?;
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

    match node {
        Node::Container(_) => unexpected_in!("a container" =>),
        Node::Variant(_) => unexpected_in!("a variant" => module, no_module),
        Node::Field(_) => {
            unexpected_in!("a filed" => into, module, skip, unit, no_module, no_skip, no_unit)
        }
    }

    attrs.thisctx = opts.finish()?;
    Ok(attrs)
}
