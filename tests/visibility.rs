use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(visibility(pub(crate)))]
pub enum Error {
    #[thisctx(visibility(pub))]
    PubVariant,
    PubCrateVariant,
}
