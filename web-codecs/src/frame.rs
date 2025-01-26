use std::fmt;

use bytes::{Bytes, BytesMut};

use crate::Timestamp;

pub struct EncodedFrame {
	pub payload: Bytes,
	pub timestamp: Timestamp,
	pub keyframe: bool,
}

impl fmt::Debug for EncodedFrame {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("EncodedFrame")
			.field("payload", &self.payload.len())
			.field("timestamp", &self.timestamp)
			.field("keyframe", &self.keyframe)
			.finish()
	}
}

impl From<web_sys::EncodedVideoChunk> for EncodedFrame {
	fn from(chunk: web_sys::EncodedVideoChunk) -> Self {
		let size = chunk.byte_length() as usize;

		let mut payload = BytesMut::with_capacity(size);
		payload.resize(size, 0);
		chunk.copy_to_with_u8_slice(&mut payload).unwrap();

		Self {
			payload: payload.freeze(),
			timestamp: Timestamp::from_micros(chunk.timestamp() as _),
			keyframe: chunk.type_() == web_sys::EncodedVideoChunkType::Key,
		}
	}
}

impl From<web_sys::EncodedAudioChunk> for EncodedFrame {
	fn from(chunk: web_sys::EncodedAudioChunk) -> Self {
		let size = chunk.byte_length() as usize;

		let mut payload = BytesMut::with_capacity(size);
		payload.resize(size, 0);
		chunk.copy_to_with_u8_slice(&mut payload).unwrap();

		Self {
			payload: payload.freeze(),
			timestamp: Timestamp::from_micros(chunk.timestamp() as _),
			keyframe: chunk.type_() == web_sys::EncodedAudioChunkType::Key,
		}
	}
}
