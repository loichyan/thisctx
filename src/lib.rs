//! A little crate that works with [thiserror](https://crates.io/crates/thiserror)
//! to create errors with context, heavily inspired by [snafu](https://crates.io/crates/snafu).
//!
//! # 🚩 Minimum supported Rust version
//!
//! All tests passed with `rustc v1.56`, earlier versions may not compile.
#![no_std]

pub use thisctx_impl::WithContext;
pub use thisctx_impl_next::WithContextNext;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct NoneSource;

pub trait IntoError<E>: Sized {
    type Source;

    fn into_error(self, source: Self::Source) -> E;

    fn build(self) -> E
    where
        Self: IntoError<E, Source = NoneSource>,
    {
        self.into_error(NoneSource)
    }

    // TODO: use never type instead?
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

    fn context<E, C>(self, context: C) -> Result<Self::Ok, E>
    where
        C: IntoError<E>,
        Self::Err: Into<C::Source>,
    {
        self.context_with(|| context)
    }
}

impl<T, Err> WithContext for Result<T, Err> {
    type Err = Err;
    type Ok = T;

    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<T, E>
    where
        C: IntoError<E>,
        Err: Into<C::Source>,
    {
        self.map_err(|e| f().into_error(e.into()))
    }
}

impl<T> WithContext for Option<T> {
    type Err = NoneSource;
    type Ok = T;

    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<T, E>
    where
        C: IntoError<E>,
        NoneSource: Into<C::Source>,
    {
        self.ok_or_else(|| f().into_error(NoneSource.into()))
    }
}

pub trait IntoErrorNext: Sized {
    type Target;
    type Source;

    fn into_error(self, source: Self::Source) -> Self::Target;

    fn build(self) -> Self::Target
    where
        Self: IntoErrorNext<Source = NoneSource>,
    {
        self.into_error(NoneSource)
    }

    fn fail<T>(self) -> Result<T, Self::Target>
    where
        Self: IntoErrorNext<Source = NoneSource>,
    {
        Err(self.build())
    }
}

pub trait Optional: Default {
    type Inner;

    fn set(&mut self, value: Self::Inner) -> Option<Self::Inner>;
}

impl<T> Optional for Option<T> {
    type Inner = T;

    fn set(&mut self, value: Self::Inner) -> Option<Self::Inner> {
        self.replace(value)
    }
}

pub trait WithOptional<T> {
    fn with_optional(&mut self, value: T) -> Option<T>;
}

pub trait WithContextNext: Sized {
    type Ok;
    type Err;

    fn context<C>(self, context: C) -> Result<Self::Ok, C::Target>
    where
        C: IntoErrorNext,
        Self::Err: Into<C::Source>,
    {
        self.context_with(|| context)
    }

    fn context_with<C>(self, f: impl FnOnce() -> C) -> Result<Self::Ok, C::Target>
    where
        C: IntoErrorNext,
        Self::Err: Into<C::Source>;

    fn provide<C>(self, value: impl Into<C>) -> Self
    where
        Self::Err: WithOptional<C>,
    {
        self.provide_with(|| value.into())
    }

    fn provide_with<C>(self, value: impl FnOnce() -> C) -> Self
    where
        Self::Err: WithOptional<C>;
}

impl<T, E> WithContextNext for Result<T, E> {
    type Err = E;
    type Ok = T;

    fn context_with<C>(self, f: impl FnOnce() -> C) -> Result<T, C::Target>
    where
        C: IntoErrorNext,
        E: Into<C::Source>,
    {
        self.map_err(|e| f().into_error(e.into()))
    }

    fn provide_with<C>(mut self, value: impl FnOnce() -> C) -> Self
    where
        E: WithOptional<C>,
    {
        if let Err(ref mut e) = self {
            e.with_optional(value());
        }
        self
    }
}

impl<T> WithContextNext for Option<T> {
    type Err = NoneSource;
    type Ok = T;

    fn context_with<C>(self, f: impl FnOnce() -> C) -> Result<T, C::Target>
    where
        C: IntoErrorNext,
        NoneSource: Into<C::Source>,
    {
        self.ok_or_else(|| f().into_error(NoneSource.into()))
    }

    fn provide_with<C>(self, _: impl FnOnce() -> C) -> Self
    where
        NoneSource: WithOptional<C>,
    {
        self
    }
}

/// **NOT PUBLIC APIS**
#[doc(hidden)]
pub mod private {
    pub use core::convert::{From, Into};
    pub use core::default::Default;
    pub use core::option::Option;

    pub use super::*;
}
