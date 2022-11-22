use thisctx::{IntoError, WithContext};

#[derive(Debug, WithContext)]
#[thisctx(unit(true))]
enum Error {
    #[thisctx(unit(false))]
    NotUnit(),
    Unit(),
}

#[test]
fn unit() {
    NotUnitContext().build();
    UnitContext.build();
}
