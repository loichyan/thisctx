#![allow(unused_imports)]

// Arbitrary tokens in custom attributes are supported since Rust 1.34.
// https://blog.rust-lang.org/2019/04/11/Rust-1.34.0.html#custom-attributes-accept-arbitrary-token-streams
mod error {
    #[derive(thisctx::WithContextNext)]
    pub enum Error {
        #[thisctx(vis = "pub")]
        Pub(i32),
        #[thisctx(vis = "pub(crate)")]
        PubCrate(i32),
        #[thisctx(vis = "pub(super)")]
        PubSuper(i32),
        #[thisctx(vis = "")]
        Private(i32),
    }
}

#[rustfmt::skip]
pub use error::Pub as _;
#[rustfmt::skip]
pub use error::PubCrate as _;

#[rustfmt::skip]
pub(crate) use error::PubCrate as _;
#[rustfmt::skip]
pub(crate) use error::PubSuper as _;

#[rustfmt::skip]
use error::PubSuper as _;
#[rustfmt::skip]
use error::Private as _;

fn main() {
    let _ = error::Pub(0);
    let _ = error::PubCrate(0);
    let _ = error::PubSuper(0);
    let _ = error::Private(0);
}
