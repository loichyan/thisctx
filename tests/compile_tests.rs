#![allow(dead_code)]

#[test]
fn compile_tests() {
    // compilation errors may vary by version
    if rustversion::cfg!(stable(1.56)) {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/compile_fail/*.rs");
    }
}
