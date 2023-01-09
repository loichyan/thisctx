// Arbitray tokens in custom attributes are supported since Rust 1.34.
// https://blog.rust-lang.org/2019/04/11/Rust-1.34.0.html#custom-attributes-accept-arbitrary-token-streams
mod error {
    use thisctx::WithContext;

    #[derive(WithContext)]
    #[thisctx(visibility = "pub(crate)")]
    pub enum Error {
        #[thisctx(visibility = "pub")]
        PubVariant(i32),
        PubCrateVariant(i32),
        #[thisctx(visibility = "")]
        PrivateVariant(i32),
        PrivateField(#[thisctx(visibility = "")] i32),
    }
}

pub use error::PrivateVariantContext;
pub use error::PubCrateVariantContext;
pub use error::PubVariantContext;

fn any<T>() -> T {
    todo!()
}

fn main() {
    let t = any::<error::PrivateFieldContext>();
    t.0;
}
