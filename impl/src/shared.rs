use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

pub struct NoneError;

impl ToTokens for NoneError {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(thisctx::private::NoneError).to_tokens(tokens);
    }
}

pub struct IntoError;

impl ToTokens for IntoError {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote!(thisctx::private::IntoError).to_tokens(tokens);
    }
}
