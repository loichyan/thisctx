use std::marker::PhantomData;

use thisctx::WithContextNext;

#[derive(WithContextNext)]
#[thisctx(suffix = "Context", generic)]
pub struct MyStruct {
    field1: String,
    field2: usize,
}

#[derive(WithContextNext)]
#[thisctx(generic)]
pub enum MyEnum {
    Variant1(String, i32),
    Variant2(usize, Vec<u32>),
    Variant3 {
        source: std::io::Error,
        #[source]
        source_attr: std::str::Utf8Error,
    },
    #[error(transparent)]
    Variant4(String),
    #[thisctx(rename = "MyEnumContext")]
    MyEnum,
}

impl From<MyEnum> for MyStruct {
    fn from(_: MyEnum) -> Self {
        todo!()
    }
}

#[derive(WithContextNext)]
#[thisctx(suffix = "Context")]
pub struct MyError<'a, T: std::fmt::Display, const N: usize>(
    #[source] std::io::Error,
    #[thisctx(generic)] std::str::Utf8Error,
    &'a [T; N],
)
where
    T: std::fmt::Debug;

#[derive(WithContextNext)]
pub enum MyEnumWithGenerics<'a, T: std::fmt::Display, const N: usize>
where
    T: std::fmt::Debug,
{
    V1(
        #[source] std::io::Error,
        #[thisctx(generic)] std::str::Utf8Error,
        &'a [T; N],
        #[thisctx(optional = "path")] Option<String>,
    ),
    V2(
        #[source] std::io::Error,
        #[thisctx(generic)] std::str::Utf8Error,
        PhantomData<&'a [T; N]>,
        #[thisctx(optional = "path")] Option<String>,
    ),
}

#[derive(WithContextNext)]
#[thisctx(suffix = "Context")]
pub struct MyErrorWithOptional<T, const N: usize> {
    #[thisctx(generic)]
    reason: String,
    #[thisctx(optional)]
    path: Option<String>,
    elems: [T; N],
}

#[derive(WithContextNext)]
#[thisctx(suffix = "Context")]
pub struct MyTransparentError(String);
