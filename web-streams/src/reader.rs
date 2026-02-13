use std::marker::PhantomData;

use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{js_sys, ReadableStream, ReadableStreamDefaultReader, ReadableStreamReadResult};

use crate::{Error, PromiseExt};

/// A wrapper around ReadableStream
pub struct Reader<T: JsCast> {
	inner: ReadableStreamDefaultReader,

	// Keep the most recent promise to make `read` cancelable
	read: Option<JsFuture>,

	_phantom: PhantomData<T>,
}

impl<T: JsCast> Reader<T> {
	/// Grab a lock on the given readable stream until dropped.
	pub fn new(stream: &ReadableStream) -> Result<Self, Error> {
		let inner = stream.get_reader().unchecked_into();
		Ok(Self {
			inner,
			read: None,
			_phantom: PhantomData,
		})
	}

	/// Read the next element from the stream, returning None if the stream is done.
	pub async fn read(&mut self) -> Result<Option<T>, Error> {
		if self.read.is_none() {
			self.read = Some(JsFuture::from(self.inner.read()));
		}

		let result: ReadableStreamReadResult = self.read.as_mut().unwrap().await?.into();
		self.read.take(); // Clear the promise on success

		//todo why do you use `Reflect` here?
		// is get_done and get_value not good enough?
		if Reflect::get(&result, &"done".into())?.is_truthy() {
			return Ok(None);
		}

		let res = Reflect::get(&result, &"value".into())?.unchecked_into();

		Ok(Some(res))
	}

	/// Abort the stream early with the given reason.
	pub fn abort(&mut self, reason: &str) {
		let str = JsValue::from_str(reason);
		self.inner.cancel_with_reason(&str).ignore();
	}

	pub async fn closed(&self) -> Result<(), Error> {
		JsFuture::from(self.inner.closed()).await?;
		Ok(())
	}
}

impl<T: JsCast> Drop for Reader<T> {
	/// Release the lock
	fn drop(&mut self) {
		self.inner.release_lock();
	}
}


use wasm_bindgen::JsCast;

#[cfg(feature = "tokio")]
mod tokio_impl {
	use std::io::{Result, Error, ErrorKind, ErrorKind::Unsupported};
	use super::*;
	use std::pin::Pin;
	use std::task::{Context, Poll};
	use tokio::io::{AsyncRead, ReadBuf};
	use wasm_bindgen::JsCast;
	use crate::reader::js_sys::Uint8Array;
	use std::future::Future;
	use Poll::{Pending, Ready};
	use js_sys::Promise;
	use ErrorKind::Other;
	use tracing::info;

	impl AsyncRead for Reader<Uint8Array> {

		fn poll_read(
			mut self: Pin<&mut Self>,
			cx: &mut Context<'_>,
			buf: &mut ReadBuf<'_>,
		) -> Poll<Result<()>> {

			//if there is no pending read, we need to create one
			if self.read.is_none() {
				self.read = Some(JsFuture::from(self.inner.read()));
			}

			let Some(promise) =  self.read.as_mut() else {
				return Ready(Err(Error::new(Other, "Unrecoverable error: No pending read found despite just queued")));
			};

			match Pin::new(promise).poll(cx) {
				Pending => Pending,
				Ready(Ok(js_val)) => {
					//we clone, set and then take here because
					//in case of pending it needs to be in read
					self.read.take();

					//it seems at the moment that the value cannot be typechecked?
					let result = js_val.unchecked_into::<ReadableStreamReadResult>();
					/*let Ok(result) = js_val.dyn_into::<ReadableStreamReadResult>() else {
						return Ready(Err(Error::new(Unsupported, "Unrecoverable error: Expected js type ReadableStreamReadResult")));
					};*/
					if result.get_done().unwrap_or(false) {
						return Ready(Ok(())); // EOF
					}

					let Ok(array) = result.get_value().dyn_into::<Uint8Array>() else {
						return Ready(Err(Error::new(Unsupported, "Unrecoverable error: Expected js type Uint8Array")));
					};
					let array_len = array.length() as usize;
					let len = std::cmp::min(buf.remaining(), array_len);

					// Copy what fits
					// # Safety: copy_to_uninit does not uninit anything and inits the first `len` bytes.
					let dst = unsafe {
						&mut buf.unfilled_mut()[0..len]
					};
					array.slice(0, len as u32).copy_to_uninit(dst);
					unsafe { buf.assume_init(len); }
					buf.advance(len);

					// If there are leftover bytes, we must not drop them
					// create a new ReadableStreamReadResult and set self.read
					if len < array_len {
						let leftover = array.slice(len as u32, array_len as u32);
						//let result = ReadableStreamReadResult::new(); i believe we can reuse the existing one
						result.set_done(false);
						result.set_value(&**leftover);
						let promise = Promise::resolve(&**result);
						self.read = Some(JsFuture::from(promise));
					}

					Ready(Ok(()))
				}
				Ready(Err(_)) => {
					self.read.take();
					Ready(Err(Error::new(Other, "js read error")))
				}
			}
		}
	}
}

