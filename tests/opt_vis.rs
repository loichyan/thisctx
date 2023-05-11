use thisctx::IntoError;

mod error {
    use thisctx::WithContext;

    #[derive(Debug, WithContext)]
    #[thisctx(vis(pub(crate)))]
    pub enum Error {
        #[thisctx(vis(pub))]
        PubVariant(#[thisctx(vis(pub(crate)))] i32),
        PubCrateVariant(i32),
    }
}

#[test]
fn opt_vis() {
    error::PubVariant(0).build();
    error::PubCrateVariant(0).build();
}
