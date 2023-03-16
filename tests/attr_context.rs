use thisctx::WithContext;

#[derive(WithContext)]
enum Error {
    GenerateContext,
    #[thisctx(skip)]
    NotGenerateContext,
}

#[test]
fn attr_context() {
    let _ = GenerateContextContext;
    let _ = Error::NotGenerateContext;
}
