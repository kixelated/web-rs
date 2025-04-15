use std::{cell::RefCell, rc::Rc, time::Duration};

use tokio::sync::{mpsc, watch};
use wasm_bindgen::prelude::*;

use crate::{EncodedFrame, Error, Timestamp};

use super::{Dimensions, VideoDecoderConfig, VideoFrame};

use derive_more::Display;

#[derive(Debug, Display, Clone, Copy)]
pub enum VideoBitrateMode {
	#[display("constant")]
	Constant,

	#[display("variable")]
	Variable,

	#[display("quantizer")]
	Quantizer,
}

#[derive(Debug, Default, Clone)]
pub struct VideoEncoderConfig {
	pub codec: String,
	pub resolution: Dimensions,
	pub display: Option<Dimensions>,
	pub hardware_acceleration: Option<bool>,
	pub latency_optimized: Option<bool>,
	pub bitrate: Option<u32>,          // bits per second
	pub framerate: Option<f64>,        // frames per second
	pub alpha_preserved: Option<bool>, // keep alpha channel
	pub scalability_mode: Option<String>,
	pub bitrate_mode: Option<VideoBitrateMode>,

	// NOTE: This is a custom configuration
	/// The maximum duration of a Group of Pictures (GOP) before forcing a new keyframe.
	pub max_gop_duration: Option<Duration>, // seconds
}

impl VideoEncoderConfig {
	pub fn new<T: Into<String>>(codec: T, resolution: Dimensions) -> Self {
		Self {
			codec: codec.into(),
			resolution,
			display: None,
			hardware_acceleration: None,
			latency_optimized: None,
			bitrate: None,
			framerate: None,
			alpha_preserved: None,
			scalability_mode: None,
			bitrate_mode: None,
			max_gop_duration: None,
		}
	}

	pub async fn is_supported(&self) -> Result<bool, Error> {
		let res =
			wasm_bindgen_futures::JsFuture::from(web_sys::VideoEncoder::is_config_supported(&self.into())).await?;

		let supported = js_sys::Reflect::get(&res, &JsValue::from_str("supported"))
			.unwrap()
			.as_bool()
			.unwrap();

		Ok(supported)
	}

	pub fn is_valid(&self) -> Result<(), Error> {
		if self.resolution.width == 0 || self.resolution.height == 0 {
			return Err(Error::InvalidDimensions);
		}

		if let Some(display) = self.display {
			if display.width == 0 || display.height == 0 {
				return Err(Error::InvalidDimensions);
			}
		}

		Ok(())
	}

	pub fn init(self) -> Result<(VideoEncoder, VideoEncoded), Error> {
		let (frames_tx, frames_rx) = mpsc::unbounded_channel();
		let (closed_tx, closed_rx) = watch::channel(Ok(()));
		let config = Rc::new(RefCell::new(None));

		let decoder = VideoEncoder::new(self, config.clone(), frames_tx, closed_tx)?;
		let decoded = VideoEncoded::new(config, frames_rx, closed_rx);

		Ok((decoder, decoded))
	}
}

impl From<&VideoEncoderConfig> for web_sys::VideoEncoderConfig {
	fn from(this: &VideoEncoderConfig) -> Self {
		let config = web_sys::VideoEncoderConfig::new(&this.codec, this.resolution.height, this.resolution.width);

		if let Some(Dimensions { width, height }) = this.display {
			config.set_display_height(height);
			config.set_display_width(width);
		}

		if let Some(preferred) = this.hardware_acceleration {
			config.set_hardware_acceleration(match preferred {
				true => web_sys::HardwareAcceleration::PreferHardware,
				false => web_sys::HardwareAcceleration::PreferSoftware,
			});
		}

		if let Some(value) = this.latency_optimized {
			config.set_latency_mode(match value {
				true => web_sys::LatencyMode::Realtime,
				false => web_sys::LatencyMode::Quality,
			});
		}

		if let Some(value) = this.bitrate {
			config.set_bitrate(value as f64);
		}

		if let Some(value) = this.framerate {
			config.set_framerate(value);
		}

		if let Some(value) = this.alpha_preserved {
			config.set_alpha(match value {
				true => web_sys::AlphaOption::Keep,
				false => web_sys::AlphaOption::Discard,
			});
		}

		if let Some(value) = &this.scalability_mode {
			config.set_scalability_mode(value);
		}

		if let Some(_value) = &this.bitrate_mode {
			// TODO not supported yet
		}

		config
	}
}

#[derive(Debug, Default)]
pub struct VideoEncodeOptions {
	// Force or deny a key frame.
	pub key_frame: Option<bool>,
	// TODO
	// pub quantizer: Option<u8>,
}

pub struct VideoEncoder {
	inner: web_sys::VideoEncoder,
	config: VideoEncoderConfig,

	last_keyframe: Rc<RefCell<Option<Timestamp>>>,

	// These are held to avoid dropping them.
	#[allow(dead_code)]
	on_error: Closure<dyn FnMut(JsValue)>,
	#[allow(dead_code)]
	on_frame: Closure<dyn FnMut(JsValue, JsValue)>,
}

impl VideoEncoder {
	fn new(
		config: VideoEncoderConfig,
		on_config: Rc<RefCell<Option<VideoDecoderConfig>>>,
		on_frame: mpsc::UnboundedSender<EncodedFrame>,
		on_error: watch::Sender<Result<(), Error>>,
	) -> Result<Self, Error> {
		let last_keyframe = Rc::new(RefCell::new(None));
		let last_keyframe2 = last_keyframe.clone();

		let on_error2 = on_error.clone();
		let on_error = Closure::wrap(Box::new(move |e: JsValue| {
			on_error.send_replace(Err(Error::from(e))).ok();
		}) as Box<dyn FnMut(_)>);

		let on_frame = Closure::wrap(Box::new(move |frame: JsValue, meta: JsValue| {
			// First parameter is the frame, second optional parameter is metadata.
			let frame: web_sys::EncodedVideoChunk = frame.unchecked_into();
			let frame = EncodedFrame::from(frame);

			if let Ok(metadata) = meta.dyn_into::<js_sys::Object>() {
				// TODO handle metadata
				if let Ok(config) = js_sys::Reflect::get(&metadata, &"decoderConfig".into()) {
					if !config.is_falsy() {
						let config: web_sys::VideoDecoderConfig = config.unchecked_into();
						let config = VideoDecoderConfig::from(config);
						on_config.borrow_mut().replace(config);
					}
				}
			}

			if frame.keyframe {
				let mut last_keyframe = last_keyframe2.borrow_mut();
				if frame.timestamp > last_keyframe.unwrap_or_default() {
					*last_keyframe = Some(frame.timestamp);
				}
			}

			if on_frame.send(frame).is_err() {
				on_error2.send_replace(Err(Error::Dropped)).ok();
			}
		}) as Box<dyn FnMut(_, _)>);

		let init = web_sys::VideoEncoderInit::new(on_error.as_ref().unchecked_ref(), on_frame.as_ref().unchecked_ref());
		let inner: web_sys::VideoEncoder = web_sys::VideoEncoder::new(&init).unwrap();
		inner.configure(&(&config).into())?;

		Ok(Self {
			config,
			inner,
			last_keyframe,
			on_error,
			on_frame,
		})
	}

	pub fn encode(&mut self, frame: &VideoFrame, options: VideoEncodeOptions) -> Result<(), Error> {
		let o = web_sys::VideoEncoderEncodeOptions::new();

		if let Some(key_frame) = options.key_frame {
			o.set_key_frame(key_frame);
		} else if let Some(max_gop_duration) = self.config.max_gop_duration {
			let timestamp = frame.timestamp();
			let mut last_keyframe = self.last_keyframe.borrow_mut();

			let duration = timestamp - last_keyframe.unwrap_or_default();
			if duration >= max_gop_duration {
				o.set_key_frame(true);
			}

			*last_keyframe = Some(timestamp);
		}

		self.inner.encode_with_options(frame.inner(), &o)?;

		Ok(())
	}

	pub fn queue_size(&self) -> u32 {
		self.inner.encode_queue_size()
	}

	pub fn config(&self) -> &VideoEncoderConfig {
		&self.config
	}

	pub async fn flush(&mut self) -> Result<(), Error> {
		wasm_bindgen_futures::JsFuture::from(self.inner.flush()).await?;
		Ok(())
	}
}

impl Drop for VideoEncoder {
	fn drop(&mut self) {
		let _ = self.inner.close();
	}
}

pub struct VideoEncoded {
	config: Rc<RefCell<Option<VideoDecoderConfig>>>,
	frames: mpsc::UnboundedReceiver<EncodedFrame>,
	closed: watch::Receiver<Result<(), Error>>,
}

impl VideoEncoded {
	fn new(
		config: Rc<RefCell<Option<VideoDecoderConfig>>>,
		frames: mpsc::UnboundedReceiver<EncodedFrame>,
		closed: watch::Receiver<Result<(), Error>>,
	) -> Self {
		Self { config, frames, closed }
	}

	pub async fn frame(&mut self) -> Result<Option<EncodedFrame>, Error> {
		tokio::select! {
			biased;
			frame = self.frames.recv() => Ok(frame),
			Ok(()) = self.closed.changed() => Err(self.closed.borrow().clone().err().unwrap()),
		}
	}

	/// Returns the decoder config, after the first frame has been encoded.
	pub fn config(&self) -> Option<VideoDecoderConfig> {
		self.config.borrow().clone()
	}
}
