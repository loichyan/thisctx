use plap::group;
use proc_macro2::TokenStream;
use syn::parse::ParseStream;
use syn::{Ident, LitBool, Type, Visibility};

pub(crate) fn parse_container(input: &syn::DeriveInput) -> syn::Result<Attrs> {
    let (mut c, args) = parse_args(&input.attrs)?;
    c.blocked_all(group![&args.optional, &args.source]);
    if matches!(input.data, syn::Data::Enum(_)) {
        c.blocked_all(group![&args.rename, &args.transparent]);
    } else {
        c.blocked(&args.skip);
    }
    build_attrs(&mut c, args)
}

pub(crate) fn parse_variant(input: &syn::Variant) -> syn::Result<Attrs> {
    let (mut c, args) = parse_args(&input.attrs)?;
    c.blocked_all(group![&args.module, &args.optional, &args.source]);
    build_attrs(&mut c, args)
}

pub(crate) fn parse_field(input: &syn::Field) -> syn::Result<Attrs> {
    let (mut c, args) = parse_args(&input.attrs)?;
    c.blocked_all(group![
        &args.module,
        &args.prefix,
        &args.remote,
        &args.skip,
        &args.suffix,
        &args.transparent,
        // field visibility are the same as its parent
        &args.vis,
        &args.visibility,
    ]);
    if input.ident.is_none() {
        for (key, value) in args.optional.keys().iter().zip(args.optional.values()) {
            if value.0.is_none() {
                c.with_error_at(
                    key.span(),
                    format!("`{}` requires a value on tuple fields", key),
                );
            }
        }
    }
    build_attrs(&mut c, args)
}

fn parse_args(attrs: &[syn::Attribute]) -> syn::Result<(plap::Checker, ThisctxArgs)> {
    let mut c = plap::Checker::default();
    let mut args = <ThisctxArgs as plap::Args>::init();
    for (name, attr) in attrs
        .iter()
        .flat_map(|a| a.path().get_ident().map(|i| (i, a)))
    {
        if name == "thisctx" {
            let r = attr.parse_args_with(|input: ParseStream| {
                plap::Parser::new(input).parse_all(&mut args)
            });
            c.with_source(name.span());
            c.with_result(r);
        } else if name == "error" {
            let t = attr.parse_args_with(|input: ParseStream| {
                syn::custom_keyword!(transparent);
                if input.peek(transparent) {
                    input.parse::<Ident>().map(Some)
                } else {
                    // ignore other values
                    while input.parse::<Option<proc_macro2::TokenTree>>()?.is_some() {}
                    Ok(None)
                }
            })?;
            if let Some(k) = t {
                let val = LitBool::new(true, k.span());
                args.transparent.add(k, val);
            }
        } else if name == "source" {
            let val = LitBool::new(true, name.span());
            args.source.add(name.clone(), val);
        }
    }
    Ok((c, args))
}

fn build_attrs(c: &mut plap::Checker, args: ThisctxArgs) -> syn::Result<Attrs> {
    plap::Args::check(&args, c);
    c.finish()?;
    let ThisctxArgs {
        attr,
        attribute,
        generic,
        module,
        optional,
        prefix,
        remote,
        rename,
        skip,
        source,
        suffix,
        transparent,
        vis,
        visibility,
    } = args;
    Ok(Attrs {
        attr: attr
            .take_any()
            .into_iter()
            .chain(attribute.take_any())
            .collect(),
        generic: generic.take_last().map(|t| t.value()),
        module: module.take_last(),
        optional: optional.take_last().map(|t| t.0),
        prefix: prefix.take_last(),
        remote: remote.take_last(),
        rename: rename.take_last(),
        skip: skip.take_last().map(|t| t.value()),
        source: source.take_flag(),
        suffix: suffix.take_last(),
        transparent: transparent.take_flag(),
        vis: vis.take_last().or_else(|| visibility.take_last()),
    })
}

pub(crate) struct Attrs {
    // field, struct, variant -> enum
    pub attr: Vec<TokenStream>,
    // field -> struct, field -> variant -> enum
    pub generic: Option<bool>,
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

plap::define_args!(
    #[check(exclusive_aliases = [vis, visibility])]
    struct ThisctxArgs {
        #[arg(is_token_tree)]
        attr: plap::Arg<TokenStream>,

        #[arg(is_token_tree)]
        attribute: plap::Arg<TokenStream>,

        #[arg(is_flag)]
        #[check(exclusive, conflicts_with_any = [optional, source, transparent])]
        generic: plap::Arg<LitBool>,

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
        #[check(exclusive, conflicts_with_any = [prefix, suffix])]
        rename: plap::Arg<Ident>,

        #[arg(is_flag)]
        #[check(exclusive)]
        skip: plap::Arg<LitBool>,

        #[arg(is_flag)]
        #[check(exclusive)]
        source: plap::Arg<LitBool>,

        #[arg(is_token_tree)]
        #[check(exclusive)]
        suffix: plap::Arg<Ident>,

        #[arg(is_flag)]
        #[check(exclusive)]
        transparent: plap::Arg<LitBool>,

        #[arg(is_token_tree)]
        vis: plap::Arg<Visibility>,

        #[arg(is_token_tree)]
        visibility: plap::Arg<Visibility>,
    }
);
