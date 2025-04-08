// Required for derive to work.
extern crate self as web_message;

#[cfg(feature = "derive")]
mod derive;
#[cfg(feature = "derive")]
pub use derive::*;

mod error;
pub use error::*;
