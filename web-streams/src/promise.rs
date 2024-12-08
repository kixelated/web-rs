use wasm_bindgen::prelude::*;
use web_sys::js_sys;

/// A helper to ignore the result of a promise.
pub(crate) trait PromiseExt {
	fn ignore(self);
}

impl PromiseExt for js_sys::Promise {
	// Ignore the result of the promise by using an empty catch.
	fn ignore(self) {
		let closure = Closure::wrap(Box::new(|_: JsValue| {}) as Box<dyn FnMut(JsValue)>);
		let _ = self.catch(&closure);
		closure.forget();
	}
}
