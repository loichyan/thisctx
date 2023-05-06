use thisctx::WithContext;

#[derive(WithContext)]
enum Error {
    #[thisctx(no_skip)]
    #[thisctx(skip)]
    Variant1,
    #[thisctx(no_skip)]
    #[thisctx(no_skip)]
    Variant2,
}

fn main() {}
