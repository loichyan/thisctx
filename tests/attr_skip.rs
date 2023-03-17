use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(skip)]
enum Error {
    #[thisctx(no_skip)]
    GenerateContext,
    NotGenerateContext,
}

#[test]
fn attr_skip() {
    let _ = GenerateContext;
    let _ = Error::NotGenerateContext;
}
