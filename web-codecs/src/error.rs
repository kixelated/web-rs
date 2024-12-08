use wasm_bindgen::prelude::*;

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
	#[error("dropped")]
	Dropped,

	#[error("invalid dimensions")]
	InvalidDimensions,

	#[error("unknown error: {0:?}")]
	Unknown(JsValue),
}

impl From<JsValue> for Error {
	fn from(e: JsValue) -> Self {
		Self::Unknown(e)
	}
}

pub type Result<T> = std::result::Result<T, Error>;
