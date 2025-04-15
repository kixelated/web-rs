use bytes::{Bytes, BytesMut};
use tokio::sync::{mpsc, watch};
use wasm_bindgen::prelude::*;

use super::AudioData;
use crate::{EncodedFrame, Error};

#[derive(Debug, Default, Clone)]
pub struct AudioDecoderConfig {
	/// The codec mimetype string.
	pub codec: String,

	/// Some codec formats use a description to configure the decoder.
	pub description: Option<Bytes>,

	/// The number of channels in the audio.
	pub channel_count: u32,

	/// The sample rate of the audio.
	pub sample_rate: u32,
}

impl AudioDecoderConfig {
	pub fn new<T: Into<String>>(codec: T, channel_count: u32, sample_rate: u32) -> Self {
		Self {
			codec: codec.into(),
			channel_count,
			sample_rate,
			..Default::default()
		}
	}

	/// Check if the configuration is supported by this browser.
	/// Returns an error if the configuration is invalid, and false if just unsupported.
	pub async fn is_supported(&self) -> Result<bool, Error> {
		let res =
			wasm_bindgen_futures::JsFuture::from(web_sys::AudioDecoder::is_config_supported(&self.into())).await?;

		let supported = js_sys::Reflect::get(&res, &JsValue::from_str("supported"))
			.unwrap()
			.as_bool()
			.unwrap();

		Ok(supported)
	}

	pub fn build(self) -> Result<(AudioDecoder, AudioDecoded), Error> {
		let (frames_tx, frames_rx) = mpsc::unbounded_channel();
		let (closed_tx, closed_rx) = watch::channel(Ok(()));
		let closed_tx2 = closed_tx.clone();

		let on_error = Closure::wrap(Box::new(move |e: JsValue| {
			closed_tx.send_replace(Err(Error::from(e))).ok();
		}) as Box<dyn FnMut(_)>);

		let on_frame = Closure::wrap(Box::new(move |e: JsValue| {
			let frame: web_sys::AudioData = e.unchecked_into();
			let frame = AudioData::from(frame);

			if frames_tx.send(frame).is_err() {
				closed_tx2.send_replace(Err(Error::Dropped)).ok();
			}
		}) as Box<dyn FnMut(_)>);

		let init = web_sys::AudioDecoderInit::new(on_error.as_ref().unchecked_ref(), on_frame.as_ref().unchecked_ref());
		let inner: web_sys::AudioDecoder = web_sys::AudioDecoder::new(&init).unwrap();
		inner.configure(&(&self).into())?;

		let decoder = AudioDecoder {
			inner,
			on_error,
			on_frame,
		};

		let decoded = AudioDecoded {
			frames: frames_rx,
			closed: closed_rx,
		};

		Ok((decoder, decoded))
	}
}

impl From<&AudioDecoderConfig> for web_sys::AudioDecoderConfig {
	fn from(this: &AudioDecoderConfig) -> Self {
		let config = web_sys::AudioDecoderConfig::new(&this.codec, this.channel_count, this.sample_rate);

		if let Some(description) = &this.description {
			config.set_description(&js_sys::Uint8Array::from(description.as_ref()));
		}

		config
	}
}

impl From<web_sys::AudioDecoderConfig> for AudioDecoderConfig {
	fn from(this: web_sys::AudioDecoderConfig) -> Self {
		let description = this.get_description().map(|d| {
			// TODO: An ArrayBuffer, a TypedArray, or a DataView containing a sequence of codec-specific bytes, commonly known as "extradata".
			let buffer = js_sys::Uint8Array::new(&d);
			let size = buffer.byte_length() as usize;

			let mut payload = BytesMut::with_capacity(size);
			payload.resize(size, 0);
			buffer.copy_to(&mut payload);

			payload.freeze()
		});

		let channels = this.get_number_of_channels();
		let sample_rate = this.get_sample_rate();

		Self {
			codec: this.get_codec(),
			description,
			channel_count: channels,
			sample_rate,
		}
	}
}

pub struct AudioDecoder {
	inner: web_sys::AudioDecoder,

	// These are held to avoid dropping them.
	#[allow(dead_code)]
	on_error: Closure<dyn FnMut(JsValue)>,
	#[allow(dead_code)]
	on_frame: Closure<dyn FnMut(JsValue)>,
}

impl AudioDecoder {
	pub fn decode(&self, frame: EncodedFrame) -> Result<(), Error> {
		let chunk_type = match frame.keyframe {
			true => web_sys::EncodedAudioChunkType::Key,
			false => web_sys::EncodedAudioChunkType::Delta,
		};

		let chunk = web_sys::EncodedAudioChunkInit::new(
			&js_sys::Uint8Array::from(frame.payload.as_ref()),
			frame.timestamp.as_micros() as _,
			chunk_type,
		);

		let chunk = web_sys::EncodedAudioChunk::new(&chunk)?;
		self.inner.decode(&chunk)?;

		Ok(())
	}

	pub async fn flush(&self) -> Result<(), Error> {
		wasm_bindgen_futures::JsFuture::from(self.inner.flush()).await?;
		Ok(())
	}

	pub fn queue_size(&self) -> u32 {
		self.inner.decode_queue_size()
	}
}

impl Drop for AudioDecoder {
	fn drop(&mut self) {
		let _ = self.inner.close();
	}
}

pub struct AudioDecoded {
	frames: mpsc::UnboundedReceiver<AudioData>,
	closed: watch::Receiver<Result<(), Error>>,
}

impl AudioDecoded {
	pub async fn next(&mut self) -> Result<Option<AudioData>, Error> {
		tokio::select! {
			biased;
			frame = self.frames.recv() => Ok(frame),
			Ok(()) = self.closed.changed() => Err(self.closed.borrow().clone().err().unwrap()),
		}
	}
}
