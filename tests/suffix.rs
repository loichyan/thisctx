use thisctx::WithContext;

#[derive(Debug, WithContext)]
#[thisctx(suffix(true))]
enum Error {
    DefaultSuffix,
    #[thisctx(suffix(false))]
    NoSuffix,
    #[thisctx(suffix(Thisctx))]
    CustomSuffix,
}

#[test]
fn use_context() {
    true.context(DefaultSuffixContext).unwrap();
    true.context(NoSuffix).unwrap();
    true.context(CustomSuffixThisctx).unwrap();
}
