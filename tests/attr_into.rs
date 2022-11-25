use thisctx::{IntoError, WithContext};

#[derive(Debug, WithContext)]
enum Error {
    #[error(transparent)]
    FromEnum(Enum),
    #[error(transparent)]
    FromStruct(Struct),
}

impl From<Enum> for Error {
    fn from(t: Enum) -> Self {
        Error::FromEnum(t)
    }
}

impl From<Struct> for Error {
    fn from(t: Struct) -> Self {
        Error::FromStruct(t)
    }
}

#[derive(Debug, WithContext)]
enum Error2 {
    #[error(transparent)]
    FromEnum(Enum),
    #[error(transparent)]
    FromStruct(Struct),
}

impl From<Enum> for Error2 {
    fn from(t: Enum) -> Self {
        Error2::FromEnum(t)
    }
}

impl From<Struct> for Error2 {
    fn from(t: Struct) -> Self {
        Error2::FromStruct(t)
    }
}

#[derive(Debug, WithContext)]
#[thisctx(into(Error))]
enum Enum {
    #[error("{0}")]
    #[thisctx(into(Error2))]
    Variant1(String),
    #[error("{0}")]
    Variant2(String),
}

#[derive(Debug, WithContext)]
#[thisctx(into(Error), into(Error2))]
#[error("{0}")]
struct Struct(String);

#[test]
fn attr_into() {
    let _: Error = Variant1Context("What").build();
    let _: Error2 = Variant1Context("is").build();
    let _: Error = Variant2Context("going").build();
    let _: Error = StructContext("on").build();
    let _: Error2 = StructContext("?").build();
}
