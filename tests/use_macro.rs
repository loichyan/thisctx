use thisctx::thisctx;
use thiserror::Error;

thisctx! {
    #[derive(Debug, Error)]
    pub enum Error {
        #[error("I/O failed '{}': {src}", .ctx.path.display())]
        IoFaild {
            #[source]
            @source
            src: std::io::Error,
            @context
            ctx:
                #[derive(Debug)]
                struct {
                    pub path: std::path::PathBuf,
                },
        },
        #[error("I/O failed: {src}")]
        IoFaildWithoutPath {
            #[source]
            @source
            src: std::io::Error,
        },
        #[error("invalid file '{}': {}", .ctx.path.display(), .ctx.desc)]
        InvalidFile {
            @context
            ctx:
                #[derive(Debug)]
                struct {
                    pub desc: String,
                    pub path: std::path::PathBuf,
                },
        },
        #[error("invalid option: '{}'", 0.0)]
        InvalidOpt (
            @context
            #[derive(Debug)]
            struct (String),
        ),
        #[error("invalid argument: '{}'", 1.0)]
        InvalidArg (
            #[source]
            @source
            thisctx::NoneError,
            @context
            #[derive(Debug)]
            struct (String)
        ),
        #[error("I have no idea about this error")]
        JustFailed { },
        #[error("it just failed either")]
        FailedEither ( ),
    }
}
