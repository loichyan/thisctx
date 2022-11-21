use thisctx::WithContext;
use thiserror::Error;

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Error, WithContext)]
#[error("{context_1}{source}{context_2}")]
struct NamedWithSource {
    context_1: String,
    source: BoxError,
    context_2: i32,
}

#[derive(Debug, Error, WithContext)]
#[error("{context_1}{original}{context_2}")]
struct NamedWithSourceAttr {
    context_1: String,
    #[source]
    original: BoxError,
    context_2: i32,
}

#[derive(Debug, Error, WithContext)]
#[error("{context_1}{context_2}")]
struct NamedWithoutSource {
    context_1: String,
    context_2: i32,
}

#[derive(Debug, Error, WithContext)]
#[error("")]
struct EmptyNamed {}

#[derive(Debug, Error, WithContext)]
#[error("{0}{1}{2}")]
struct UnnamedWithSource(String, #[source] BoxError, i32);

#[derive(Debug, Error, WithContext)]
#[error("{0}{1}")]
struct UnnamedWithoutSource(String, i32);

#[derive(Debug, Error, WithContext)]
#[error("")]
struct EmptyUnnamed();

#[derive(Debug, Error, WithContext)]
#[error("")]
struct Unit;

fn ok() -> Result<(), BoxError> {
    Ok(())
}

#[test]
fn derive_struct() {
    ok().context(NamedWithSourceContext {
        context_1: "",
        context_2: 0,
    })
    .unwrap();
    ok().context(NamedWithSourceAttrContext {
        context_1: "",
        context_2: 0,
    })
    .unwrap();
    true.context(NamedWithoutSourceContext {
        context_1: "",
        context_2: 0,
    })
    .unwrap();
    true.context(EmptyNamedContext).unwrap();
    ok().context(UnnamedWithSourceContext("", 0)).unwrap();
    true.context(UnnamedWithoutSourceContext("", 0)).unwrap();
    true.context(EmptyUnnamedContext).unwrap();
    true.context(UnitContext).unwrap();
}
