#![allow(dead_code)]

#[derive(Debug, thisctx::WithContext)]
#[allow(clippy::enum_variant_names)]
enum Error {
    #[thisctx(prefix = "Error")]
    WithPrefix,
    #[thisctx(suffix = "Context")]
    WithSuffix,
    #[thisctx(rename = "ErrorWithRenameContext")]
    WithRename,
}

#[test]
fn attr_suffix() {
    let _ = ErrorWithPrefix;
    let _ = WithSuffixContext;
    let _ = ErrorWithRenameContext;
}
