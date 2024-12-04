use tokio::sync::{mpsc, watch};
use wasm_bindgen::prelude::*;

use super::{Dimensions, VideoColorSpaceConfig, VideoFrame};
use crate::{EncodedFrame, Error};

#[derive(Debug)]
pub struct VideoDecoderConfig {
    pub codec: String,
    pub resolution: Option<Dimensions>,
    pub display: Option<Dimensions>,
    pub color_space: Option<VideoColorSpaceConfig>,
    pub description: Option<Vec<u8>>,
    pub hardware_acceleration: Option<bool>,
    pub latency_optimized: Option<bool>,
}

impl VideoDecoderConfig {
    pub fn new<T: Into<String>>(codec: T) -> Self {
        Self {
            codec: codec.into(),
            resolution: None,
            color_space: None,
            display: None,
            description: None,
            hardware_acceleration: None,
            latency_optimized: None,
        }
    }

    pub async fn is_supported(&self) -> Result<bool, Error> {
        let res = wasm_bindgen_futures::JsFuture::from(web_sys::VideoDecoder::is_config_supported(
            &self.into(),
        ))
        .await?;

        let supported = js_sys::Reflect::get(&res, &JsValue::from_str("supported"))
            .unwrap()
            .as_bool()
            .unwrap();

        Ok(supported)
    }

    pub fn configure(self) -> Result<(VideoDecoder, VideoDecoded), Error> {
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

        let init = web_sys::VideoDecoderInit::new(
            on_error.as_ref().unchecked_ref(),
            on_frame.as_ref().unchecked_ref(),
        );
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
