use thisctx::{IntoError, WithContext};

#[derive(Debug, Eq, PartialEq, WithContext)]
struct NamedWithSource {
    context_1: String,
    source: &'static str,
    context_2: i32,
}

#[derive(Debug, Eq, PartialEq, WithContext)]
struct NamedWithSourceAttr {
    context_1: String,
    #[source]
    original: &'static str,
    context_2: i32,
}

#[derive(Debug, Eq, PartialEq, WithContext)]
struct NamedWithoutSource {
    context_1: String,
    context_2: i32,
}

#[derive(Debug, Eq, PartialEq, WithContext)]
struct EmptyNamed {}

#[derive(Debug, Eq, PartialEq, WithContext)]
struct UnnamedWithSource(String, #[source] &'static str, i32);

#[derive(Debug, Eq, PartialEq, WithContext)]
struct UnnamedWithoutSource(String, i32);

#[derive(Debug, Eq, PartialEq, WithContext)]
struct EmptyUnnamed();

#[derive(Debug, Eq, PartialEq, WithContext)]
struct Unit;

#[test]
fn derive_enum() {
    assert_eq!(
        NamedWithSourceContext {
            context_1: "Hello,",
            context_2: 233,
        }
        .into_error("world!"),
        NamedWithSource {
            context_1: "Hello,".to_owned(),
            source: "world!",
            context_2: 233
        },
    );
    assert_eq!(
        NamedWithSourceAttrContext {
            context_1: "What's",
            context_2: 777,
        }
        .into_error("going on?"),
        NamedWithSourceAttr {
            context_1: "What's".to_owned(),
            original: "going on?",
            context_2: 777
        },
    );
    assert_eq!(
        NamedWithoutSourceContext {
            context_1: "whatever",
            context_2: 4399,
        }
        .build(),
        NamedWithoutSource {
            context_1: "whatever".to_owned(),
            context_2: 4399
        },
    );
    assert_eq!(EmptyNamedContext.build(), EmptyNamed {});
    assert_eq!(
        UnnamedWithSourceContext("anyhow", 360).into_error("blah"),
        UnnamedWithSource("anyhow".to_owned(), "blah", 360),
    );
    assert_eq!(
        UnnamedWithoutSourceContext("failed", 1314).build(),
        UnnamedWithoutSource("failed".to_owned(), 1314),
    );
    assert_eq!(EmptyUnnamedContext.build(), EmptyUnnamed());
    assert_eq!(UnitContext.build(), Unit);
}
