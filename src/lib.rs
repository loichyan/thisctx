//! A simple crate work with [thiserror](https://crates.io/crates/thiserror)
//! to create errors with contexts, inspired by [snafu](https://crates.io/crates/snafu).
//!
//! # Example
//!
//! ```
//! use std::path::{Path, PathBuf};
//! use thisctx::WithContext;
//! use thiserror::Error;
//!
//! #[derive(Debug, Error, WithContext)]
//! pub enum Error {
//!     #[error("I/O failed '{path}': {source}")]
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
#![no_std]

pub use thisctx_impl::WithContext;

pub trait IntoError<E> {
    type Source;

    fn into_error(self, source: Self::Source) -> E;

    #[inline]
    fn build(self) -> E
    where
        Self: Sized,
        Self::Source: Default,
    {
        self.into_error(<_>::default())
    }

    #[inline]
    fn fail<T>(self) -> Result<T, E>
    where
        Self: Sized,
        Self::Source: Default,
    {
        Err(self.build())
    }
}

// TODO: must use?
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
