use thisctx::WithContext;

#[derive(WithContext)]
enum Error {
    GeneratedGeneric(String),
    NoGeneratedGenericOnField(String, #[thisctx(no_generic)] String, String),
    #[thisctx(no_generic)]
    NoGeneratedGeneric(String),
}

#[test]
fn attr_generic() {
    let _ = NoGeneratedGenericOnFieldContext::<&str, &str>("What's", "going".to_owned(), "on");
}
