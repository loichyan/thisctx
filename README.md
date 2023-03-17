# 🎈 thisctx

A small crate works with [thiserror](https://crates.io/crates/thiserror) to
create errors with contexts, inspired by
[snafu](https://crates.io/crates/snafu).

## ✍️ Examples

```rust
use std::path::{Path, PathBuf};
use thisctx::WithContext;
use thiserror::Error;

#[derive(Debug, Error, WithContext)]
pub enum Error {
    #[error("I/O failed '{path}': {source}")]
    IoFaild {
        source: std::io::Error,
        path: PathBuf,
    },
}

fn load_config(path: &Path) -> Result<String, Error> {
    std::fs::read_to_string(path).context(IoFaild { path })
}
```

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
