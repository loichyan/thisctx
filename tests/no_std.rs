#![no_std]

#[derive(thisctx::WithContext)]
enum Error {
    Variant1(i32),
    Variant2(usize),
}
