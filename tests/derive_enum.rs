#[derive(Debug, Eq, PartialEq, thisctx::WithContext)]
enum Error {
    NamedWithSource {
        context_1: String,
        source: &'static str,
        context_2: i32,
    },
    NamedWithSourceAttr {
        context_1: String,
        #[source]
        original: &'static str,
        context_2: i32,
    },
    NamedWithoutSource {
        context_1: String,
        context_2: i32,
    },
    EmptyNamed {},
    UnnamedWithSource(String, #[source] &'static str, i32),
    UnnamedWithoutSource(String, i32),
    EmptyUnnamed(),
    Unit,
}

#[test]
fn derive_enum() {
    use thisctx::IntoError;
    assert_eq!(
        NamedWithSource {
            context_1: "Hello,",
            context_2: 233,
        }
        .into_error("world!"),
        Error::NamedWithSource {
            context_1: "Hello,".to_owned(),
            source: "world!",
            context_2: 233
        },
    );
    assert_eq!(
        NamedWithSourceAttr {
            context_1: "What's",
            context_2: 777,
        }
        .into_error("going on?"),
        Error::NamedWithSourceAttr {
            context_1: "What's".to_owned(),
            original: "going on?",
            context_2: 777
        },
    );
    assert_eq!(
        NamedWithoutSource {
            context_1: "whatever",
            context_2: 4399,
        }
        .build(),
        Error::NamedWithoutSource {
            context_1: "whatever".to_owned(),
            context_2: 4399
        },
    );
    assert_eq!(EmptyNamed.build(), Error::EmptyNamed {});
    assert_eq!(
        UnnamedWithSource("anyhow", 360).into_error("blah"),
        Error::UnnamedWithSource("anyhow".to_owned(), "blah", 360),
    );
    assert_eq!(
        UnnamedWithoutSource("failed", 1314).build(),
        Error::UnnamedWithoutSource("failed".to_owned(), 1314),
    );
    assert_eq!(EmptyUnnamed.build(), Error::EmptyUnnamed());
    assert_eq!(Unit.build(), Error::Unit);
}
