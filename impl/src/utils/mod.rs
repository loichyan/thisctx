mod punctuated;
mod surround;

pub use self::punctuated::Punctuated;
pub use self::surround::{
    AngleBracket, Brace, Paren, ParseWith, StructBodySurround, Surround, WithSurround,
};

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{parse_quote, Attribute, Ident};

pub struct TokensWith<'a>(Box<dyn 'a + FnOnce(&mut TokenStream)>);

impl<'a> TokensWith<'a> {
    pub fn new<F>(f: F) -> Self
    where
        F: 'a + FnOnce(&mut TokenStream),
    {
        Self(Box::new(f))
    }

    pub fn into_tokens(self, tokens: &mut TokenStream) {
        self.0(tokens)
    }

    pub fn into_token_stream(self) -> TokenStream {
        let mut tokens = TokenStream::new();
        self.into_tokens(&mut tokens);
        tokens
    }
}

pub struct Attributes(pub Vec<Attribute>);

impl Parse for Attributes {
    fn parse(input: ParseStream) -> Result<Self> {
        let inner = Attribute::parse_outer(input)?;
        Ok(Self(inner))
    }
}

impl ToTokens for Attributes {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self(inner) = self;
        quote!(#(#inner)*).to_tokens(tokens);
    }
}

pub fn custom_ident(s: &str) -> Ident {
    Ident::new(s, Span::call_site())
}

pub fn custom_token<T: Parse>(s: &str) -> T {
    let ident = custom_ident(s);
    parse_quote!(#ident)
}
