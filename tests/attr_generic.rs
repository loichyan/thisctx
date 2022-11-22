use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(generic(false))]
enum Error {
    #[thisctx(generic(true))]
    GeneratedGeneric(String),
    #[thisctx(generic(true))]
    NoGeneratedGenericOnField(String, #[thisctx(generic(false))] String, String),
    NoGeneratedGeneric(String),
}

#[test]
fn attr_generic() {
    let _ = NoGeneratedGenericOnFieldContext::<&str, &str>("What's", "going".to_owned(), "on");
}
