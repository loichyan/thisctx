pub struct NoneError;

pub trait IntoError {
    type Error;
    type Source;

    fn into_error(self, source: Self::Source) -> Self::Error;
}

pub trait ResultExt<T, E> {
    fn context<C>(self, context: C) -> Result<T, C::Error>
    where
        C: IntoError<Source = E>;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn context<C>(self, context: C) -> Result<T, C::Error>
    where
        C: IntoError<Source = E>,
    {
        self.map_err(|e| context.into_error(e))
    }
}

pub trait OptionExt<T> {
    fn context<C>(self, context: C) -> Result<T, C::Error>
    where
        C: IntoError<Source = NoneError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn context<C>(self, context: C) -> Result<T, C::Error>
    where
        C: IntoError<Source = NoneError>,
    {
        self.ok_or_else(|| context.into_error(NoneError))
    }
}
