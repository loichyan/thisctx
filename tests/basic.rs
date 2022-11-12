use thisctx::WithContext;
use thiserror::Error;

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Error, WithContext)]
pub enum ErrorEnum {
    #[error("{context_1}{source}{context_2}")]
    NamedWithSource {
        context_1: String,
        source: BoxError,
        context_2: String,
    },
    #[error("{context_1}{original}{context_2}")]
    NamedWithSourceAttr {
        context_1: String,
        #[source]
        original: BoxError,
        context_2: String,
    },
    #[error("{context_1}{context_2}")]
    NamedWithoutSource {
        context_1: String,
        context_2: String,
    },
    #[error("")]
    EmptyNamed {},
    #[error("{0}{1}{2}")]
    UnnamedWithSource(String, #[source] BoxError, String),
    #[error("{0}{1}")]
    UnnamedWithoutSource(String, String),
    #[error("")]
    EmptyUnnamed(),
    #[error("")]
    Unit,
}

#[derive(Debug, Error, WithContext)]
#[error("{context_1}{source}{context_2}")]
pub struct ErrorStruct {
    context_1: String,
    source: BoxError,
    context_2: String,
}
