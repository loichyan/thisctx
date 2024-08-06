use proc_macro2::Ident;
use syn::{GenericArgument, PathArguments, Type};

macro_rules! matches_any {
    ($ident:expr $(,$value:expr)* $(,)?) => {{
        let ident = $ident;
        false $(|| ident == $value)*
    }};
}

/// Returns the first generic argument of an optional type.
pub(crate) fn get_optional_inner(ty: &Type) -> Option<&Type> {
    let (_, args) = infer_std(ty)?;
    if let PathArguments::AngleBracketed(args) = &args {
        args.args.first().and_then(|args| {
            if let GenericArgument::Type(inner) = args {
                Some(inner)
            } else {
                None
            }
        })
    } else {
        None
    }
}

pub(crate) fn is_in_magic_whitelist(ty: &Type) -> bool {
    if let Some((name, _)) = infer_std(ty) {
        matches_any!(name, "String", "PathBuf", "Vec", "Box", "Arc", "OsString", "CString", "Rc")
    } else {
        false
    }
}

/// Infers a potential `std` type.
fn infer_std(ty: &Type) -> Option<(&Ident, &PathArguments)> {
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
        2.. if matches_any!(&segments[0].ident, "std", "core", "alloc") => segments.last().unwrap(),
        _ => return None,
    };
    Some((&segment.ident, &segment.arguments))
}

#[cfg(test)]
mod tests {
    use syn::Type;

    fn test_input_with(input: &str, f: impl FnOnce(&Type)) {
        println!("test: {}", input);
        let ty: Type = syn::parse_str(input).unwrap();
        f(&ty);
    }

    fn test_infer_std(input: &str, expected: &str) {
        test_input_with(input, |ty| {
            let (name, _) = super::infer_std(ty).expect("failed to infer name");
            if name != expected {
                panic!("{} != {}", name, expected);
            }
        });
    }

    fn test_in_magic_whitelist(input: &str) {
        test_input_with(input, |ty| {
            if !super::is_in_magic_whitelist(ty) {
                panic!("{} is not in the magic whitelist", input);
            }
        });
    }

    fn test_not_in_magic_whitelist(input: &str) {
        test_input_with(input, |ty| {
            if super::is_in_magic_whitelist(ty) {
                panic!("{} is in the magic whitelist", input);
            }
        });
    }

    fn test_optional_inner_type(input: &str, expected: &str) {
        test_input_with(input, |ty| {
            let inner = super::get_optional_inner(ty).expect("failed to infer inner type");
            let (inner_name, _) = super::infer_std(inner).expect("failed to infer inner name");
            if inner_name != expected {
                panic!("{} != {}", inner_name, expected);
            }
        });
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
    #[should_panic = "failed to infer name"]
    fn infer_std_with_leading_colon() {
        test_infer_std("::String", "String");
    }

    #[test]
    #[should_panic = "String != string"]
    fn infer_std_mismatched() {
        test_infer_std("String", "string")
    }

    #[test]
    fn in_magic_whitelist() {
        test_in_magic_whitelist("String");
        test_in_magic_whitelist("String<>");
        test_in_magic_whitelist("String::<>");

        test_in_magic_whitelist("std::path::PathBuf");
        test_in_magic_whitelist("std::ffi::OsString");
        test_in_magic_whitelist("std::ffi::OsString");

        test_in_magic_whitelist("CString");
        test_in_magic_whitelist("CString<Allocator>");
        test_in_magic_whitelist("CString::<Allocator>");

        test_in_magic_whitelist("Vec<i32>");
        test_in_magic_whitelist("Box<str, Allocator>");
        test_in_magic_whitelist("std::sync::Arc<std::path::Path>");
        test_in_magic_whitelist("::std::rc::Rc<std::ffi::CStr, Allocator>");

        // this eventually causes a compilation error
        test_in_magic_whitelist("std::wrong::path::Arc");
    }

    #[test]
    fn not_in_magic_whitelist() {
        test_not_in_magic_whitelist("::String");
        test_not_in_magic_whitelist("some::magical::path::Arc");
    }

    #[test]
    fn infer_optional_inner() {
        test_optional_inner_type("Option<String>", "String");
        test_optional_inner_type("Option::<i32>", "i32");
        test_optional_inner_type("MyOptional<Inner, T>", "Inner");
        test_optional_inner_type("Vec::<T, Allocator>", "T");
    }
}
