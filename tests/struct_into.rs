use thisctx::WithContext;
use thiserror::Error;

#[derive(Debug, Error, WithContext)]
enum Error {
    #[error(transparent)]
    Transparent(#[from] TransparentStruct),
}

#[derive(Debug, Error, WithContext)]
enum Error2 {
    #[error(transparent)]
    Transparent2(#[from] TransparentStruct),
}

#[derive(Debug, Error, WithContext)]
#[thisctx(into(Error))]
#[thisctx(into(Error2))]
#[error("{reason}")]
struct TransparentStruct {
    reason: String,
}

#[test]
fn with_context() {
    let _: Error = ().context(TransparentStructContext { reason: "whatever" }).unwrap_err();
    let _: Error2 = ().context(TransparentStructContext { reason: "whatever" }).unwrap_err();
}
