# 🎈 thisctx

A small crate works with [thiserror](https://crates.io/crates/thiserror) to
create errors with contexts, heavily inspired by
[snafu](https://crates.io/crates/snafu).

## ✍️ Examples

```rust
#[derive(Debug, Error, WithContext)]
pub enum Error {
    #[error("I/O failed at '{1}'")]
    Io(#[source] std::io::Error, PathBuf),
    #[error(transparent)]
    ParseInt(std::num::ParseIntError),
}

fn read_file(path: &Path) -> Result<String, Error> {
    std::fs::read_to_string(path).context(Io(path))
}
```

## ⚙️ Attributes

You can use the `#[thisctx]` attribute with the following options to customize
the expanded code:

| Option       | Type            | Inherited | Container | Variant | Field |
| ------------ | --------------- | --------- | --------- | ------- | ----- |
| `attr`       | `TokenStream[]` | ✔         | ✔         | ✔       | ✔     |
| `generic`    | `bool`          | ✔         | ✔         | ✔       | ✔     |
| `into`       | `Type[]`        | ✔         | ✔         | ✔       |       |
| `module`     | `bool \| Ident` |           | ✔         |         |       |
| `skip`       | `Ident`         | ✔         | ✔         | ✔       |       |
| `suffix`     | `bool \| Ident` | ✔         | ✔         | ✔       |       |
| `unit`       | `bool`          | ✔         | ✔         | ✔       |       |
| `visibility` | `Visibility`    | ✔         | ✔         | ✔       | ✔     |

The `#[source]` and `#[error]` attributes defined in `thiserror` will also be
checked to determine the source error type.

### Option arguments

`#[thisctx]` supports two syntaxes for passing arguments to an option:

- Put tokens directly in the parentheses, e.g. `#[thisctx(visibility(pub))]`
- Use a string literal, e.g. `#[thisctx(visibility = "pub")]`, this is useful in
  older versions of `rustc` that don't support arbitrary tokens in non-macro
  attributes.

An option of type `T[]` can occur multiple times in the same node, while other
types will lead an error.

### Boolean options

You can omit the `true` value in boolean options, e.g. `#[thisctx(skip)]` is
equal to `#[thisctx(skip(true))]`.

Reversed boolean options starts with `no_` can also be used as a shortcut to
pass `false`, e.g. `#[thisctx(no_skip)]` is equal to `#[thisctx(skip(false))]`.

### Inherited options

An inherited option uses the value of its parent node if no value is provided,
for example:

```rust
#[derive(Debug, Error, WithContext)]
#[thisctx(skip)]
enum Error {
    // This variant will be ignored since `skip=true` is inherited.
    #[error(transparent)]
    Io(std::io::Error),
    // This variant will be processed.
    #[thisctx(no_skip)]
    #[error(transparent)]
    ParseInt(std::num::ParseIntError),
}
```

An option of type `T[]` will concatenate arguments from its ancestors instead of
overriding them.

```rust
#[derive(WithContext)]
#[thisctx(attr(derive(Debug)))]
enum Error {
    #[thisctx(attr(derive(Clone, Copy)))]
    Io(#[source] std::io::Error),
    ParseInt(#[source] std::num::ParseIntError),
}
```

Expanded example:


```rust
// The order of attributes (and other options) is guaranteed by the order of
// inheritance.
// Attributes from the child node.
#[derive(Clone, Copy)]
// Attributes from the parent node.
#[derive(Debug)]
struct Io;

#[derive(Debug)]
struct ParseInt;
```

### `source`

If a field has the `#[source]` attribute or is named `source`, the type of this
field will be assigned to `IntoError::Source` and will not appear in the
generated context types.

```rust
#[derive(Debug, Error, WithContext)]
#[error("IO failed at '{1}'")]
struct Error(#[source] std::io::Error, PathBuf);
```

Expanded example:

```rust
struct ErrorContext<T1 = PathBuf>(T1);

impl<T1> IntoError<Error> for ErrorContext<T1>
where
    T1: Into<PathBuf>,
{
    type Source = std::io::Error;

    fn into_error(self, source: Self::Source) -> Error {
        Error(source, self.0.into())
    }
}
```

### `error`

If a variant is transparent (which has `#[error(transparent)]`), the first field
(which should also be the only field) will be considered as the source field.

### `thisctx.attr`

An option used to add extra attributes to a generated node.

```rust
#[derive(WithContext)]
#[thisctx(attr(derive(Debug)))]
struct Error {
    reason: String,
}
```

Expanded example:

```rust
#[derive(Debug)]
struct ErrorContext<T1 = String> {
    reason: T1,
}
```

### `thisctx.generic`

An option to disable generics of a generated node.

```rust
#[derive(WithContext)]
struct Error {
    reason: String,
    #[thisctx(no_generic)]
    path: PathBuf,
}
```

Expanded example:

```rust
struct ErrorContext<T1 = String> {
    reason: T1,
    path: PathBuf,
}
```

The generics provide a convenient way to construct context types, for example:

```rust
let _: Error = ErrorContext {
    // You can use &str directly because String implements From<&str>,
    reason: "anyhow",
    // whereas without generics you have to convert the data to PathBuf manually.
    path: "/some/path".into(),
}.build();
```

### `thisctx.into`

An option for converting generated types to a remote error type.

```rust
// Probably an error defined in another crate.
#[derive(Debug, Error)]
enum RemoteError {
    #[error("Custom: {0}")]
    Custom(String),
}

// From<T> is required by #[thisctx(into)]
impl From<MyError> for RemoteError {
    fn from(e: MyError) -> Self {
        Self::Custom(e.to_string())
    }
}

#[derive(Debug, Error, WithContext)]
#[thisctx(into(RemoteError))]
#[error("MyError: {0}")]
struct MyError(String);

let _: MyError = MyErrorContext("anyhow").build();
// It's possible to construct a remote error from the local context type.
let _: RemoteError = MyErrorContext("anyhow").build();
```

### `thisctx.module`

This option allows you put all generated context types into a single module.

```rust
#[derive(Debug, Error, WithContext)]
#[thisctx(module(context))]
pub enum Error {
    #[error(transparent)]
    Io(std::io::Error),
    #[error(transparent)]
    ParseInt(std::num::ParseIntError),
}
```

Expanded example:

```rust
pub mod context {
    pub struct Io;
    pub struct ParseInt;
}
```

You can also set this option to `true` to use the snake case of the container
name as the module name, e.g. `#[thisctx(module)]` on `enum MyError` is equal to
`#[thisctx(module(my_error))]`.

### `thisctx.skip`

This option is used to skip generating context types for the specified variant.

```rust
#[derive(Debug, Error, WithContext)]
enum Error {
    #[thisctx(skip)]
    #[error(transparent)]
    Io(std::io::Error),
    #[error(transparent)]
    ParseInt(std::num::ParseIntError),
}
```

Expanded example:

```rust
pub struct ParseInt;
```

### `thisctx.suffix`

An option to add a suffix to the names of the generated context types.

By default, only `struct`s will be added the builtin suffix `Context` since the
generated type without a suffix will confict with the error type.

```rust
#[derive(Debug, Error, WithContext)]
#[thisctx(suffix(Error))]
enum Error {
    #[error(transparent)]
    Io(std::io::Error),
    #[error(transparent)]
    ParseInt(std::num::ParseIntError),
}
```

Expanded example:

```rust
pub struct IoError;
pub struct ParseIntError;
```

The value `true` means that the default suffix is used `Context` and the value
`false` will remove the suffix from the generated type.

## 📝 Todo

- [x] ~~Switch to Rust 2021.~~
- [x] MSRV v1.33
- [x] Use derive macro instead.
- [x] Add attributes to context types.
- [x] Support transparent error.
- [x] Support generics.
- [x] Simplify the derive implementation.
- [ ] More documentation.
- [ ] More tests.

## 🚩 Minimal suppoted Rust version

All tests under `tests/*` passed with `rustc v1.33`, previous versions may not
compile.

## ⚖️ License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
