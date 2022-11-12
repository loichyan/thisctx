//! A simple crate work with [thiserror](https://crates.io/crates/thiserror)
//! to create errors with contexts, inspired by [snafu](https://crates.io/crates/snafu).
//!
//! # Example
//! ```
//! use std::path::{Path, PathBuf};
//! use thisctx::WithContext;
//! use thiserror::Error;
//!
//! #[derive(Debug, Error, WithContext)]
//! pub enum Error {
//!     #[error("I/O failed '{}': {source}", .path.display())]
//!     IoFaild {
//!         source: std::io::Error,
//!         path: PathBuf,
//!     },
//! }
//!
//! fn load_config(path: &Path) -> Result<String, Error> {
//!     std::fs::read_to_string(path).context(IoFaildContext { path })
//! }
//! ```

pub use thisctx_impl::WithContext;

pub trait IntoError {
    type Error;
    type Source;

    fn into_error(self, source: Self::Source) -> Self::Error;
}

pub trait WithContext {
    type Ok;
    type Err;

    fn context_with<C>(self, f: impl FnOnce() -> C) -> Result<Self::Ok, C::Error>
    where
        C: IntoError<Source = Self::Err>;

    #[inline]
    fn context<C>(self, context: C) -> Result<Self::Ok, C::Error>
    where
        Self: Sized,
        C: IntoError<Source = Self::Err>,
    {
        self.context_with(|| context)
    }
}

impl<T, E> WithContext for Result<T, E> {
    type Ok = T;
    type Err = E;

    #[inline]
    fn context_with<C>(self, f: impl FnOnce() -> C) -> Result<T, C::Error>
    where
        C: IntoError<Source = E>,
    {
        self.map_err(|e| f().into_error(e))
    }
}

impl<T> WithContext for Option<T> {
    type Ok = T;
    type Err = ();

    #[inline]
    fn context_with<C>(self, f: impl FnOnce() -> C) -> Result<T, C::Error>
    where
        C: IntoError<Source = ()>,
    {
        self.ok_or_else(|| f().into_error(()))
    }
}

impl WithContext for bool {
    type Ok = ();
    type Err = ();

    #[inline]
    fn context_with<C>(self, f: impl FnOnce() -> C) -> Result<(), C::Error>
    where
        C: IntoError<Source = ()>,
    {
        if self {
            Ok(())
        } else {
            Err(f().into_error(()))
        }
    }
}

impl WithContext for () {
    type Ok = ();
    type Err = ();

    #[inline]
    fn context_with<C>(self, f: impl FnOnce() -> C) -> Result<(), C::Error>
    where
        C: IntoError<Source = ()>,
    {
        Err(f().into_error(()))
    }
}
