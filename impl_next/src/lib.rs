//! Macro implementation of [thisctx](https://crates.io/crates/thisctx).

#[macro_use]
mod util;
mod attrs;
mod derive_with_context;
mod infer;

use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(WithContextNext, attributes(error, source, thisctx))]
pub fn derive_with_context(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_with_context::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
