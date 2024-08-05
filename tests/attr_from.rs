struct Remote;

#[derive(thisctx::WithContext)]
pub(crate) enum Error {
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
