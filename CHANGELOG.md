# Changelog

## v0.3.0 (2023-03-11)

### Feat

- **derive**: allow use `#[thisctx(pub(...))]` to change visibility
- **lib**: use `into()` to construct the source type of a context
- **lib**: use `NoneSource` instead of `()`

### Fix

- **derive**: only impl `From<Context>` for an error when `source` is not specified

### Refactor

- **lib**: traits require `Sized` bound
- **derive**: simplify implementation details

## v0.2.0 (2023-01-09)

### Feat

- **derive**: change MSRV to v1.33
- **derive**: support `#[thisctx(attr = "...")]` syntax
- minimal suppoted rust version v1.34
- **derive**: use `#[thisctx(module)]` to generate context types into a single module
- **lib**: add `IntoError::{build, fail}`
- **derive**: convert context types into errors when associated source types implement `Default`
- **derive**: add attributes `#[thisctx(context)]` and `#[thisctx(generic)]`
- **lib**: support `#![no_std]`
- **derive**: support generics (#7)
- **derive**: associate contexts with a remote error (#6)
- **derive**: support add extra attributes to generated types (#5)
- **derive**: auto generate unit structs for empty contexts (#4)
- **derive**: generate extra traits and methods (#3)
- **derive**: add attribute `#[thisctx(suffix)]` (#2)
- **derive**: add attribute `#[thisctx(visibility)]` (#1)
- **impl**: use derive macro instead
- **macro**: implement `fail()` for `@context`
- **macro**: implement `build()` for `@context`
- **macro**: implement `From<@source>` for error enum
- **macro**: do not generate context without `@context`
- **macro**: no generic for unit context struct

### Fix

- **derive**: allow `#[error]` attribute
- **derive**: generate conversion generics based on field names
- **derive**: don't generate `into_error` method
- **derive**: inherit visibility for generated fields

### Refactor

- **derive**: use more new type patterns
- **macro**: use `IntoError::into_error()` in `@context::build()`
- **macro**: reduce redundant code
- **macro**: rename some items
- **macro**: make `Parse` of `Context` separate

## v0.1.0 (2021-06-18)

### BREAKING CHANGE

- The tuple variant will be resolved in a similar way to the `Sturct Variant`, which
means you have to explicitly provide `@source` and `@context`.

### Feat

- **macro**: support add attributes to source and context fields
- **ext**: add `context_with()`
- **macro**: support generic context
- **macro**: support empty tuple variant
- support visibility control
- support tuple variant
- move `thisctx::ext::{IntoError,NoneError}` to public
- implement converting from context without source for error enum
- implement `IntoError` for context struts, add `ResultExt` and `OptionExt`
- support tuple struct context
- use proc_macro instead of macro_rules
- support unit and struct enum variants

### Refactor

- **macro**: eliminate redundant code
- **macro**: eliminate redundant code
- rewrite `thisctx_impl::expand`
- rename local variables, remove redundant code
