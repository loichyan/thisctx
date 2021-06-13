#[cfg(test)]
mod test {
    #![allow(unused)]

    use thisctx::thisctx;
    use thiserror::Error;

    thisctx! {
        #[derive(Debug, Error)]
        enum Error {
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
        }
    }
}
