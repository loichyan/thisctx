use thisctx::WithContext;

#[derive(Debug, WithContext)]
enum Error {
    ErrorFromContext(String),
    IntoError(#[source] String),
}

fn requires_error(_: Error) {}

#[test]
fn into_error() {
    requires_error(ErrorFromContextContext("").into());
    requires_error(ErrorFromContextContext("").into_error(()));
    requires_error(IntoErrorContext.into_error("".to_owned()));
}
