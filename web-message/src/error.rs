#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("missing '{0}' field")]
	MissingTag(&'static str),

	#[error("invalid '{0}' field: {1:?}")]
	InvalidTag(&'static str, ::js_sys::wasm_bindgen::JsValue),

	#[error("unknown tag: {0} ({1:?})")]
	UnknownTag(&'static str, ::js_sys::wasm_bindgen::JsValue),

	#[error("missing '{0}' field")]
	MissingField(&'static str),

	#[error("invalid '{0}' field: {1:?}")]
	InvalidField(&'static str, ::js_sys::wasm_bindgen::JsValue),

	#[error("expected object: {0:?}")]
	ExpectedObject(::js_sys::wasm_bindgen::JsValue),
}
