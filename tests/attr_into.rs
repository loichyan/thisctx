use thisctx::{IntoError, WithContext};
use thiserror::Error;

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    FromEnum(#[from] Enum),
    #[error(transparent)]
    FromStruct(#[from] Struct),
}

#[derive(Debug, Error)]
enum Error2 {
    #[error(transparent)]
    FromEnum(#[from] Enum),
    #[error(transparent)]
    FromStruct(#[from] Struct),
}

#[derive(Debug, Error, WithContext)]
#[thisctx(into(Error))]
enum Enum {
    #[error("{0}")]
    #[thisctx(into(Error2))]
    Variant1(String),
    #[error("{0}")]
    Variant2(String),
}

#[derive(Debug, Error, WithContext)]
#[thisctx(into(Error), into(Error2))]
#[error("{0}")]
struct Struct(String);

#[test]
fn attr_into() {
    let _: Error = Variant1("What").build();
    let _: Error2 = Variant1("is").build();
    let _: Error = Variant2("going").build();
    let _: Error = StructContext("on").build();
    let _: Error2 = StructContext("?").build();
}
