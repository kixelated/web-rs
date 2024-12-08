use bytes::{Bytes, BytesMut};
use tokio::sync::{mpsc, watch};
use wasm_bindgen::prelude::*;

use super::{Dimensions, VideoColorSpaceConfig, VideoFrame};
use crate::{EncodedFrame, Error};

#[derive(Debug, Default, Clone)]
pub struct VideoDecoderConfig {
	/// The codec mimetype string.
	pub codec: String,

	/// The resolution of the media.
	/// Neither width nor height can be zero.
	pub resolution: Option<Dimensions>,

	/// The resolution that the media should be displayed at.
	/// Neither width nor height can be zero.
	pub display: Option<Dimensions>,

	/// Color stuff.
	pub color_space: Option<VideoColorSpaceConfig>,

	/// Some codec formats use a description to configure the decoder.
	/// ex. For h264:
	///   - If present: AVC format, with the SPS/PPS in this description.
	///   - If absent: Annex-B format, with the SPS/PPS before each keyframe.
	pub description: Option<Bytes>,

	/// Optionally require or disable hardware acceleration.
	pub hardware_acceleration: Option<bool>,

	/// Optionally optimize for latency.
	pub latency_optimized: Option<bool>,
}

impl VideoDecoderConfig {
	pub fn new<T: Into<String>>(codec: T) -> Self {
		Self {
			codec: codec.into(),
			..Default::default()
		}
	}

	/// Check if the configuration is supported by this browser.
	/// Returns an error if the configuration is invalid, and false if just unsupported.
	pub async fn is_supported(&self) -> Result<bool, Error> {
		let res =
			wasm_bindgen_futures::JsFuture::from(web_sys::VideoDecoder::is_config_supported(&self.into())).await?;

		let supported = js_sys::Reflect::get(&res, &JsValue::from_str("supported"))
			.unwrap()
			.as_bool()
			.unwrap();

		Ok(supported)
	}

	pub fn is_valid(&self) -> Result<(), Error> {
		if self.resolution.map_or(true, |d| d.width == 0 || d.height == 0) {
			return Err(Error::InvalidDimensions);
		}

		if self.display.map_or(true, |d| d.width == 0 || d.height == 0) {
			return Err(Error::InvalidDimensions);
		}

		Ok(())
	}

	pub fn build(self) -> Result<(VideoDecoder, VideoDecoded), Error> {
		let (frames_tx, frames_rx) = mpsc::unbounded_channel();
		let (closed_tx, closed_rx) = watch::channel(Ok(()));
		let closed_tx2 = closed_tx.clone();

		let on_error = Closure::wrap(Box::new(move |e: JsValue| {
			closed_tx.send_replace(Err(Error::from(e))).ok();
		}) as Box<dyn FnMut(_)>);

		let on_frame = Closure::wrap(Box::new(move |e: JsValue| {
			let frame: web_sys::VideoFrame = e.unchecked_into();
			let frame = VideoFrame::from(frame);

			if frames_tx.send(frame).is_err() {
				closed_tx2.send_replace(Err(Error::Dropped)).ok();
			}
		}) as Box<dyn FnMut(_)>);

		let init = web_sys::VideoDecoderInit::new(on_error.as_ref().unchecked_ref(), on_frame.as_ref().unchecked_ref());
		let inner: web_sys::VideoDecoder = web_sys::VideoDecoder::new(&init).unwrap();
		inner.configure(&(&self).into())?;

		let decoder = VideoDecoder {
			inner,
			on_error,
			on_frame,
		};

		let decoded = VideoDecoded {
			frames: frames_rx,
			closed: closed_rx,
		};

		Ok((decoder, decoded))
	}
}

impl From<&VideoDecoderConfig> for web_sys::VideoDecoderConfig {
	fn from(this: &VideoDecoderConfig) -> Self {
		let config = web_sys::VideoDecoderConfig::new(&this.codec);

		if let Some(Dimensions { width, height }) = this.resolution {
			config.set_coded_width(width);
			config.set_coded_height(height);
		}

		if let Some(Dimensions { width, height }) = this.display {
			config.set_display_aspect_height(height);
			config.set_display_aspect_width(width);
		}

		if let Some(description) = &this.description {
			config.set_description(&js_sys::Uint8Array::from(description.as_ref()));
		}

		if let Some(color_space) = &this.color_space {
			config.set_color_space(&color_space.into());
		}

		if let Some(preferred) = this.hardware_acceleration {
			config.set_hardware_acceleration(match preferred {
				true => web_sys::HardwareAcceleration::PreferHardware,
				false => web_sys::HardwareAcceleration::PreferSoftware,
			});
		}

		if let Some(value) = this.latency_optimized {
			config.set_optimize_for_latency(value);
		}

		config
	}
}

impl From<web_sys::VideoDecoderConfig> for VideoDecoderConfig {
	fn from(this: web_sys::VideoDecoderConfig) -> Self {
		let resolution = match (this.get_coded_width(), this.get_coded_height()) {
			(Some(width), Some(height)) if width != 0 && height != 0 => Some(Dimensions { width, height }),
			_ => None,
		};

		let display = match (this.get_display_aspect_width(), this.get_display_aspect_height()) {
			(Some(width), Some(height)) if width != 0 && height != 0 => Some(Dimensions { width, height }),
			_ => None,
		};

		let color_space = this.get_color_space().map(VideoColorSpaceConfig::from);

		let description = this.get_description().map(|d| {
			// TODO: An ArrayBuffer, a TypedArray, or a DataView containing a sequence of codec-specific bytes, commonly known as "extradata".
			let buffer = js_sys::Uint8Array::new(&d);
			let size = buffer.byte_length() as usize;

			let mut payload = BytesMut::with_capacity(size);
			payload.resize(size, 0);
			buffer.copy_to(&mut payload);

			payload.freeze()
		});

		let hardware_acceleration = match this.get_hardware_acceleration() {
			Some(web_sys::HardwareAcceleration::PreferHardware) => Some(true),
			Some(web_sys::HardwareAcceleration::PreferSoftware) => Some(false),
			_ => None,
		};

		let latency_optimized = this.get_optimize_for_latency();

		Self {
			codec: this.get_codec(),
			resolution,
			display,
			color_space,
			description,
			hardware_acceleration,
			latency_optimized,
		}
	}
}

pub struct VideoDecoder {
	inner: web_sys::VideoDecoder,

	// These are held to avoid dropping them.
	#[allow(dead_code)]
	on_error: Closure<dyn FnMut(JsValue)>,
	#[allow(dead_code)]
	on_frame: Closure<dyn FnMut(JsValue)>,
}

impl VideoDecoder {
	pub fn decode(&self, frame: EncodedFrame) -> Result<(), Error> {
		let chunk_type = match frame.keyframe {
			true => web_sys::EncodedVideoChunkType::Key,
			false => web_sys::EncodedVideoChunkType::Delta,
		};

		let chunk = web_sys::EncodedVideoChunkInit::new(
			&js_sys::Uint8Array::from(frame.payload.as_ref()),
			frame.timestamp,
			chunk_type,
		);

		let chunk = web_sys::EncodedVideoChunk::new(&chunk)?;
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

impl Drop for VideoDecoder {
	fn drop(&mut self) {
		let _ = self.inner.close();
	}
}

pub struct VideoDecoded {
	frames: mpsc::UnboundedReceiver<VideoFrame>,
	closed: watch::Receiver<Result<(), Error>>,
}

impl VideoDecoded {
	pub async fn next(&mut self) -> Result<Option<VideoFrame>, Error> {
		tokio::select! {
			biased;
			frame = self.frames.recv() => Ok(frame),
			Ok(()) = self.closed.changed() => Err(self.closed.borrow().clone().err().unwrap()),
		}
	}
}
