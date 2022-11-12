use thisctx::WithContext;
use thiserror::Error;

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Error, WithContext)]
enum Error {
    #[error("{context_1}{source}{context_2}")]
    NamedWithSource {
        context_1: String,
        source: BoxError,
        context_2: i32,
    },
    #[error("{context_1}{original}{context_2}")]
    NamedWithSourceAttr {
        context_1: String,
        #[source]
        original: BoxError,
        context_2: i32,
    },
    #[error("{context_1}{context_2}")]
    NamedWithoutSource { context_1: String, context_2: i32 },
    #[error("")]
    EmptyNamed {},
    #[error("{0}{1}{2}")]
    UnnamedWithSource(String, #[source] BoxError, i32),
    #[error("{0}{1}")]
    UnnamedWithoutSource(String, i32),
    #[error("")]
    EmptyUnnamed(),
    #[error("")]
    Unit,
}

fn ok() -> Result<(), BoxError> {
    Ok(())
}

#[test]
fn with_context() {
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
    true.context(EmptyNamedContext {}).unwrap();
    ok().context(UnnamedWithSourceContext("", 0)).unwrap();
    true.context(UnnamedWithoutSourceContext("", 0)).unwrap();
    true.context(EmptyUnnamedContext()).unwrap();
    true.context(UnitContext).unwrap();
}
