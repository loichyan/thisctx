#![allow(dead_code)]

#[derive(thisctx::WithContext)]
#[thisctx(module = "context")]
pub enum Error {
    Variant1(String),
    Variant2(i32),
}

#[test]
fn attr_module() {
    let _ = context::Variant1("anyhow");
    let _ = context::Variant2(0);
}
