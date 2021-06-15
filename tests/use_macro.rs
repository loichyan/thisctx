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
        #[error("I/O failed: {src}")]
        IoFaildWithoutPath {
            @source
            src: std::io::Error,
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
        #[error("invalid option: '{}'", .ctx.0)]
        InvalidOpt {
            @context
            ctx:
                #[derive(Debug)]
                struct (String),
        },
        #[error("invalid argument: '{}'", 0.0)]
        InvalidArg (
            #[derive(Debug)]
            struct (String)
        ),
        #[error("I have no idea about this error")]
        JustFailed { },
        #[error("it just failed either")]
        FailedEither ( ),
    }
}
