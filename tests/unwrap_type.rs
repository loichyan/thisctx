use thisctx::{IntoError, WithContext};

#[derive(WithContext)]
pub struct UnwrapOption(Option<String>);

#[derive(WithContext)]
#[thisctx(no_unwrap)]
pub struct NotUnwrapOption(Option<String>);

#[test]
fn optional() {
    UnwrapOptionContext(Some("optional")).build();
    NotUnwrapOptionContext(Some("optional".to_owned())).build();
}
