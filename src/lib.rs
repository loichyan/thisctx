//! # 🎈 thisctx
//!
//! A small crate works with [thiserror](https://crates.io/crates/thiserror) to
//! create errors with contexts, inspired by
//! [snafu](https://crates.io/crates/snafu).
//!
//! ## ✍️ Examples
//!
//! ```rust
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
//!     std::fs::read_to_string(path).context(IoFaild { path })
//! }
//! ```
//!
//! ## 📝 Todo
//!
//! - [x] ~~Switch to Rust 2021.~~
//! - [x] MSRV v1.33
//! - [x] Use derive macro instead.
//! - [x] Add attributes to context types.
//! - [x] Support transparent error.
//! - [x] Support generics.
//! - [x] Simplify the derive implementation.
//! - [ ] More documentation.
//! - [ ] More tests.
//!
//! ## 🚩 Minimal suppoted Rust version
//!
//! All tests under `tests/*` passed with `rustc v1.33`, previous versions may not
//! compile.
//!
//! ## ⚖️ License
//!
//! Licensed under either of
//!
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
//!   <http://www.apache.org/licenses/LICENSE-2.0>)
//! - MIT license ([LICENSE-MIT](LICENSE-MIT) or
//!   <http://opensource.org/licenses/MIT>)
//!
//! at your option.
#![no_std]

pub use thisctx_impl::WithContext;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct NoneSource;

pub trait IntoSource<T>: Sized {
    fn into_source(self) -> T;
}

pub trait IntoError<E>: Sized {
    type Source;

    fn into_error(self, source: Self::Source) -> E;

    #[inline]
    fn build(self) -> E
    where
        Self: IntoError<E, Source = NoneSource>,
    {
        self.into_error(NoneSource)
    }

    // TODO: use never type instead?
    #[inline]
    fn fail<T>(self) -> Result<T, E>
    where
        Self: IntoError<E, Source = NoneSource>,
    {
        Err(self.build())
    }
}

pub trait WithContext: Sized {
    type Ok;
    type Err;

    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<Self::Ok, E>
    where
        C: IntoError<E>,
        Self::Err: Into<C::Source>;

    #[inline]
    fn context<E, C>(self, context: C) -> Result<Self::Ok, E>
    where
        C: IntoError<E>,
        Self::Err: Into<C::Source>,
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
        C: IntoError<E>,
        Err: Into<C::Source>,
    {
        self.map_err(|e| f().into_error(e.into()))
    }
}

impl<T> WithContext for Option<T> {
    type Ok = T;
    type Err = NoneSource;

    #[inline]
    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<T, E>
    where
        C: IntoError<E>,
        NoneSource: Into<C::Source>,
    {
        self.ok_or_else(|| f().into_error(NoneSource.into()))
    }
}
