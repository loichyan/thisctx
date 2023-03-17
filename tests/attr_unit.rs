use thisctx::{IntoError, WithContext};

#[derive(Debug, WithContext)]
#[thisctx(no_unit)]
enum Error {
    NotUnit(),
    #[thisctx(unit)]
    Unit(),
}

#[test]
fn attr_no_unit() {
    NotUnit().build();
    Unit.build();
}
