# THISCTX

A simple crate work with [thiserror](https://crates.io/crates/thiserror) to create errors with contexts, inspired by [snafu](https://crates.io/crates/snafu);

## Examples

```rust
use thisctx::thisctx;
use thiserror::Error;

thisctx! {
    #[derive(Debug, Error)]
    pub enum Error {
        #[error("I/O failed '{}': {src}", .ctx.path.display())]
        IoFaild {
            @source
            src: std::io::Error,
            @context
            ctx:
                #[derive(Debug)]
                struct {
                    path: std::path::PathBuf,
                },
        },
        #[error("invalid file '{}': {}", .ctx.path.display(), .ctx.disc)]
        InvalidFile {
            @context
            ctx:
                #[derive(Debug)]
                struct {
                    disc: String,
                    path: std::path::PathBuf,
                },
        },
        #[error("invalid argument: '{}'", 0.0)]
        InvalidArg (
            #[derive(Debug)]
            struct (String)
        ),
        #[error("I have no idea about this error")]
        JustFailed,
    }
}
```

## License

This software is released under the [MIT License](./LICENSE).
