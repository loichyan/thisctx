use thisctx::IntoError;

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
fn visibility() {
    error::PubVariantContext(0).build();
    error::PubCrateVariantContext(0).build();
}
