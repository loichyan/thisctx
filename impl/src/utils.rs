use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{braced, parenthesized, token, Attribute};

pub struct TokensWith<F>(F);

impl<F> ToTokens for TokensWith<F>
where
    F: Fn(&mut TokenStream),
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0(tokens)
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

pub struct Braced;

impl Braced {
    pub fn parse_with<T, F>(input: ParseStream, f: F) -> Result<(token::Brace, T)>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let braced;
        Ok((braced!(braced in input), f(&braced)?))
    }
}

pub struct Parened;

impl Parened {
    pub fn parse_with<T, F>(input: ParseStream, f: F) -> Result<(token::Paren, T)>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let braced;
        Ok((parenthesized!(braced in input), f(&braced)?))
    }
}

pub fn tokens_with<F>(f: F) -> TokensWith<F>
where
    F: Fn(&mut TokenStream),
{
    TokensWith(f)
}

pub fn punctuated_parse<P, F>(input: ParseStream, mut f: F) -> Result<()>
where
    P: Parse,
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

pub fn punctuated_tokens<P, T, I>(tokens: &mut TokenStream, iter: I)
where
    P: ToTokens + Default,
    T: ToTokens,
    I: IntoIterator<Item = T>,
{
    let punct = P::default();
    iter.into_iter().for_each(|item| {
        let item = item.to_token_stream();
        if item.is_empty() {
            return;
        }
        item.to_tokens(tokens);
        punct.to_tokens(tokens);
    })
}
