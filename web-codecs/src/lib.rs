//! WebCodecs API bindings for Rust.
mod audio;
mod error;
mod frame;
mod video;

pub use error::*;
pub use frame::*;
pub use video::*;

pub type Timestamp = std::time::Duration;
