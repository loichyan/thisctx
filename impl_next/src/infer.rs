use proc_macro2::Ident;
use syn::Type;

macro_rules! matches_any {
    ($ident:expr $(,$value:expr)* $(,)?) => {{
        let ident = $ident;
        false $(|| ident == $value)*
    }};
}

pub(crate) fn is_in_generic_whitelist(ty: &Type) -> bool {
    if let Some(name) = infer_std(ty) {
        matches_any!(name, "String", "PathBuf", "Vec", "Box", "Arc", "OsString", "CString", "Rc")
    } else {
        false
    }
}

/// Infers a potential `std` type.
fn infer_std(ty: &Type) -> Option<&Ident> {
    let ty = if let Type::Path(p) = ty {
        p
    } else {
        return None;
    };
    if ty.qself.is_some() {
        return None;
    }

    let segments = &ty.path.segments;
    let segment = match segments.len() {
        // a single identifier
        1 if ty.path.leading_colon.is_none() => &segments[0],
        // or a full qualified type
        2.. if matches_any!(&segments[0].ident, "std", "alloc", "core") => segments.last().unwrap(),
        _ => return None,
    };

    Some(&segment.ident)
}

#[cfg(test)]
mod tests {
    use syn::Type;

    fn test_infer_std(input: &str, expected: &str) {
        println!("infer: {}", input);
        let ty: Type = syn::parse_str(input).unwrap();
        let name = super::infer_std(&ty).unwrap_or_else(|| panic!("expected {}", expected));
        if name != expected {
            panic!("{} != {}", name, expected);
        }
    }

    fn test_in_generic_whitelist(input: &str) {
        let ty: Type = syn::parse_str(input).unwrap();
        if !super::is_in_generic_whitelist(&ty) {
            panic!("{} is not in the generic whitelist", input);
        }
    }

    fn test_not_in_generic_whitelist(input: &str) {
        let ty: Type = syn::parse_str(input).unwrap();
        if super::is_in_generic_whitelist(&ty) {
            panic!("{} is in the generic whitelist", input);
        }
    }

    #[test]
    fn infer_std() {
        test_infer_std("String", "String");
        test_infer_std("String<>", "String");
        test_infer_std("String::<>", "String");

        test_infer_std("Vec<String>", "Vec");
        test_infer_std("Vec<String<>>", "Vec");
        test_infer_std("Vec::<String::<>>", "Vec");
    }

    #[test]
    fn infer_std_qualified() {
        test_infer_std("std::boxed::Box<str>", "Box");
        test_infer_std("::std::rc::Rc<str>", "Rc");

        test_infer_std("::alloc::vec::Vec<i32>", "Vec");
        test_infer_std("core::whatever::Box<str>", "Box");
    }

    #[test]
    #[should_panic = "expected String"]
    fn infer_std_with_leading_colon() {
        test_infer_std("::String", "String");
    }

    #[test]
    #[should_panic = "String != string"]
    fn infer_std_mismatched() {
        test_infer_std("String", "string")
    }

    #[test]
    fn in_generic_whitelist() {
        test_in_generic_whitelist("String");
        test_in_generic_whitelist("String<>");
        test_in_generic_whitelist("String::<>");

        test_in_generic_whitelist("std::path::PathBuf");
        test_in_generic_whitelist("std::ffi::OsString");
        test_in_generic_whitelist("std::ffi::OsString");

        test_in_generic_whitelist("CString");
        test_in_generic_whitelist("CString<Allocator>");
        test_in_generic_whitelist("CString::<Allocator>");

        test_in_generic_whitelist("Vec<i32>");
        test_in_generic_whitelist("Box<str, Allocator>");
        test_in_generic_whitelist("std::sync::Arc<std::path::Path>");
        test_in_generic_whitelist("::std::rc::Rc<std::ffi::CStr, Allocator>");

        // this eventually causes a compilation error
        test_in_generic_whitelist("std::wrong::path::Arc");
    }

    #[test]
    fn not_in_generic_whitelist() {
        test_not_in_generic_whitelist("::String");
        test_not_in_generic_whitelist("some::magical::path::Arc");
    }
}
