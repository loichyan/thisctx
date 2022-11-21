use thisctx::WithContext;

#[derive(WithContext)]
#[thisctx(attr(doc = "I'm on Line#2\n\n"))]
#[thisctx(attr(derive(Clone)))]
pub enum Error {
    #[thisctx(attr(doc = "I'm on Line#1\n\n"))]
    #[thisctx(attr(derive(Copy)))]
    ExtendAttributes(String),
    #[thisctx(attr(doc = "I'm also on Line#1\n\n"))]
    FieldAttributes(#[thisctx(attr(doc = "I'm a field"))] String),
}

fn requires_copied<T: Copy>(_: T) {}

#[test]
fn attr() {
    let ctx = ExtendAttributesContext("What's going on?");
    requires_copied(ctx);
}
