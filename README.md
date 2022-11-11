# 🎈 thisctx

A simple crate work with [thiserror](https://crates.io/crates/thiserror) to
create errors with contexts, inspired by
[snafu](https://crates.io/crates/snafu).

## ✍️ Examples

```rust
use std::fs;
use std::path::{Path, PathBuf};
use thisctx::{thisctx, ResultExt};
use thiserror::Error;

thisctx! {
    #[derive(Debug, Error)]
    pub enum Error {
        #[error("I/O failed '{}': {src}", .ctx.0.display())]
        IoFaild {
            #[source]
            @source
            src: std::io::Error,
            @context
            ctx:
                #[derive(Debug)]
                struct (PathBuf),
        },
    }
}

fn load_config(path: &Path) -> Result<String, Error> {
    fs::read_to_string(path).context(IoFaild(path))
}
```

## 📝 Todo

- [x] Switch to Rust 2021.
- [ ] Use derive macro instead.

## ⚖️ License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.
