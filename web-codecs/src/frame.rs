use bytes::{Bytes, BytesMut};

pub struct EncodedFrame {
    pub payload: Bytes,
    pub timestamp: f64,
    pub keyframe: bool,
}

impl From<web_sys::EncodedVideoChunk> for EncodedFrame {
    fn from(chunk: web_sys::EncodedVideoChunk) -> Self {
        let size = chunk.byte_length() as usize;

        let mut payload = BytesMut::with_capacity(size);
        payload.resize(size, 0);
        chunk.copy_to_with_u8_slice(&mut payload).unwrap();

        Self {
            payload: payload.freeze(),
            timestamp: chunk.timestamp(),
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
            timestamp: chunk.timestamp(),
            keyframe: chunk.type_() == web_sys::EncodedAudioChunkType::Key,
        }
    }
}
