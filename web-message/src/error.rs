#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("missing '{0}' field")]
	MissingField(&'static str),

	#[error("invalid '{0}' field")]
	InvalidField(&'static str),

	#[error("expected object with a single (string) key")]
	ExpectedUnitObject,

	#[error("expected null")]
	ExpectedNull,

	#[error("invalid type: {0}")]
	InvalidType(&'static str),

	#[error("expected string")]
	ExpectedString,

	#[error("unknown tag: {0}")]
	UnknownTag(String),

	#[cfg(feature = "url")]
	#[error("invalid URL: {0}")]
	InvalidUrl(url::ParseError),
}
