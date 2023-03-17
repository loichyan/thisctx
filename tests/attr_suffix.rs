use thisctx::{IntoError, WithContext};

#[derive(Debug, WithContext)]
#[allow(clippy::enum_variant_names)]
#[thisctx(suffix)]
enum Error {
    DefaultSuffix,
    #[thisctx(no_suffix)]
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
