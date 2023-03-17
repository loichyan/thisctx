use thisctx::{IntoError, WithContext};

#[derive(WithContext)]
pub struct UnwrapOption(Option<String>);

#[derive(WithContext)]
#[thisctx(no_unwrap)]
pub struct NotUnwrapOption(Option<String>);

fn main() {
    UnwrapOptionContext(Some("optional")).build();
    NotUnwrapOptionContext(Some("optional".to_owned())).build();
    NotUnwrapOptionContext(Some("optional")).build();
}
