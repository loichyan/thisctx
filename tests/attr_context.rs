use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(context(false))]
enum Error {
    #[thisctx(context(true))]
    GenerateContext,
    NotGenerateContext,
}

#[test]
fn attr_context() {
    let _ = GenerateContextContext;
    let _ = Error::NotGenerateContext;
}
