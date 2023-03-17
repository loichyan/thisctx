use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(generic(false))]
enum Error {
    #[thisctx(generic)]
    GeneratedGeneric(String),
    #[thisctx(generic)]
    NoGeneratedGenericOnField(String, #[thisctx(generic(false))] String, String),
    NoGeneratedGeneric(String),
}

#[test]
fn attr_no_generic() {
    let _ = NoGeneratedGenericOnField::<&str, &str>("What's", "going".to_owned(), "on");
}
