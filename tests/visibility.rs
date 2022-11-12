use thisctx::WithContext;

mod error {
    use thisctx::WithContext;

    #[derive(Debug, WithContext)]
    #[thisctx(visibility(pub(crate)))]
    pub enum Error {
        #[thisctx(visibility(pub))]
        PubVariant(#[thisctx(visibility(pub(crate)))] i32),
        PubCrateVariant(i32),
    }
}

#[test]
fn with_context() {
    true.context(error::PubVariantContext(0)).unwrap();
    true.context(error::PubCrateVariantContext(0)).unwrap();
}
