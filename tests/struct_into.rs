use thisctx::WithContext;
use thiserror::Error;

#[derive(Debug, Error, WithContext)]
enum Error {
    #[error(transparent)]
    Transparent(#[from] TransparentStruct),
}

#[derive(Debug, Error, WithContext)]
#[thisctx(into(Error))]
#[error("{reason}")]
struct TransparentStruct {
    reason: String,
}

fn requires_error(_: Error) {}

#[test]
fn with_context() {
    requires_error(
        ().context(TransparentStructContext { reason: "whatever" })
            .unwrap_err(),
    );
}
