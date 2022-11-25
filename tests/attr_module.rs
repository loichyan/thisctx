use thisctx::{IntoError, WithContext};

#[derive(WithContext)]
#[thisctx(module(context), suffix(false))]
pub(crate) enum Error {
    Variant1(String),
    Variant2(i32),
}

#[test]
fn attr_module() {
    let _: Error = context::Variant1("anyhow").build();
    let _: Error = context::Variant2(0).build();
}
