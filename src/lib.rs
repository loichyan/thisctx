mod ext;

pub use ext::{OptionExt, ResultExt};
pub use thisctx_impl::thisctx;

#[doc(hidden)]
pub mod private {
    pub use crate::ext::{IntoError, NoneError};
}
