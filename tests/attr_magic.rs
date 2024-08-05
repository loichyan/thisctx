#![allow(dead_code)]

#[derive(thisctx::WithContext)]
#[thisctx(magic = false)]
enum Error {
    #[thisctx(magic)]
    GeneratedGeneric(String),
    #[thisctx(magic)]
    NoGeneratedGenericOnField(String, #[thisctx(magic = false)] String, String),
    NoGeneratedGeneric(String),
}

#[test]
fn attr_generic() {
    let _ = NoGeneratedGenericOnField::<&str, &str>("What's", "going".to_owned(), "on");
}
