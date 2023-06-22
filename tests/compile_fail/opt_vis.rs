// Arbitray tokens in custom attributes are supported since Rust 1.34.
// https://blog.rust-lang.org/2019/04/11/Rust-1.34.0.html#custom-attributes-accept-arbitrary-token-streams
mod error {
    use thisctx::WithContext;

    #[derive(WithContext)]
    #[thisctx(vis = "pub(crate)")]
    pub enum Error {
        #[thisctx(vis = "pub")]
        PubVariant(i32),
        PubCrateVariant(i32),
        #[thisctx(vis = "")]
        PrivateVariant(i32),
        PrivateField(#[thisctx(vis = "")] i32),
    }
}

pub use error::PrivateVariant;
pub use error::PubCrateVariant;
pub use error::PubVariant;

fn any<T>() -> T {
    todo!()
}

fn main() {
    let t = any::<error::PrivateField>();
    t.0;
}
