// Arbitray tokens in custom attributes are supported since Rust 1.34.
// https://blog.rust-lang.org/2019/04/11/Rust-1.34.0.html#custom-attributes-accept-arbitrary-token-streams

use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(visibility(pub(crate)))]
pub struct UseKeywords;

fn main() {}
