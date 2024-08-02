//! A little crate that works with [thiserror](https://crates.io/crates/thiserror)
//! to create errors with context, heavily inspired by
//! [snafu](https://crates.io/crates/snafu).
//!
//! # ✍️ Examples
//!
//! ```
//! # use std::path::{Path, PathBuf};
//! # use thisctx::WithContext;
//! # use thiserror::Error;
//! #[derive(Debug, Error, WithContext)]
//! pub enum Error {
//!     #[error("I/O failed at '{1}'")]
//!     Io(#[source] std::io::Error, PathBuf),
//!     #[error(transparent)]
//!     ParseInt(std::num::ParseIntError),
//! }
//!
//! fn read_file(path: &Path) -> Result<String, Error> {
//!     std::fs::read_to_string(path).context(Io(path))
//! }
//! ```
//!
//! # ⚙️ Attributes
//!
//! You can use the `#[thisctx]` attribute with the following options to
//! customize the expanded code:
//!
//! | Option       | Type            | Inherited | Container | Variant | Field |
//! | ------------ | --------------- | --------- | --------- | ------- | ----- |
//! | `attr`       | `TokenStream[]` | ✔         | ✔         | ✔       | ✔     |
//! | `generic`    | `bool`          | ✔         | ✔         | ✔       | ✔     |
//! | `into`       | `Type[]`        | ✔         | ✔         | ✔       |       |
//! | `module`     | `bool \| Ident` |           | ✔         |         |       |
//! | `skip`       | `Ident`         | ✔         | ✔         | ✔       |       |
//! | `suffix`     | `bool \| Ident` | ✔         | ✔         | ✔       |       |
//! | `unit`       | `bool`          | ✔         | ✔         | ✔       |       |
//! | `visibility` | `Visibility`    | ✔         | ✔         | ✔       | ✔     |
//!
//! The `#[source]` and `#[error]` attributes defined in `thiserror` are also
//! checked to determine the source error type.
//!
//! ## Option arguments
//!
//! `#[thisctx]` supports two syntaxes for passing arguments to an option:
//!
//! - Put tokens directly in parentheses, e.g. `#[thisctx(visibility(pub))]`
//! - Use a string literal, e.g. `#[thisctx(visibility = "pub")]`, which is
//!   useful in older versions of `rustc` that don't support arbitrary tokens in
//!   non-macro attributes.
//!
//! An option of type `T[]` can occur multiple times in the same node, while
//! other types result in an error.
//!
//! ## Boolean options
//!
//! You can omit the `true` value in boolean options, e.g. `#[thisctx(skip)]` is
//! equal to `#[thisctx(skip(true))]`.
//!
//! Negative boolean options starting with `no_` can also be used as a shortcut
//! to pass `false`, e.g. `#[thisctx(no_skip)]` is equal to
//! `#[thisctx(skip(false))]`.
//!
//! ## Inherited options
//!
//! An inherited option uses the value of its parent node if no value is
//! specified, for example:
//!
//! ```
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! #[thisctx(skip)]
//! enum Error {
//!     // This variant is ignored since `skip=true` is inherited.
//!     Io(#[source] std::io::Error),
//!     // This variant is processed.
//!     #[thisctx(no_skip)]
//!     ParseInt(#[source] std::num::ParseIntError),
//! }
//! ```
//!
//! An option of type `T[]` concatenates arguments from its ancestors instead of
//! overriding them:
//!
//! ```
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! #[thisctx(attr(derive(Debug)))]
//! enum Error {
//!     #[thisctx(attr(derive(Clone, Copy)))]
//!     Io(#[source] std::io::Error),
//!     ParseInt(#[source] std::num::ParseIntError),
//! }
//! ```
//!
//! Expanded example:
//!
//! ```
//! // The order of attributes (and other options) is determined by the order of inheritance.
//! // Attributes from the child node.
//! #[derive(Clone, Copy)]
//! // Attributes from the parent node.
//! #[derive(Debug)]
//! struct Io;
//!
//! #[derive(Debug)]
//! struct ParseInt;
//! ```
//!
//! ## `source`
//!
//! If a field has the `#[source]` attribute or is named `source`, the type of
//! this field is assigned to `IntoError::Source` and doesn't appear in the
//! generated context types.
//!
//! ```
//! # use std::path::PathBuf;
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! struct Error(#[source] std::io::Error, PathBuf);
//! ```
//!
//! Expanded example:
//!
//! ```
//! # use std::path::PathBuf;
//! # use thisctx::{IntoError, WithContext};
//! # struct Error(std::io::Error, PathBuf);
//! struct ErrorContext<T1 = PathBuf>(T1);
//!
//! impl<T1> IntoError<Error> for ErrorContext<T1>
//! where
//!     T1: Into<PathBuf>,
//! {
//!     type Source = std::io::Error;
//!
//!     fn into_error(self, source: Self::Source) -> Error {
//!         Error(source, self.0.into())
//!     }
//! }
//! ```
//!
//! ## `error`
//!
//! If a variant is transparent (which has `#[error(transparent)]`), the first
//! field (which should also be the only field) is considered as the source
//! field.
//!
//! ## `thisctx.attr`
//!
//! An option used to append addition attributes to a generated node.
//!
//! ```
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! #[thisctx(attr(derive(Debug)))]
//! struct Error {
//!     reason: String,
//! }
//! ```
//!
//! Expanded example:
//!
//! ```
//! #[derive(Debug)]
//! struct ErrorContext<T1 = String> {
//!     reason: T1,
//! }
//! ```
//!
//! `thisctx` allows you to add some common attributes without `attr(...)`,
//! including:
//!
//! - `cfg`
//! - `cfg_attr`
//! - `derive`
//! - `doc`
//!
//! This means that the preceding example can also be written as:
//!
//! ```
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! #[thisctx(derive(Debug))]
//! struct Error {
//!     reason: String,
//! }
//! ```
//!
//! ## `thisctx.generic`
//!
//! An option to remove generics from generated code.
//!
//! ```
//! # use std::path::PathBuf;
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! struct Error {
//!     reason: String,
//!     #[thisctx(no_generic)]
//!     path: PathBuf,
//! }
//! ```
//!
//! Expanded example:
//!
//! ```
//! # use std::path::PathBuf;
//! struct ErrorContext<T1 = String> {
//!     reason: T1,
//!     path: PathBuf,
//! }
//! ```
//!
//! The generics provide a convenient way to construct context types, for
//! example:
//!
//! ```
//! # use std::path::PathBuf;
//! # use thisctx::{IntoError, WithContext};
//! # #[derive(WithContext)]
//! # struct Error {
//! #     reason: String,
//! #     #[thisctx(no_generic)]
//! #     path: PathBuf,
//! # }
//! let _: Error = ErrorContext {
//!     // You can use &str directly because String implements From<&str>,
//!     reason: "anyhow",
//!     // whereas without generics you have to convert the data to PathBuf manually.
//!     path: "/some/path".into(),
//! }
//! .build();
//! ```
//!
//! ## `thisctx.into`
//!
//! An option for converting generated types to a remote error type.
//!
//! ```
//! # use thisctx::{IntoError, WithContext};
//! // Probably an error defined in another crate.
//! enum RemoteError {
//!     Custom(String),
//! }
//!
//! // From<T> is required by #[thisctx(into)]
//! impl From<MyError> for RemoteError {
//!     fn from(e: MyError) -> Self {
//!         RemoteError::Custom(e.0)
//!     }
//! }
//!
//! #[derive(WithContext)]
//! #[thisctx(into(RemoteError))]
//! struct MyError(String);
//!
//! let _: MyError = MyErrorContext("anyhow").build();
//! // It's possible to construct a remote error from the local context type.
//! let _: RemoteError = MyErrorContext("anyhow").build();
//! ```
//!
//! ## `thisctx.module`
//!
//! This option allows you to put all generated context types into a single
//! module.
//!
//! ```
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! #[thisctx(module(context))]
//! pub enum Error {
//!     Io(#[source] std::io::Error),
//!     ParseInt(#[source] std::num::ParseIntError),
//! }
//! # fn main() {}
//! ```
//!
//! Expanded example:
//!
//! ```
//! pub mod context {
//!     pub struct Io;
//!     pub struct ParseInt;
//! }
//! ```
//!
//! > You can also set this option to `true` to use the snake case of the
//! > container
//! > name as the module name, e.g. `#[thisctx(module)]` on `enum MyError` is
//! > equal
//! > to `#[thisctx(module(my_error))]`.
//!
//! ## `thisctx.skip`
//!
//! This option is used to skip generating context types for the specified
//! variant.
//!
//! ```
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! enum Error {
//!     #[thisctx(skip)]
//!     Io(#[source] std::io::Error),
//!     ParseInt(#[source] std::num::ParseIntError),
//! }
//! ```
//!
//! Expanded example:
//!
//! ```
//! struct ParseInt;
//! ```
//!
//! ## `thisctx.suffix`
//!
//! An option to add a suffix to the names of the generated context types.
//!
//! By default, only `struct`s get the builtin suffix `Context` since the
//! generated types without suffix conflict with the error type.
//!
//! ```
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! #[thisctx(suffix(Error))]
//! enum Error {
//!     Io(#[source] std::io::Error),
//!     ParseInt(#[source] std::num::ParseIntError),
//! }
//! ```
//!
//! Expanded example:
//!
//! ```
//! struct IoError;
//! struct ParseIntError;
//! ```
//!
//! > The value `true` means to use the default suffix `Context` and the value
//! > `false` removes the suffix from the generated type.
//!
//! ## `thisctx.unit`
//!
//! In Rust, parentheses are required to construct a tuple struct even if it's
//! empty. `thisctx` converts an empty struct to a unit struct by default. This
//! allows you use the struct name to create a new context without having to add
//! parentheses each time and can be turned off by passing
//! `#[thisctx(no_unit)]`.
//!
//! ```rust
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! enum Error {
//!     #[thisctx(no_unit)]
//!     Io(#[source] std::io::Error),
//!     ParseInt(#[source] std::num::ParseIntError),
//! }
//! ```
//!
//! Expanded example:
//!
//! ```rust
//! struct IoError();
//! struct ParseIntError;
//! ```
//!
//! ## `thisctx.visibility`
//!
//! This option is used to change the visibility of the generated types and
//! fields and can be written in shorthand as `#[pub(...)]`.
//!
//! ```rust
//! # use thisctx::WithContext;
//! #[derive(WithContext)]
//! #[thisctx(pub(crate))]
//! pub enum Error {
//!     Io(#[source] std::io::Error),
//!     ParseInt(#[source] std::num::ParseIntError),
//! }
//! ```
//!
//! Expanded example:
//!
//! ```rust
//! pub(crate) struct IoError;
//! pub(crate) struct ParseIntError;
//! ```
//!
//! # 📝 Todo
//!
//! - [ ] Simplify the derive implementation.
//! - [ ] More tests.
//!
//! # 🚩 Minimal supported Rust version
//!
//! All tests passed with `rustc v1.56`, earlier versions may not compile.
#![no_std]

pub use thisctx_impl::WithContext;

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct NoneSource;

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
    type Err = Err;
    type Ok = T;

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
    type Err = NoneSource;
    type Ok = T;

    #[inline]
    fn context_with<E, C>(self, f: impl FnOnce() -> C) -> Result<T, E>
    where
        C: IntoError<E>,
        NoneSource: Into<C::Source>,
    {
        self.ok_or_else(|| f().into_error(NoneSource.into()))
    }
}
