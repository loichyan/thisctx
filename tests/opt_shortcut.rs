use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(doc = "I'm on Line#2\n\n", derive(Clone))]
pub enum Error {
    #[thisctx(doc = "I'm on Line#1\n\n", derive(Copy))]
    ExtendAttributes(String),
    #[thisctx(doc = "I'm also on Line#1\n\n")]
    FieldAttributes(#[thisctx(doc = "I'm a field")] String),
}

fn requires_copied<T: Copy>(_: T) {}

#[test]
fn opt_attr() {
    requires_copied(ExtendAttributes("What's going on?"));
}
