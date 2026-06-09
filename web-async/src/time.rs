//! Portable async time: `tokio::time` on native, [`wasmtimer`] in the browser.
//!
//! Code that sleeps, ticks on an interval, or reads an `Instant` needs to work
//! on both targets. Native tokio's clock is `std::time::Instant::now()`, which
//! panics on `wasm32-unknown-unknown` (no clock), and its timers need a runtime
//! time driver that `wasm_bindgen_futures::spawn_local` doesn't provide. So in
//! the browser we route through `wasmtimer`, which implements the same API on
//! `performance.now()` + `setTimeout`. Use `web_async::time` in place of
//! `tokio::time` and the same code runs on both.
//!
//! The surface mirrors `tokio::time` 1:1. The mockable test clock
//! (`tokio::time::pause`/`advance`) is intentionally not re-exported: it lives
//! behind tokio's `test-util` feature, which must not leak into normal builds,
//! and it only ever runs in native tests anyway.

pub use std::time::Duration;

// wasi has a real clock and tokio support, so it goes with native (matching the
// cfg split in `spawn`).
#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
pub use tokio::time::{
	interval, interval_at, sleep, sleep_until, timeout, timeout_at, Instant, Interval, MissedTickBehavior, Sleep,
	Timeout,
};

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
pub use wasmtimer::{
	std::Instant,
	tokio::{
		interval, interval_at, sleep, sleep_until, timeout, timeout_at, Interval, MissedTickBehavior, Sleep, Timeout,
	},
};
