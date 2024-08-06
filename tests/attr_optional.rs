#![allow(dead_code)]

#[derive(thisctx::WithContext)]
pub enum Error {
    Optional(#[thisctx(optional = "path")] Option<String>),
    WithSource(
        #[source] String,
        #[thisctx(optional = "path")] Option<String>,
    ),
    WithFrom(
        #[thisctx(from)] String,
        #[thisctx(optional = "path")] Option<String>,
    ),
}

#[derive(thisctx::WithContext)]
#[thisctx(suffix = "Context")]
pub struct Struct {
    #[thisctx(from)]
    source: String,
    #[thisctx(optional)]
    path: Option<String>,
}
