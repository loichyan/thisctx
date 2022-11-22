use thisctx::WithContext;
use thiserror::Error;

#[derive(Debug, Eq, Error, PartialEq)]
#[error("{0}")]
struct HelloWorld(&'static str);

impl Default for HelloWorld {
    fn default() -> Self {
        Self("Hello, world!")
    }
}

#[derive(Debug, Eq, PartialEq, WithContext)]
enum Error {
    NoneSource(&'static str),
    SourceImplDefaut(#[source] HelloWorld),
}

#[test]
fn from_context() {
    assert_eq!(
        Error::from(NoneSourceContext("Hello, thisctx!")),
        Error::NoneSource("Hello, thisctx!"),
    );
    assert_eq!(
        Error::from(SourceImplDefautContext),
        Error::SourceImplDefaut(HelloWorld("Hello, world!")),
    );
}
