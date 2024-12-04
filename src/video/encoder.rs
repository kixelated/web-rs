use tokio::sync::{mpsc, watch};
use wasm_bindgen::prelude::*;

use crate::{EncodedFrame, Error};

use super::{Dimensions, VideoFrame};

#[derive(Debug)]
pub enum BitrateMode {
    Constant,
    Variable,
    Quantizer,
}

#[derive(Debug)]
pub struct VideoEncoderConfig {
    pub codec: String,
    pub resolution: Dimensions,
    pub display: Option<Dimensions>,
    pub hardware_acceleration: Option<bool>,
    pub latency_optimized: Option<bool>,
    pub bit_rate: Option<f64>,         // bits per second
    pub frame_rate: Option<f64>,       // frames per second
    pub alpha_preserved: Option<bool>, // keep alpha channel
    pub scalability_mode: Option<String>,
    pub bitrate_mode: Option<BitrateMode>,
}

impl VideoEncoderConfig {
    pub fn new<T: Into<String>>(codec: T, resolution: Dimensions) -> Self {
        Self {
            codec: codec.into(),
            resolution,
            display: None,
            hardware_acceleration: None,
            latency_optimized: None,
            bit_rate: None,
            frame_rate: None,
            alpha_preserved: None,
            scalability_mode: None,
            bitrate_mode: None,
        }
    }

    pub async fn is_supported(&self) -> Result<bool, Error> {
        let res = wasm_bindgen_futures::JsFuture::from(web_sys::VideoEncoder::is_config_supported(
            &self.into(),
        ))
        .await?;

        let supported = js_sys::Reflect::get(&res, &JsValue::from_str("supported"))
            .unwrap()
            .as_bool()
            .unwrap();

        Ok(supported)
    }

    pub fn configure(self) -> Result<(VideoEncoder, VideoEncoded), Error> {
        let (frames_tx, frames_rx) = mpsc::unbounded_channel();
        let (closed_tx, closed_rx) = watch::channel(Ok(()));
        let closed_tx2 = closed_tx.clone();

        let on_error = Closure::wrap(Box::new(move |e: JsValue| {
            closed_tx.send_replace(Err(Error::from(e))).ok();
        }) as Box<dyn FnMut(_)>);

        let on_frame = Closure::wrap(Box::new(move |e: JsValue| {
            let frame: web_sys::EncodedVideoChunk = e.unchecked_into();
            let frame = EncodedFrame::from(frame);

            if frames_tx.send(frame).is_err() {
                closed_tx2.send_replace(Err(Error::Dropped)).ok();
            }
        }) as Box<dyn FnMut(_)>);

        let init = web_sys::VideoEncoderInit::new(
            on_error.as_ref().unchecked_ref(),
            on_frame.as_ref().unchecked_ref(),
        );
        let inner: web_sys::VideoEncoder = web_sys::VideoEncoder::new(&init).unwrap();
        inner.configure(&(&self).into())?;

        let decoder = VideoEncoder {
            inner,
            on_error,
            on_frame,
        };

        let decoded = VideoEncoded {
            frames: frames_rx,
            closed: closed_rx,
        };

        Ok((decoder, decoded))
    }
}

impl From<&VideoEncoderConfig> for web_sys::VideoEncoderConfig {
    fn from(this: &VideoEncoderConfig) -> Self {
        let config = web_sys::VideoEncoderConfig::new(
            &this.codec,
            this.resolution.width,
            this.resolution.height,
        );

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

        if let Some(value) = this.bit_rate {
            config.set_bitrate(value);
        }

        if let Some(value) = this.frame_rate {
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

        if let Some(value) = &this.bitrate_mode {
            // TODO not supported yet
        }

        config
    }
}

pub struct VideoEncoder {
    inner: web_sys::VideoEncoder,

    // These are held to avoid dropping them.
    #[allow(dead_code)]
    on_error: Closure<dyn FnMut(JsValue)>,
    #[allow(dead_code)]
    on_frame: Closure<dyn FnMut(JsValue)>,
}

impl VideoEncoder {
    pub fn encode(&self, frame: VideoFrame) -> Result<(), Error> {
        self.inner.encode(&frame)?;
        Ok(())
    }

    pub async fn flush(&self) -> Result<(), Error> {
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
    frames: mpsc::UnboundedReceiver<EncodedFrame>,
    closed: watch::Receiver<Result<(), Error>>,
}

impl VideoEncoded {
    pub async fn next(&mut self) -> Result<Option<EncodedFrame>, Error> {
        tokio::select! {
            biased;
            frame = self.frames.recv() => Ok(frame),
            Ok(()) = self.closed.changed() => Err(self.closed.borrow().clone().err().unwrap()),
        }
    }
}
