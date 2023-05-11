use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(no_generic)]
enum Error {
    #[thisctx(generic)]
    GeneratedGeneric(String),
    #[thisctx(generic)]
    NoGeneratedGenericOnField(String, #[thisctx(no_generic)] String, String),
    NoGeneratedGeneric(String),
}

#[test]
fn opt_generic() {
    let _ = NoGeneratedGenericOnField::<&str, &str>("What's", "going".to_owned(), "on");
}
