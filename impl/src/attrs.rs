use plap::group;
use proc_macro2::TokenStream;
use syn::parse::{Nothing, ParseStream};
use syn::{Ident, LitBool, Type, Visibility};

pub(crate) fn parse_container(input: &syn::DeriveInput) -> syn::Result<Attrs> {
    let (mut c, thisctx, thiserror) = parse_args(&input.attrs)?;
    c.blocked_all(group![&thisctx.from, &thisctx.optional, &thiserror.source]);
    if matches!(input.data, syn::Data::Enum(_)) {
        c.blocked_all(group![&thisctx.rename, &thiserror.transparent]);
    } else {
        c.blocked(&thisctx.skip);
    }
    build_attrs(&mut c, thisctx, thiserror)
}

pub(crate) fn parse_variant(input: &syn::Variant) -> syn::Result<Attrs> {
    let (mut c, thisctx, thiserror) = parse_args(&input.attrs)?;
    c.blocked_all(group![
        &thisctx.from,
        &thisctx.module,
        &thisctx.optional,
        &thiserror.source,
    ]);
    build_attrs(&mut c, thisctx, thiserror)
}

pub(crate) fn parse_field(input: &syn::Field) -> syn::Result<Attrs> {
    let (mut c, thisctx, thiserror) = parse_args(&input.attrs)?;
    c.blocked_all(group![
        &thisctx.module,
        &thisctx.prefix,
        &thisctx.remote,
        &thisctx.skip,
        &thisctx.suffix,
        // field visibility are the same as its parent
        &thisctx.vis,
        &thisctx.visibility,
        &thiserror.transparent,
    ]);
    if input.ident.is_none() {
        for (key, value) in thisctx
            .optional
            .keys()
            .iter()
            .zip(thisctx.optional.values())
        {
            if value.0.is_none() {
                c.with_error_at(
                    key.span(),
                    format!("`{}` requires a value on tuple fields", key),
                );
            }
        }
    }
    build_attrs(&mut c, thisctx, thiserror)
}

fn parse_args(
    attrs: &[syn::Attribute],
) -> syn::Result<(plap::Checker, ThisctxArgs, ThiserrorArgs)> {
    let mut c = plap::Checker::default();
    let mut thisctx = <ThisctxArgs as plap::Args>::init();
    let mut thiserror = ThiserrorArgs {
        transparent: plap::Arg::new("transparent"),
        source: plap::Arg::new("source"),
    };
    for (name, attr) in attrs
        .iter()
        .flat_map(|a| a.path().get_ident().map(|i| (i, a)))
    {
        if name == "thisctx" {
            let r = attr.parse_args_with(|input: ParseStream| {
                plap::Parser::new(input).parse_all(&mut thisctx)
            });
            c.with_source(name.span());
            c.with_result(r);
        } else if name == "error" {
            attr.parse_args_with(|input: ParseStream| {
                syn::custom_keyword!(transparent);
                if input.peek(transparent) {
                    let k = input.parse::<Ident>()?;
                    thiserror.transparent.add(k, Nothing);
                } else {
                    // ignore other values
                    while input.parse::<Option<proc_macro2::TokenTree>>()?.is_some() {}
                }
                Ok(())
            })?;
        } else if name == "source" {
            thiserror.source.add(name.clone(), Nothing);
        }
    }

    // checks between thisctx and thiserror attributes
    c.conflicts_with(&thisctx.attr, &thiserror.source);
    c.conflicts_with(&thisctx.attribute, &thiserror.source);
    c.conflicts_with_each(
        &thisctx.magic,
        group![&thiserror.source, &thiserror.transparent],
    );

    Ok((c, thisctx, thiserror))
}

fn build_attrs(
    c: &mut plap::Checker,
    thisctx: ThisctxArgs,
    thiserror: ThiserrorArgs,
) -> syn::Result<Attrs> {
    plap::Args::check(&thisctx, c);
    c.finish()?;
    let ThisctxArgs {
        attr,
        attribute,
        from,
        magic,
        module,
        optional,
        prefix,
        remote,
        rename,
        skip,
        suffix,
        vis,
        visibility,
    } = thisctx;
    let ThiserrorArgs {
        transparent,
        source,
    } = thiserror;
    Ok(Attrs {
        attr: attr
            .take_any()
            .into_iter()
            .chain(attribute.take_any())
            .collect(),
        from: from.take_flag(),
        magic: magic.take_last().map(|t| t.value()),
        module: module.take_last(),
        optional: optional.take_last().map(|t| t.0),
        prefix: prefix.take_last(),
        remote: remote.take_last(),
        rename: rename.take_last(),
        skip: skip.take_last().map(|t| t.value()),
        source: !source.is_empty(),
        suffix: suffix.take_last(),
        transparent: !transparent.is_empty(),
        vis: vis.take_last().or_else(|| visibility.take_last()),
    })
}

pub(crate) struct Attrs {
    // field, struct, variant -> enum
    pub attr: Vec<TokenStream>,
    // field
    pub from: bool,
    // field -> struct, field -> variant -> enum
    pub magic: Option<bool>,
    // struct, enum
    pub module: Option<Ident>,
    // field
    pub optional: Option<Option<Ident>>,
    // struct, variant -> enum
    pub prefix: Option<Ident>,
    // struct, variant -> enum
    pub remote: Option<Type>,
    // struct, variant
    pub rename: Option<Ident>,
    // variant -> enum
    pub skip: Option<bool>,
    // #[thisctx(source)] or #[source]
    // field
    pub source: bool,
    // struct, variant -> enum
    pub suffix: Option<Ident>,
    // #[thisctx(transparent)] or #[error(transparent)]
    // struct, variant
    pub transparent: bool,
    // struct, variant -> enum
    pub vis: Option<Visibility>,
}

pub(crate) struct ThiserrorArgs {
    pub transparent: plap::Arg<Nothing>,
    pub source: plap::Arg<Nothing>,
}

plap::define_args!(
    #[check(exclusive_aliases = [vis, visibility])]
    struct ThisctxArgs {
        #[arg(is_token_tree)]
        #[check(conflicts_with_each = [from, optional])]
        attr: plap::Arg<TokenStream>,

        #[arg(is_token_tree)]
        #[check(conflicts_with_each = [from, optional])]
        attribute: plap::Arg<TokenStream>,

        #[arg(is_flag)]
        #[check(exclusive, conflicts_with = optional)]
        from: plap::Arg<LitBool>,

        #[arg(is_flag)]
        #[check(exclusive, conflicts_with_each = [from ,optional])]
        magic: plap::Arg<LitBool>,

        // TODO: link docs to argument keys
        // #[arg(is_help)]
        // help: plap::Arg<LitBool>,
        #[arg(is_token_tree)]
        #[check(exclusive)]
        module: plap::Arg<Ident>,

        #[arg(is_token_tree, optional)]
        #[check(exclusive)]
        optional: plap::OptionalArg<Ident>,

        #[arg(is_token_tree)]
        #[check(exclusive)]
        prefix: plap::Arg<Ident>,

        #[arg(is_token_tree)]
        #[check(exclusive)]
        remote: plap::Arg<Type>,

        #[arg(is_token_tree)]
        #[check(exclusive, conflicts_with_each = [prefix, suffix])]
        rename: plap::Arg<Ident>,

        #[arg(is_flag)]
        #[check(exclusive)]
        skip: plap::Arg<LitBool>,

        #[arg(is_token_tree)]
        #[check(exclusive)]
        suffix: plap::Arg<Ident>,

        #[arg(is_token_tree)]
        vis: plap::Arg<Visibility>,

        #[arg(is_token_tree)]
        visibility: plap::Arg<Visibility>,
    }
);
