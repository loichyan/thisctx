#[derive(thisctx::WithContext)]
#[thisctx(module = "context")]
pub(crate) enum Error {
    Variant1(String),
    Variant2(i32),
}

#[test]
fn attr_module() {
    let _ = context::Variant1("anyhow");
    let _ = context::Variant2(0);
}
