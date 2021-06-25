use super::TokensWith;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::{Lookahead1, Parse, ParseStream, Result};
use syn::{braced, parenthesized, token, Token};

pub struct WithSurround<T, S> {
    pub surround: S,
    pub content: T,
}

impl<T, S: ParseWith> WithSurround<T, S> {
    pub fn parse_with<F>(input: ParseStream, f: F) -> Result<Self>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        S::parse_with(input, f)
    }
}

impl<T, S: Surround> WithSurround<T, S> {
    pub fn to_token_stream_with<F>(&self, f: F) -> TokenStream
    where
        F: FnOnce(&T) -> TokenStream,
    {
        self.surround.surround(f(&self.content))
    }
}

impl<T: Parse, S: ParseWith> Parse for WithSurround<T, S> {
    fn parse(input: ParseStream) -> Result<Self> {
        Self::parse_with(input, T::parse)
    }
}

impl<T: ToTokens, S: Surround> ToTokens for WithSurround<T, S> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.to_token_stream_with(T::to_token_stream)
            .to_tokens(tokens)
    }
}

pub trait ParseWith: Sized {
    fn parse_with<T, F>(input: ParseStream, f: F) -> Result<WithSurround<T, Self>>
    where
        F: FnOnce(ParseStream) -> Result<T>;
}

pub trait Surround {
    fn surround<T: ToTokens>(&self, content: T) -> TokenStream;
}

#[derive(Clone, Copy)]
pub enum StructBodySurround {
    Brace(Brace),
    Paren(Paren),
    None,
}

impl StructBodySurround {
    pub fn brace(brace: token::Brace) -> Self {
        Self::Brace(Brace(brace))
    }

    pub fn paren(paren: token::Paren) -> Self {
        Self::Paren(Paren(paren))
    }
}

impl StructBodySurround {
    pub fn parse_with<T, F1, F2, F3>(
        input: ParseStream,
        parse_brace: F1,
        parse_paren: F2,
        parse_none: F3,
    ) -> Result<WithSurround<T, Self>>
    where
        F1: FnOnce(ParseStream) -> Result<T>,
        F2: FnOnce(ParseStream) -> Result<T>,
        F3: FnOnce(Lookahead1) -> Result<T>,
    {
        let lookhead = input.lookahead1();
        if lookhead.peek(token::Brace) {
            let WithSurround { surround, content } = Brace::parse_with(input, parse_brace)?;
            let surround = Self::Brace(surround);
            Ok(WithSurround { surround, content })
        } else if lookhead.peek(token::Paren) {
            let WithSurround { surround, content } = Paren::parse_with(input, parse_paren)?;
            let surround = Self::Paren(surround);
            Ok(WithSurround { surround, content })
        } else {
            let content = parse_none(lookhead)?;
            let surround = Self::None;
            Ok(WithSurround { surround, content })
        }
    }
}

impl Surround for StructBodySurround {
    fn surround<T: ToTokens>(&self, content: T) -> TokenStream {
        match self {
            Self::Brace(brace) => brace.surround(content),
            Self::Paren(paren) => paren.surround(content),
            Self::None => TokenStream::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Brace(pub token::Brace);

impl ParseWith for Brace {
    fn parse_with<T, F>(input: ParseStream, f: F) -> Result<WithSurround<T, Self>>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let content;
        let surround = Self(braced!(content in input));
        f(&content).map(|content| WithSurround { surround, content })
    }
}

impl Surround for Brace {
    fn surround<T: ToTokens>(&self, content: T) -> TokenStream {
        TokensWith::new(|tokens| self.0.surround(tokens, |tokens| content.to_tokens(tokens)))
            .into_token_stream()
    }
}

#[derive(Clone, Copy)]
pub struct Paren(pub token::Paren);

impl ParseWith for Paren {
    fn parse_with<T, F>(input: ParseStream, f: F) -> Result<WithSurround<T, Self>>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let content;
        let surround = Self(parenthesized!(content in input));
        f(&content).map(|content| WithSurround { surround, content })
    }
}

impl Surround for Paren {
    fn surround<T: ToTokens>(&self, content: T) -> TokenStream {
        TokensWith::new(move |tokens| self.0.surround(tokens, |tokens| content.to_tokens(tokens)))
            .into_token_stream()
    }
}

#[derive(Clone, Copy)]
pub struct AngleBracket(pub Token![<], pub Token![>]);

impl ParseWith for AngleBracket {
    fn parse_with<T, F>(input: ParseStream, f: F) -> Result<WithSurround<T, Self>>
    where
        F: FnOnce(ParseStream) -> Result<T>,
    {
        let lt = input.parse::<Token![<]>()?;
        let content = f(input)?;
        let gt = input.parse::<Token![>]>()?;
        let surround = Self(lt, gt);
        Ok(WithSurround { surround, content })
    }
}

impl Surround for AngleBracket {
    fn surround<T: ToTokens>(&self, content: T) -> TokenStream {
        let Self(lt, gt) = self;
        quote!(#lt #content #gt)
    }
}
