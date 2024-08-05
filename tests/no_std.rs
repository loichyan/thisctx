#![no_std]

#[derive(thisctx::WithContextNext)]
enum Error {
    Variant1(i32),
    Variant2(usize),
}
