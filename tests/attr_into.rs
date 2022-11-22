use thisctx::{IntoError, WithContext};
use thiserror::Error;

#[derive(Debug, Error, WithContext)]
enum Error {
    #[error(transparent)]
    FromEnum(#[from] Enum),
    #[error(transparent)]
    FromStruct(#[from] Struct),
}

#[derive(Debug, Error, WithContext)]
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
    #[thisctx(into(Error2))]
    Variant2(String),
}

#[derive(Debug, Error, WithContext)]
#[thisctx(into(Error))]
#[thisctx(into(Error2))]
#[error("{0}")]
struct Struct(String);

#[test]
fn enum_into() {
    let _: Error = Variant1Context("What").build();
    let _: Error2 = Variant1Context("is").build();
    let _: Error = Variant2Context("going").build();
    let _: Error2 = Variant2Context("on").build();
    let _: Error = StructContext("?").build();
    let _: Error2 = StructContext("!").build();
}
