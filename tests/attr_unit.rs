use thisctx::{IntoError, WithContext};

#[derive(Debug, WithContext)]
enum Error {
    #[thisctx(no_unit)]
    NotUnit(),
    Unit(),
}

#[test]
fn attr_unit() {
    NotUnit().build();
    Unit.build();
}
