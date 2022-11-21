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

pub trait IntoError<E> {
    type Source;

    fn into_error(self, source: Self::Source) -> E;
}

pub trait WithContext {
    type Ok;
    type Err;

    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<Self::Ok, E>
    where
        C: IntoError<E, Source = Self::Err>;

    #[inline]
    fn context<E, C>(self, context: C) -> Result<Self::Ok, E>
    where
        Self: Sized,
        C: IntoError<E, Source = Self::Err>,
    {
        self.context_with(|| context)
    }
}

impl<T, Err> WithContext for Result<T, Err> {
    type Ok = T;
    type Err = Err;

    #[inline]
    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<T, E>
    where
        C: IntoError<E, Source = Err>,
    {
        self.map_err(|e| f().into_error(e))
    }
}

impl<T> WithContext for Option<T> {
    type Ok = T;
    type Err = ();

    #[inline]
    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<T, E>
    where
        C: IntoError<E, Source = ()>,
    {
        self.ok_or_else(|| f().into_error(()))
    }
}

impl WithContext for bool {
    type Ok = ();
    type Err = ();

    #[inline]
    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<(), E>
    where
        C: IntoError<E, Source = ()>,
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
    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<(), E>
    where
        C: IntoError<E, Source = ()>,
    {
        Err(f().into_error(()))
    }
}
