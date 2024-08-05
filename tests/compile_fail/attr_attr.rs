#[derive(thisctx::WithContextNext)]
#[thisctx(attr = "derive(Debug)")]
pub enum Error {
    DebugDerived(String),
    #[thisctx(attr = "cfg(all())")]
    DebugNotDerived(String),
    #[thisctx(transparent, attr = "derive(Clone, Copy)")]
    CopyDerived(String),
}

fn requires_debug<T: std::fmt::Debug>(_: T) {}
fn requires_copied<T: Copy>(_: T) {}

fn main() {
    requires_debug(DebugDerived("with some generic magic"));
    requires_debug(DebugNotDerived("parent attributes are overridden"));
    requires_copied(CopyDerived);
}
