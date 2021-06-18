#[derive(Debug)]
pub struct NoneError;

impl std::fmt::Display for NoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "an placeholder for none source error variant")
    }
}

impl std::error::Error for NoneError {}

pub trait IntoError {
    type Error;
    type Source;

    fn into_error(self, source: Self::Source) -> Self::Error;
}

pub trait ResultExt<T, E>: Sized {
    fn context_with<C, F>(self, f: F) -> Result<T, C::Error>
    where
        C: IntoError<Source = E>,
        F: FnOnce() -> C;

    fn context<C>(self, context: C) -> Result<T, C::Error>
    where
        C: IntoError<Source = E>,
    {
        self.context_with(|| context)
    }
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn context_with<C, F>(self, f: F) -> Result<T, C::Error>
    where
        C: IntoError<Source = E>,
        F: FnOnce() -> C,
    {
        self.map_err(|e| f().into_error(e))
    }
}

pub trait OptionExt<T>: Sized {
    fn context_with<C, F>(self, f: F) -> Result<T, C::Error>
    where
        C: IntoError<Source = NoneError>,
        F: FnOnce() -> C;

    fn context<C>(self, context: C) -> Result<T, C::Error>
    where
        C: IntoError<Source = NoneError>,
    {
        self.context_with(|| context)
    }
}

impl<T> OptionExt<T> for Option<T> {
    fn context_with<C, F>(self, f: F) -> Result<T, C::Error>
    where
        C: IntoError<Source = NoneError>,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| f().into_error(NoneError))
    }
}
