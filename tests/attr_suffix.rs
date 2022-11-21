use thisctx::WithContext;

#[derive(Debug, WithContext)]
#[thisctx(suffix(true))]
#[allow(clippy::enum_variant_names)]
enum Error {
    DefaultSuffix,
    #[thisctx(suffix(false))]
    NoSuffix,
    #[thisctx(suffix(Thisctx))]
    CustomSuffix,
}

#[test]
fn suffix() {
    true.context(DefaultSuffixContext).unwrap();
    true.context(NoSuffix).unwrap();
    true.context(CustomSuffixThisctx).unwrap();
}
