use wasm_bindgen::prelude::*;

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
	#[error("unknown error: {0:?}")]
	Unknown(JsValue),
}

impl From<JsValue> for Error {
	fn from(e: JsValue) -> Self {
		Self::Unknown(e)
	}
}
