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
enum TransparentEnum {
    #[error("{0}")]
    #[thisctx(into(Error2))]
    Whatever(String),
}

#[test]
fn enum_into() {
    let _: Error = ().context(WhateverContext("whatever")).unwrap_err();
    let _: Error2 = ().context(WhateverContext("whatever")).unwrap_err();
}
