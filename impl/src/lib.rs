mod expand;

use expand::ThisCtx;
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

#[proc_macro]
pub fn thisctx(tokens: TokenStream) -> TokenStream {
    parse_macro_input!(tokens as ThisCtx)
        .into_token_stream()
        .into()
}
