use thisctx::{IntoError, WithContext};

#[derive(Debug, WithContext)]
#[thisctx(unit(false))]
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
