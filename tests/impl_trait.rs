use thisctx::{IntoError, WithContext};

#[derive(Debug, Eq, PartialEq)]
struct HelloWorld(&'static str);

impl Default for HelloWorld {
    fn default() -> Self {
        Self("Hello, world!")
    }
}

#[derive(Debug, Eq, PartialEq, WithContext)]
enum Error {
    NoneSource(&'static str),
}

// #[test]
// fn from_context() {
//     assert_eq!(
//         Error::from(NoneSource("Hello, thisctx!")),
//         Error::NoneSource("Hello, thisctx!"),
//     );
// }

#[test]
fn into_error() {
    assert_eq!(
        NoneSource("Hello, world!").build(),
        Error::NoneSource("Hello, world!"),
    );
    assert_eq!(
        NoneSource("Hello, world!").fail::<()>(),
        Err(Error::NoneSource("Hello, world!")),
    );
}
