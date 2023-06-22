// Const generics are supported since Rust 1.51.
// https://blog.rust-lang.org/2021/03/25/Rust-1.51.0.html#const-generics-mvp

use thisctx::WithContext;

#[derive(WithContext)]
enum ConstGeneric<const N1: usize, const N2: usize> {
    Variant13([String; N1]),
    Variant14([String; N2]),
}

fn main() {}
