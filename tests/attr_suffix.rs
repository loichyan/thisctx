use thisctx::{IntoError, WithContext};

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
    DefaultSuffixContext.build();
    NoSuffix.build();
    CustomSuffixThisctx.build();
}
