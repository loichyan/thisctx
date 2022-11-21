#![no_std]

use thisctx::WithContext;

#[derive(WithContext)]
enum Error {
    Variant1(i32),
    Variant2(usize),
}
