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
fn generic() {
    let _ = NoGeneratedGenericOnFieldContext::<&str, &str>("what's", "going".to_string(), "on");
}
