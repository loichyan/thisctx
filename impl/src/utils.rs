use std::cell::RefCell;
use std::iter::FromIterator;

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream, Result};
use syn::{braced, parenthesized, parse_quote, token, Attribute, Ident, Token};

pub struct TokensWith<'a>(RefCell<Box<dyn 'a + FnMut(&mut TokenStream)>>);

impl<'a> ToTokens for TokensWith<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.borrow_mut()(tokens)
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

pub struct WithSurround<T, S> {
    pub surround: S,
    pub content: T,
}

impl<T, S: ParseWith> WithSurround<T, S> {
    pub fn parse_with<F>(input: ParseStream, f: F) -> Result<Self>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let (surround, content) = S::parse_with(input, f)?;
        Ok(Self { surround, content })
    }
}

impl<T, S: Surround> WithSurround<T, S> {
    pub fn to_tokens_with<F>(&self, tokens: &mut TokenStream, f: F)
    where
        F: FnOnce(&mut TokenStream),
    {
        self.surround.surround(tokens, f)
    }
}

impl<T: Parse, S: ParseWith> Parse for WithSurround<T, S> {
    fn parse(input: ParseStream) -> Result<Self> {
        Self::parse_with(input, T::parse)
    }
}

impl<T: ToTokens, S: Surround> ToTokens for WithSurround<T, S> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.to_tokens_with(tokens, |tokens| self.content.to_tokens(tokens))
    }
}

pub trait ParseWith: Sized {
    fn parse_with<T, F>(input: ParseStream, f: F) -> Result<(Self, T)>
    where
        F: FnOnce(ParseStream) -> Result<T>;
}

pub trait Surround {
    fn surround<F>(&self, tokens: &mut TokenStream, f: F)
    where
        F: FnOnce(&mut TokenStream);
}

pub enum SurroundEnum {
    Brace(Brace),
    Paren(Paren),
}

impl Surround for SurroundEnum {
    fn surround<F>(&self, tokens: &mut TokenStream, f: F)
    where
        F: FnOnce(&mut TokenStream),
    {
        match self {
            Self::Brace(brace) => brace.surround(tokens, f),
            Self::Paren(paren) => paren.surround(tokens, f),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Brace(pub token::Brace);

impl ParseWith for Brace {
    fn parse_with<T, F>(input: ParseStream, f: F) -> Result<(Self, T)>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let content;
        let brace = braced!(content in input);
        f(&content).map(|t| (Self(brace), t))
    }
}

impl Surround for Brace {
    fn surround<F>(&self, tokens: &mut TokenStream, f: F)
    where
        F: FnOnce(&mut TokenStream),
    {
        self.0.surround(tokens, f)
    }
}

#[derive(Clone, Copy)]
pub struct Paren(pub token::Paren);

impl ParseWith for Paren {
    fn parse_with<T, F>(input: ParseStream, f: F) -> Result<(Self, T)>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let content;
        let paren = parenthesized!(content in input);
        f(&content).map(|t| (Self(paren), t))
    }
}

impl Surround for Paren {
    fn surround<F>(&self, tokens: &mut TokenStream, f: F)
    where
        F: FnOnce(&mut TokenStream),
    {
        self.0.surround(tokens, f)
    }
}

pub struct AngleBracket(pub Token![<], pub Token![>]);

impl ParseWith for AngleBracket {
    fn parse_with<T, F>(input: ParseStream, f: F) -> Result<(Self, T)>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let lt = input.parse::<Token![<]>()?;
        let t = f(input)?;
        let gt = input.parse::<Token![>]>()?;
        Ok((Self(lt, gt), t))
    }
}

impl Surround for AngleBracket {
    fn surround<F>(&self, tokens: &mut TokenStream, f: F)
    where
        F: FnOnce(&mut TokenStream),
    {
        self.0.to_tokens(tokens);
        f(tokens);
        self.1.to_tokens(tokens);
    }
}

pub fn tokens_with<'a, F>(f: F) -> TokensWith<'a>
where
    F: 'a + FnMut(&mut TokenStream),
{
    TokensWith(RefCell::new(Box::new(f)))
}

pub struct Punctuated<T, P>(pub syn::punctuated::Punctuated<T, P>);

impl<T, P> Punctuated<T, P> {
    pub fn iter(&self) -> syn::punctuated::Iter<T> {
        self.0.iter()
    }
}

impl<P: Parse> Punctuated<(), P> {
    pub fn visit_parse_with<F>(input: ParseStream, mut f: F) -> Result<()>
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
    pub fn to_tokens_with<'a, F>(&'a self, f: F) -> TokensWith
    where
        F: 'a + FnMut(&T) -> TokenStream,
    {
        let mut f = f;
        tokens_with(move |tokens| {
            self.0.pairs().for_each(|p| {
                f(p.value()).to_tokens(tokens);
                p.punct().to_tokens(tokens)
            })
        })
    }
}

impl<T: ToTokens, P: ToTokens> ToTokens for Punctuated<T, P> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.to_tokens_with(T::to_token_stream).to_tokens(tokens)
    }
}

impl<T, P: Default> FromIterator<T> for Punctuated<T, P> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut inner = syn::punctuated::Punctuated::new();
        for item in iter {
            inner.push(item)
        }
        Self(inner)
    }
}

pub fn custom_ident(s: &str) -> Ident {
    Ident::new(s, Span::call_site())
}

pub fn custom_token<T: Parse>(s: &str) -> T {
    let ident = custom_ident(s);
    parse_quote!(#ident)
}
