#[derive(thisctx::WithContextNext)]
#[thisctx(skip)]
enum Error {
    #[thisctx(skip = false)]
    GenerateContext,
    NotGenerateContext,
}

fn main() {
    let _ = GenerateContext;
    let _ = NotGenerateContext;
}
