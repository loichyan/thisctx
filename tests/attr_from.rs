#![allow(dead_code)]

struct Remote;

#[derive(thisctx::WithContext)]
enum Error {
    #[thisctx(remote = "Remote")]
    Variant1(#[thisctx(from)] String),
    Variant2(#[thisctx(from)] i32),
}

impl From<Error> for Remote {
    fn from(_: Error) -> Self {
        todo!()
    }
}

#[test]
fn attr_module() {}
