use js_sys::Array;
use js_sys::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::js_sys;

use crate::Error;
pub trait Message: Sized {
	// Serializes the message into a JsValue.
	// Any transferable fields are appended to the given array.
	fn into_message(self, transferable: &mut Array) -> JsValue;

	// Deserializes the message from a JsValue.
	fn from_message(message: JsValue) -> Result<Self, Error>;
}

macro_rules! upstream {
	($($t:ty),*) => {
		$(
			impl Message for $t {
				fn into_message(self, _transferable: &mut Array) -> JsValue {
					self.into()
				}

				fn from_message(message: JsValue) -> Result<Self, Error> {
					Self::try_from(message).map_err(|_| Error::InvalidType(stringify!($t)))
				}
			}
		)*
	};
}

// Macro for implementing Message for primitive casts supported in wasm-bindgen
upstream!(String, f64, i128, i64, u128, u64);

macro_rules! integer {
	($($t:ty),*) => {
		$(
			impl Message for $t {
				fn into_message(self, _transferable: &mut Array) -> JsValue {
					self.into()
				}

				fn from_message(message: JsValue) -> Result<Self, Error> {
					Ok(message.as_f64().ok_or(Error::InvalidType(stringify!($t)))? as $t)
				}
			}
		)*
	};
}

// Macro for implementing Message for floating point types, less than Number.MAX_SAFE_INTEGER
integer!(u32, i32, u16, i16, u8, i8);

impl Message for bool {
	fn into_message(self, _transferable: &mut Array) -> JsValue {
		self.into()
	}

	fn from_message(message: JsValue) -> Result<Self, Error> {
		message.as_bool().ok_or(Error::InvalidType("bool"))
	}
}

impl<T: Message> Message for Option<T> {
	fn into_message(self, transferable: &mut Array) -> JsValue {
		match self {
			Some(value) => value.into_message(transferable),
			None => JsValue::NULL,
		}
	}

	fn from_message(message: JsValue) -> Result<Self, Error> {
		Ok(match message.is_null() {
			true => None,
			false => Some(T::from_message(message)?),
		})
	}
}

impl<T: Message> Message for Vec<T> {
	fn into_message(self, transferable: &mut Array) -> JsValue {
		let array = Array::new();
		for value in self {
			array.push(&value.into_message(transferable));
		}
		array.into()
	}

	fn from_message(message: JsValue) -> Result<Self, Error> {
		if !message.is_array() {
			return Err(Error::InvalidType("Vec"));
		}

		let array = Array::from(&message);
		let mut values = Vec::with_capacity(array.length() as usize);
		for i in 0..array.length() {
			values.push(T::from_message(array.get(i))?);
		}
		Ok(values)
	}
}

impl Message for js_sys::ArrayBuffer {
	fn into_message(self, transferable: &mut Array) -> JsValue {
		transferable.push(&self);
		self.into()
	}

	fn from_message(message: JsValue) -> Result<Self, Error> {
		message
			.dyn_into::<js_sys::ArrayBuffer>()
			.map_err(|_| Error::InvalidType("ArrayBuffer"))
	}
}

macro_rules! transferable_feature {
	($($feature:literal = $t:ident),* $(,)?) => {
		$(
			#[cfg(feature = $feature)]
			impl Message for web_sys::$t {
				fn into_message(self, transferable: &mut Array) -> JsValue {
					transferable.push(&self);
					self.into()
				}

				fn from_message(message: JsValue) -> Result<Self, Error> {
					message
						.dyn_into::<web_sys::$t>()
						.map_err(|_| Error::InvalidType(stringify!($t)))
				}
			}
		)*
	};
}

// These feature names copy web_sys for all (currently) transferable types.
// https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Transferable_objects
transferable_feature!(
	"MessagePort" = MessagePort,
	"ReadableStream" = ReadableStream,
	"WritableStream" = WritableStream,
	"TransformStream" = TransformStream,
	"WebTransportReceiveStream" = WebTransportReceiveStream,
	"WebTransportSendStream" = WebTransportSendStream,
	"AudioData" = AudioData,
	"ImageBitmap" = ImageBitmap,
	"VideoFrame" = VideoFrame,
	"OffscreenCanvas" = OffscreenCanvas,
	"RtcDataChannel" = RtcDataChannel,
	//"MediaSourceHandle" = MediaSourceHandle,
	"MidiAccess" = MidiAccess,
);

#[cfg(feature = "url")]
impl Message for url::Url {
	fn into_message(self, _transferable: &mut Array) -> JsValue {
		self.to_string().into()
	}

	fn from_message(message: JsValue) -> Result<Self, Error> {
		let str = message.as_string().ok_or(Error::ExpectedString)?;
		url::Url::parse(&str).map_err(Error::InvalidUrl)
	}
}
