use thisctx::WithContext;

#[derive(WithContext)]
enum Error {
    GenerateContext,
    #[thisctx(skip)]
    NotGenerateContext,
}

#[test]
fn attr_context() {
    let _ = GenerateContext;
    let _ = Error::NotGenerateContext;
}
