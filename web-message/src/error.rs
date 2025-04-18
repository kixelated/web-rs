#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("missing '{0}' field")]
	MissingField(&'static str),

	#[error("invalid '{0}' field")]
	InvalidField(&'static str),

	#[error("unexpected length")]
	UnexpectedLength,

	#[error("unexpected type")]
	UnexpectedType,

	#[error("unknown tag")]
	UnknownTag,

	#[cfg(feature = "url")]
	#[error("invalid URL: {0}")]
	InvalidUrl(url::ParseError),
}
