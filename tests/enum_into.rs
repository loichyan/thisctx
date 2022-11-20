use thisctx::WithContext;
use thiserror::Error;

#[derive(Debug, Error, WithContext)]
enum Error {
    #[error(transparent)]
    Transparent(#[from] TransparentEnum),
}

#[derive(Debug, Error, WithContext)]
enum Error2 {
    #[error(transparent)]
    Transparent2(#[from] TransparentEnum),
}

#[derive(Debug, Error, WithContext)]
#[thisctx(into(Error))]
#[thisctx(into(Error2))]
enum TransparentEnum {
    #[error("{0}")]
    Whatever(String),
}

#[test]
fn with_context() {
    let _: Error = ().context(WhateverContext("whatever")).unwrap_err();
    let _: Error2 = ().context(WhateverContext("whatever")).unwrap_err();
}
