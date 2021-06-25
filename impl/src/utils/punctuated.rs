use super::TokensWith;
use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::parse::{Parse, ParseStream, Result};

pub struct Punctuated<T, P>(pub syn::punctuated::Punctuated<T, P>);

impl<T, P> Punctuated<T, P> {
    pub fn new() -> Self {
        Self(syn::punctuated::Punctuated::new())
    }
}

impl<P: Parse> Punctuated<(), P> {
    pub fn visit_with<F>(input: ParseStream, mut f: F) -> Result<()>
    where
        F: FnMut(ParseStream) -> Result<()>,
    {
        loop {
            if input.is_empty() {
                break;
            }
            f(input)?;
            if input.is_empty() {
                break;
            }
            input.parse::<P>()?;
        }
        Ok(())
    }
}

impl<T, P: Parse> Punctuated<T, P> {
    pub fn parse_with<F>(input: ParseStream, mut f: F) -> Result<Self>
    where
        F: FnMut(ParseStream) -> Result<T>,
    {
        let mut inner = syn::punctuated::Punctuated::new();
        loop {
            if input.is_empty() {
                break;
            }
            inner.push_value(f(input)?);
            if input.is_empty() {
                break;
            }
            inner.push_punct(input.parse::<P>()?);
        }
        Ok(Self(inner))
    }
}

impl<T: Parse, P: Parse> Parse for Punctuated<T, P> {
    fn parse(input: ParseStream) -> Result<Self> {
        Self::parse_with(input, T::parse)
    }
}

impl<T, P: ToTokens> Punctuated<T, P> {
    pub fn to_token_stream_with<F>(&self, f: F) -> TokenStream
    where
        F: FnMut(&T) -> TokenStream,
    {
        let mut f = f;
        TokensWith::new(|tokens| {
            self.0.pairs().for_each(|p| {
                f(p.value()).to_tokens(tokens);
                p.punct().to_tokens(tokens)
            })
        })
        .into_token_stream()
    }
}

impl<T: ToTokens, P: ToTokens> ToTokens for Punctuated<T, P> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.to_token_stream_with(T::to_token_stream)
            .to_tokens(tokens)
    }
}
