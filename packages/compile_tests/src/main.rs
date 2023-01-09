// `trybuild` requires `rustc v1.45+`
fn main() {
    let t = trybuild::TestCases::new();
    t.compile_fail("compile_tests/*.rs");

    if rustversion::cfg!(since(1.34)) {
        t.pass("compatibility_tests/keywords_visibility.rs");
    }

    if rustversion::cfg!(since(1.51)) {
        t.pass("compatibility_tests/const_generics.rs");
    }
}
