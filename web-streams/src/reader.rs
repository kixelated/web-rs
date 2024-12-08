use std::marker::PhantomData;

use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{js_sys, ReadableStream, ReadableStreamDefaultReader, ReadableStreamReadResult};

use crate::{Error, PromiseExt};

/// A wrapper around ReadableStream
pub struct Reader<T: JsCast> {
	inner: ReadableStreamDefaultReader,
	_phantom: PhantomData<T>,
}

impl<T: JsCast> Reader<T> {
	/// Grab a lock on the given readable stream until dropped.
	pub fn new(stream: &ReadableStream) -> Result<Self, Error> {
		let inner = stream.get_reader().unchecked_into();
		Ok(Self {
			inner,
			_phantom: PhantomData,
		})
	}

	/// Read the next element from the stream, returning None if the stream is done.
	pub async fn read(&mut self) -> Result<Option<T>, Error> {
		let result: ReadableStreamReadResult = JsFuture::from(self.inner.read()).await?.into();

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
}

impl<T: JsCast> Drop for Reader<T> {
	/// Release the lock
	fn drop(&mut self) {
		self.inner.release_lock();
	}
}
