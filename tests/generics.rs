use thisctx::WithContext;

#[derive(WithContext)]
enum EmptyGeneric<T1, T2> {
    Variant1(T1, T2),
    Variant2(T1, String, T2),
}

#[derive(WithContext)]
enum EmptyLifetime<'a, 'b> {
    Variant3(&'a str, &'b str),
    Variant4(&'a str, String, &'b str),
}

#[derive(WithContext)]
enum BoundedGeneric<T1: Into<String>, T2>
where
    T2: Into<String>,
    String: Into<String>,
{
    Variant5(T1, T2),
    Variant6(T1, String, T2),
}

#[derive(WithContext)]
enum BoundedLifetime<'a, 'b: 'a> {
    Variant7(&'a str, &'b str),
    Variant8(&'a str, String, &'b str),
}

#[derive(WithContext)]
enum UnusedLifetime<'a, 'b> {
    Variant9(&'a str),
    Variant10(&'b [u8]),
}

#[derive(WithContext)]
enum UnusedGeneric<T1, T2> {
    Variant11(T1),
    Variant12(T2),
}

#[derive(WithContext)]
enum ConstGeneric<const N1: usize, const N2: usize> {
    Variant13([String; N1]),
    Variant14([String; N2]),
}

#[derive(WithContext)]
struct GenricOrder<T1, T2>(T2, T1);

#[derive(WithContext)]
struct GenericDefault<T1, T2>(T1, String, T2);

#[test]
fn generic_order() {
    let _ = GenricOrderContext::<String, i32>(0, String::default());
    let _ = GenericDefaultContext::<i32, ()>(0, String::default(), ());
}
