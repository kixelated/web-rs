use std::future::Future;
use std::pin::Pin;

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
/// A trait that is `Send` on native/WASI targets and empty in browser WASM.
pub trait MaybeSend: Send {}

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
impl<T: Send> MaybeSend for T {}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
/// A trait that is `Send` on native/WASI targets and empty in browser WASM.
pub trait MaybeSend {}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
impl<T> MaybeSend for T {}

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
/// A trait that is `Sync` on native/WASI targets and empty in browser WASM.
pub trait MaybeSync: Sync {}

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
impl<T: Sync> MaybeSync for T {}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
/// A trait that is `Sync` on native/WASI targets and empty in browser WASM.
pub trait MaybeSync {}

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
impl<T> MaybeSync for T {}

#[cfg(any(not(target_arch = "wasm32"), target_os = "wasi"))]
/// A boxed future that is `Send` on native/WASI targets and local in browser WASM.
pub type MaybeSendBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
/// A boxed future that is `Send` on native/WASI targets and local in browser WASM.
pub type MaybeSendBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// Extension helpers for boxing futures with target-appropriate `Send` bounds.
pub trait MaybeFutureExt: Future + Sized {
	/// Box this future with target-appropriate `Send` bounds.
	fn maybe_boxed<'a>(self) -> MaybeSendBoxFuture<'a, Self::Output>
	where
		Self: MaybeSend + 'a,
	{
		Box::pin(self)
	}
}

impl<F: Future> MaybeFutureExt for F {}
