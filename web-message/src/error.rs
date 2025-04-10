#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("missing '{0}' tag")]
	MissingTag(&'static str),

	#[error("invalid '{0}' tag")]
	InvalidTag(&'static str),

	#[error("unknown tag: {0}")]
	UnknownTag(&'static str),

	#[error("missing '{0}' field")]
	MissingField(&'static str),

	#[error("invalid '{0}' field")]
	InvalidField(&'static str),

	#[error("expected object")]
	ExpectedObject,

	#[error("invalid type: {0}")]
	InvalidType(&'static str),
}
