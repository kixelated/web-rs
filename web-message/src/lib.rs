//! A crate for sending and receiving messages via `postMessage`.
//!
//! Any type that implements [Message] can be serialized and unserialized.
//! Unlike using Serde for JSON encoding, this approach preserves [Transferable Objects](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects) and can avoid expensive allocations and copying.
//! Unlike using #[wasm-bindgen], this approach works outside of the `wasm-bindgen` ABI, supporting more types (ex. named enum variants).
//!
//! For example, the main thread can send a [js_sys::ArrayBuffer] or a Web Worker without copying the data.
//! If the WASM worker only needs to process a few header bytes, it can use the [js_sys::ArrayBuffer] instead of copying into a [Vec<u8>].
//! The resulting bytes can then be passed to [VideoDecoder](https://developer.mozilla.org/en-US/docs/Web/API/VideoDecoder) and the resulting [VideoFrame](https://developer.mozilla.org/en-US/docs/Web/API/VideoFrame) (transferable) can be posted back to the main thread.
//! You can even pass around a [web_sys::MessagePort]!
//!
//! This crate is designed to be used in conjunction with the `web-message-derive` crate.
//! We currently attempt parity with [ts-rs](https://docs.rs/ts-rs/latest/ts_rs/) so the resulting types can use `postMessage` directly from Typescript.
//!
//! ```rs
//! // NOTE: This is not possible with `wasm-bindgen` or `wasm-bindgen-serde`
//! #[derive(Message)]
//! #[msg(tag = "command")]
//! enum Command {
//!     Connect {
//!         url: String,
//!     },
//!     Frame {
//!         keyframe: bool,
//!         payload: js_sys::ArrayBuffer,
//!     },
//!     Close,
//! }
//! ```
//!
//! Some transferable types are gated behind feature flags:
//!

// Required for derive to work.
extern crate self as web_message;

#[cfg(feature = "derive")]
mod derive;
#[cfg(feature = "derive")]
pub use derive::*;

mod error;
mod message;

pub use error::*;
pub use message::*;
