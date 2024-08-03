use proc_macro2::TokenStream;

macro_rules! NewToken {
    ($($tt:tt)*) => {
        <::syn::Token![$($tt)*] as ::core::default::Default>::default()
    };
}

macro_rules! NewIdent {
    ($ident:ident) => {
        ::syn::Ident::new(stringify!($ident), ::proc_macro2::Span::call_site())
    };
}

pub(crate) struct QuoteWith<F>(pub F)
where
    F: Fn(&mut TokenStream);

impl<F> quote::ToTokens for QuoteWith<F>
where
    F: Fn(&mut TokenStream),
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        (self.0)(tokens)
    }
}
