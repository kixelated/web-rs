use std::{cell::RefCell, rc::Rc};

use tokio::sync::{mpsc, watch};
use wasm_bindgen::prelude::*;

use crate::{EncodedFrame, Error};

use super::{AudioData, AudioDecoderConfig};

// TODO support the full specification: https://developer.mozilla.org/en-US/docs/Web/API/AudioEncoder/configure
#[derive(Debug, Default, Clone)]
pub struct AudioEncoderConfig {
	pub codec: String,
	pub channel_count: Option<u32>,
	pub sample_rate: Option<u32>,
	pub bitrate: Option<u32>, // bits per second
}

impl AudioEncoderConfig {
	pub fn new<T: Into<String>>(codec: T) -> Self {
		Self {
			codec: codec.into(),
			channel_count: None,
			sample_rate: None,
			bitrate: None,
		}
	}

	pub async fn is_supported(&self) -> Result<bool, Error> {
		let res =
			wasm_bindgen_futures::JsFuture::from(web_sys::AudioEncoder::is_config_supported(&self.into())).await?;

		let supported = js_sys::Reflect::get(&res, &JsValue::from_str("supported"))
			.unwrap()
			.as_bool()
			.unwrap();

		Ok(supported)
	}

	pub fn init(self) -> Result<(AudioEncoder, AudioEncoded), Error> {
		let (frames_tx, frames_rx) = mpsc::unbounded_channel();
		let (closed_tx, closed_rx) = watch::channel(Ok(()));
		let config = Rc::new(RefCell::new(None));

		let decoder = AudioEncoder::new(self, config.clone(), frames_tx, closed_tx)?;
		let decoded = AudioEncoded::new(config, frames_rx, closed_rx);

		Ok((decoder, decoded))
	}
}

impl From<&AudioEncoderConfig> for web_sys::AudioEncoderConfig {
	fn from(this: &AudioEncoderConfig) -> Self {
		let config = web_sys::AudioEncoderConfig::new(&this.codec);

		if let Some(channels) = this.channel_count {
			config.set_number_of_channels(channels);
		}

		if let Some(sample_rate) = this.sample_rate {
			config.set_sample_rate(sample_rate);
		}

		if let Some(bit_rate) = this.bitrate {
			config.set_bitrate(bit_rate as f64);
		}

		config
	}
}

pub struct AudioEncoder {
	inner: web_sys::AudioEncoder,
	config: AudioEncoderConfig,

	// These are held to avoid dropping them.
	#[allow(dead_code)]
	on_error: Closure<dyn FnMut(JsValue)>,
	#[allow(dead_code)]
	on_frame: Closure<dyn FnMut(JsValue, JsValue)>,
}

impl AudioEncoder {
	fn new(
		config: AudioEncoderConfig,
		on_config: Rc<RefCell<Option<AudioDecoderConfig>>>,
		on_frame: mpsc::UnboundedSender<EncodedFrame>,
		on_error: watch::Sender<Result<(), Error>>,
	) -> Result<Self, Error> {
		let on_error2 = on_error.clone();
		let on_error = Closure::wrap(Box::new(move |e: JsValue| {
			on_error.send_replace(Err(Error::from(e))).ok();
		}) as Box<dyn FnMut(_)>);

		let on_frame = Closure::wrap(Box::new(move |frame: JsValue, meta: JsValue| {
			// First parameter is the frame, second optional parameter is metadata.
			let frame: web_sys::EncodedAudioChunk = frame.unchecked_into();
			let frame = EncodedFrame::from(frame);

			if let Ok(metadata) = meta.dyn_into::<js_sys::Object>() {
				// TODO handle metadata
				if let Ok(config) = js_sys::Reflect::get(&metadata, &"decoderConfig".into()) {
					if !config.is_falsy() {
						let config: web_sys::AudioDecoderConfig = config.unchecked_into();
						let config = AudioDecoderConfig::from(config);
						on_config.borrow_mut().replace(config);
					}
				}
			}

			if on_frame.send(frame).is_err() {
				on_error2.send_replace(Err(Error::Dropped)).ok();
			}
		}) as Box<dyn FnMut(_, _)>);

		let init = web_sys::AudioEncoderInit::new(on_error.as_ref().unchecked_ref(), on_frame.as_ref().unchecked_ref());
		let inner: web_sys::AudioEncoder = web_sys::AudioEncoder::new(&init).unwrap();
		inner.configure(&(&config).into())?;

		Ok(Self {
			config,
			inner,
			on_error,
			on_frame,
		})
	}

	pub fn encode(&mut self, frame: &AudioData) -> Result<(), Error> {
		self.inner.encode(frame.inner())?;
		Ok(())
	}

	pub fn queue_size(&self) -> u32 {
		self.inner.encode_queue_size()
	}

	pub fn config(&self) -> &AudioEncoderConfig {
		&self.config
	}

	pub async fn flush(&mut self) -> Result<(), Error> {
		wasm_bindgen_futures::JsFuture::from(self.inner.flush()).await?;
		Ok(())
	}
}

impl Drop for AudioEncoder {
	fn drop(&mut self) {
		let _ = self.inner.close();
	}
}

pub struct AudioEncoded {
	config: Rc<RefCell<Option<AudioDecoderConfig>>>,
	frames: mpsc::UnboundedReceiver<EncodedFrame>,
	closed: watch::Receiver<Result<(), Error>>,
}

impl AudioEncoded {
	fn new(
		config: Rc<RefCell<Option<AudioDecoderConfig>>>,
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

	pub fn config(&self) -> Option<AudioDecoderConfig> {
		self.config.borrow().clone()
	}
}
