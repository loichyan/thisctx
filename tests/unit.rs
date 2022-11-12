use thisctx::WithContext;

#[derive(Debug, WithContext)]
#[thisctx(unit(true))]
enum Error {
    #[thisctx(unit(false))]
    NotUnit(),
    Unit(),
}

#[test]
fn with_context() {
    true.context(NotUnitContext()).unwrap();
    true.context(UnitContext).unwrap();
}
