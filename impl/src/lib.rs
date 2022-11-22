//! Macro implementation of the [thisctx](https://crates.io/crates/thisctx) crate.

mod ast;
mod attr;
mod expand;
mod generics;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(WithContext, attributes(error, source, thisctx))]
pub fn derive_with_context(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand::derive(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
