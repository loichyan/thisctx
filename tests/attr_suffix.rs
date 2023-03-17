use thisctx::{IntoError, WithContext};

#[derive(Debug, WithContext)]
#[allow(clippy::enum_variant_names)]
enum Error {
    #[thisctx(suffix)]
    DefaultSuffix,
    NoSuffix,
    #[thisctx(suffix(Thisctx))]
    CustomSuffix,
}

#[test]
fn attr_suffix() {
    DefaultSuffixContext.build();
    NoSuffix.build();
    CustomSuffixThisctx.build();
}
