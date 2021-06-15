mod expand;
mod shared;
mod utils;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

#[proc_macro]
pub fn thisctx(tokens: TokenStream) -> TokenStream {
    parse_macro_input!(tokens as expand::ThisCtx)
        .into_token_stream()
        .into()
}
