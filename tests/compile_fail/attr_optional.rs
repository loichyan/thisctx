#![allow(dead_code)]

#[derive(thisctx::WithContext)]
enum Error {
    Optional(#[thisctx(optional = "path")] Option<String>),
    WithSource(
        #[source] String,
        #[thisctx(optional = "path")] Option<String>,
    ),
    WithFrom(
        #[thisctx(from)] String,
        #[thisctx(optional = "path")] Option<String>,
    ),
    WithConflict(
        #[thisctx(from)] String,
        #[thisctx(optional = "path")] Option<String>,
        #[thisctx(optional = "path")] Option<usize>,
    ),
}

#[derive(thisctx::WithContext)]
#[thisctx(suffix = "Context")]
struct Struct {
    #[thisctx(from)]
    source: String,
    #[thisctx(optional)]
    path: Option<String>,
    #[thisctx(optional = "path")]
    path2: Option<usize>,
}

#[derive(thisctx::WithContext)]
#[thisctx(suffix = "Context")]
struct MultipleOptionalFields {
    #[thisctx(from)]
    source: String,
    #[thisctx(optional)]
    path: Option<String>,
    #[thisctx(optional)]
    path2: Option<usize>,
}

fn main() {}
