use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{js_sys, JsFuture};
use web_sys::{WritableStream, WritableStreamDefaultWriter};

use crate::{Error, PromiseExt};

// Wrapper around WritableStream
pub struct Writer {
	inner: WritableStreamDefaultWriter,
}

impl Writer {
	pub fn new(stream: &WritableStream) -> Result<Self, Error> {
		let inner = stream.get_writer()?.unchecked_into();
		Ok(Self { inner })
	}

	pub async fn write(&mut self, v: &JsValue) -> Result<(), Error> {
		JsFuture::from(self.inner.write_with_chunk(v)).await?;
		Ok(())
	}

	pub fn close(&mut self) {
		self.inner.close().ignore();
	}

	pub fn abort(&mut self, reason: &str) {
		let str = JsValue::from_str(reason);
		self.inner.abort_with_reason(&str).ignore();
	}

	pub async fn closed(&self) -> Result<(), Error> {
		JsFuture::from(self.inner.closed()).await?;
		Ok(())
	}
}

impl Drop for Writer {
	fn drop(&mut self) {
		self.inner.release_lock();
	}
}

impl<T: JsCast> From<Writer> for TypedWriter<T> {
	fn from(value: Writer) -> Self {
		let value: ManuallyDrop<Writer> = ManuallyDrop::new(value);

		TypedWriter {
			inner: value.inner.clone(),
			write_promise: None,
			_phantom: PhantomData,
		}
	}
}

impl<T: JsCast> Drop for TypedWriter<T> {
	fn drop(&mut self) {
		self.inner.release_lock();
	}
}

impl<T: JsCast> TryFrom<TypedWriter<T>> for Writer {
	type Error = TypedWriter<T>;

	fn try_from(value: TypedWriter<T>) -> Result<Self, Self::Error> {
		if value.write_promise.is_some() {
			Err(value)
		} else {
			let value: ManuallyDrop<TypedWriter<T>> = ManuallyDrop::new(value);
			Ok(Writer {
				inner: value.inner.clone(),
			})
		}

	}
}


pub struct TypedWriter<T: JsCast> {
	inner: WritableStreamDefaultWriter,
	// Keep the most recent promise to make `write` cancelable
	write_promise: Option<JsFuture>,

	_phantom: PhantomData<T>,
}

impl<T: JsCast> TypedWriter<T> {
    pub fn new(stream: &WritableStream) -> Result<Self, Error> {
        let inner = stream.get_writer()?.unchecked_into();
        Ok(Self {
            inner,
            write_promise: None,
            _phantom: PhantomData,
        })
    }

    pub async fn write(&mut self, v: &T) -> Result<(), Error> {
		if let Some(promise) = &mut  self.write_promise.take() {
            promise.await?;
		}
        let js_value = JsValue::from(v);
        self.write_promise = Some(JsFuture::from(self.inner.write_with_chunk(&js_value)));
        if let Some(promise) = &mut self.write_promise {
            promise.await?;
            self.write_promise = None;
        }
        Ok(())
    }

    pub fn close(&mut self) {
		/*if let Some(promise) = self.write_promise.take() {
			promise.block;
		}*/
        self.inner.close().ignore();
    }

    pub fn abort(&mut self, reason: &str) {
		/*if let Some(promise) = self.write_promise.take() {
			promise.ignore();
		}*/
        let str = JsValue::from_str(reason);
        self.inner.abort_with_reason(&str).ignore();
    }

    /// Wait for the stream to be closed
    pub async fn closed(&self) -> Result<(), Error> {
		//todo is it correct that this only requires &self?
		/*if let Some(promise) = &self.write_promise {
            promise.await?;
		}*/

        JsFuture::from(self.inner.closed()).await?;
        Ok(())
    }
}

#[cfg(feature = "tokio")]
mod tokio_impl {
	use std::future::Future;
	use super::*;
    use std::io::{Result, Error, ErrorKind};
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::AsyncWrite;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use js_sys::Uint8Array;
    use ErrorKind::{BrokenPipe, Other};
    use std::task::Poll::Ready;
    use Poll::Pending;
    use tracing::info;

    impl<T: JsCast + Unpin> TypedWriter<T> {
		fn project(self: Pin<&mut Self>) -> (&mut WritableStreamDefaultWriter, &mut Option<JsFuture>) {
			// Safety: None of the fields are self-referential or require pinning
			let this = self.get_mut();
			(&mut this.inner, &mut this.write_promise)
		}
	}

    impl AsyncWrite for TypedWriter<Uint8Array> {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            info!("poll_write called with buf{{len={}}}: {:?}", buf.len(), buf);
            
            let Ok(Some(desired_size)) = self.inner.desired_size() else {
                return Ready(Err(Error::new(BrokenPipe, "stream is closed, not writable, or abort queued")));
            };
            
            let (inner, write_promise) = Self::project(self);
            info!("desired size: {}", desired_size);
            if desired_size < 1f64 {
                // if we return Pending here we must also ensure a waker is provided
                return if let Some(promise) = write_promise {
                    match Pin::new(promise).poll(cx) {
                        Pending => Pending,
                        Ready(Ok(_)) => {
                            *write_promise = None;
                            Ready(Ok(0))
                        },
                        Ready(Err(err)) => {
                            *write_promise = None;
                            let js_err_str = err.as_string().unwrap_or_else(|| "unknown error".to_string());
                            Ready(Err(Error::new(Other, format!("js wait for write error: {}", js_err_str))))
                        },
                    }
                } else {
                    Ready(Ok(0)) // No pending write, nothing to flush
                };

                //return Ready(Err(Error::from(WouldBlock)));
                //return Ready(Err(Error::new(WouldBlock, format!("desired size is too small: {}", desired_size))));
            }
            //let desired_size = desired_size as usize;
            if let Some(promise) = write_promise {
                if let Ready(Err(err)) = Pin::new(promise).poll(cx) {
                    *write_promise = None;
                    let js_err_str = err.as_string().unwrap_or_else(|| "unknown error".to_string());
                    return Ready(Err(Error::new(Other, format!("js write error: {}", js_err_str))));
				}
            }

            //let len = std::cmp::min(buf.len(), desired_size);
            let array = Uint8Array::from(buf);//.slice(0, len as u32);
            //todo this looks like a proper issue to me!
            let p = JsFuture::from(inner.write_with_chunk(&array));
            *write_promise = Some(p); //this promise should only resolve after the current anyway
            /*match write_promise {
                Some(val) => {
                    *val = val.then(&Closure::<dyn FnMut(JsValue)>::new(move |_| {
                        p
                    }));
                },
                opt @ None => *opt = Some(p),
            }*/
            Ready(Ok(buf.len()))
        }

        fn poll_flush(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Result<()>> {
            let (_ , write_promise) = Self::project(self);
            if let Some(promise) = write_promise {
				match Pin::new(promise).poll(cx) {
                    Pending => Pending,
					Ready(Ok(_)) => {
						*write_promise = None;
                        Ready(Ok(()))
					},
					Ready(Err(err)) => {
                        *write_promise = None;
						let js_err_str = err.as_string().unwrap_or_else(|| "unknown error".to_string());
						Ready(Err(Error::new(Other, format!("js flush error: {}", js_err_str))))
					},
				}
			} else {
                Ready(Ok(())) // No pending write, nothing to flush
            }
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<()>> {
            let (inner, _) = Self::project(self);
            inner.close().ignore();
            let p = inner.closed();
            let mut js_future = JsFuture::from(p);
            match Pin::new(&mut js_future).poll(_cx) {
                Pending => Pending,
                Ready(Ok(_)) => Ready(Ok(())),
                Ready(Err(err)) => {
                    let js_err_str = err.as_string().unwrap_or_else(|| "unknown error".to_string());
                    Ready(Err(Error::new(Other, format!("js shutdown error: {}", js_err_str))))
                },
            }
        }
    }
}