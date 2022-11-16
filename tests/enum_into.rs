use thisctx::WithContext;
use thiserror::Error;

#[derive(Debug, Error, WithContext)]
enum Error {
    #[error(transparent)]
    Transparent(#[from] TransparentEnum),
}

#[derive(Debug, Error, WithContext)]
#[thisctx(into(Error))]
enum TransparentEnum {
    #[error("{0}")]
    Whatever(String),
}

fn requires_error(_: Error) {}

#[test]
fn with_context() {
    requires_error(().context(WhateverContext("whatever")).unwrap_err());
}
