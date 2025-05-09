use std::ops::{Deref, DerefMut};
use std::time::Duration;

use crate::{Error, Result, Timestamp};

pub use web_sys::AudioSampleFormat as AudioDataFormat;

/// A wrapper around [web_sys::AudioData] that closes on Drop.
// It's an option so `leak` can return the inner AudioData if needed.
#[derive(Debug)]
pub struct AudioData(Option<web_sys::AudioData>);

impl AudioData {
	/// A helper to construct AudioData in a more type-safe way.
	/// This currently only supports F32.
	pub fn new<'a>(
		channels: impl ExactSizeIterator<Item = &'a [f32]>,
		sample_rate: u32,
		timestamp: Timestamp,
	) -> Result<Self> {
		let mut channels = channels.enumerate();
		let channel_count = channels.size_hint().0;
		let (_, channel) = channels.next().ok_or(Error::NoChannels)?;

		let frame_count = channel.len();
		let total_samples = channel_count * frame_count;

		// Annoyingly, we need to create a contiguous buffer for the data.
		let data = js_sys::Float32Array::new_with_length(total_samples as _);

		// Copy the first channel using a Float32Array as a view into the buffer.
		let slice = js_sys::Float32Array::new_with_byte_offset_and_length(&data.buffer(), 0, frame_count as _);
		slice.copy_from(channel);

		for (i, channel) in channels {
			// Copy the other channels using a Float32Array as a view into the buffer.
			let slice = js_sys::Float32Array::new_with_byte_offset_and_length(
				&data.buffer(),
				(i * frame_count) as u32,
				frame_count as _,
			);
			slice.copy_from(channel);
		}

		let init = web_sys::AudioDataInit::new(
			&data,
			AudioDataFormat::F32Planar,
			channel_count as _,
			frame_count as _,
			sample_rate as _,
			timestamp.as_micros() as _,
		);

		// Manually add `transfer` to the init options.
		// TODO Update web_sys to support this natively.
		// I'm not even sure if this works.
		let transfer = js_sys::Array::new();
		transfer.push(&data.buffer());
		js_sys::Reflect::set(&init, &js_sys::JsString::from("transfer"), &transfer)?;

		let audio_data = web_sys::AudioData::new(&init)?;
		Ok(Self(Some(audio_data)))
	}

	pub fn timestamp(&self) -> Timestamp {
		Timestamp::from_micros(self.0.as_ref().unwrap().timestamp() as _)
	}

	pub fn duration(&self) -> Duration {
		Duration::from_micros(self.0.as_ref().unwrap().duration() as _)
	}

	pub fn sample_rate(&self) -> u32 {
		self.0.as_ref().unwrap().sample_rate() as u32
	}

	pub fn append_to<T: AudioAppend>(&self, dst: &mut T, channel: usize, options: AudioCopyOptions) -> Result<()> {
		dst.append_to(self, channel, options)
	}

	pub fn copy_to<T: AudioCopy>(&self, dst: &mut T, channel: usize, options: AudioCopyOptions) -> Result<()> {
		dst.copy_to(self, channel, options)
	}

	pub fn leak(mut self) -> web_sys::AudioData {
		self.0.take().unwrap()
	}
}

impl Clone for AudioData {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl Deref for AudioData {
	type Target = web_sys::AudioData;

	fn deref(&self) -> &Self::Target {
		self.0.as_ref().unwrap()
	}
}

impl DerefMut for AudioData {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.0.as_mut().unwrap()
	}
}

// Make sure we close the frame on drop.
impl Drop for AudioData {
	fn drop(&mut self) {
		if let Some(audio_data) = self.0.take() {
			audio_data.close();
		}
	}
}

impl From<web_sys::AudioData> for AudioData {
	fn from(this: web_sys::AudioData) -> Self {
		Self(Some(this))
	}
}

pub trait AudioCopy {
	fn copy_to(&mut self, data: &AudioData, channel: usize, options: AudioCopyOptions) -> Result<()>;
}

impl AudioCopy for [u8] {
	fn copy_to(&mut self, data: &AudioData, channel: usize, options: AudioCopyOptions) -> Result<()> {
		let options = options.into_web_sys(channel);
		// NOTE: The format is unuset so it will default to the AudioData format.
		// This means you couldn't export as U8Planar for whatever that's worth...
		data.0.as_ref().unwrap().copy_to_with_u8_slice(self, &options)?;
		Ok(())
	}
}

impl AudioCopy for [f32] {
	fn copy_to(&mut self, data: &AudioData, channel: usize, options: AudioCopyOptions) -> Result<()> {
		let options = options.into_web_sys(channel);
		options.set_format(AudioDataFormat::F32Planar);

		// Cast from a f32 to a u8 slice.
		let bytes = bytemuck::cast_slice_mut(self);
		data.0.as_ref().unwrap().copy_to_with_u8_slice(bytes, &options)?;
		Ok(())
	}
}

impl AudioCopy for js_sys::Uint8Array {
	fn copy_to(&mut self, data: &AudioData, channel: usize, options: AudioCopyOptions) -> Result<()> {
		let options = options.into_web_sys(channel);
		data.0.as_ref().unwrap().copy_to_with_u8_array(self, &options)?;
		Ok(())
	}
}

impl AudioCopy for js_sys::Float32Array {
	fn copy_to(&mut self, data: &AudioData, channel: usize, options: AudioCopyOptions) -> Result<()> {
		let options = options.into_web_sys(channel);
		data.0.as_ref().unwrap().copy_to_with_buffer_source(self, &options)?;
		Ok(())
	}
}

pub trait AudioAppend {
	fn append_to(&mut self, data: &AudioData, channel: usize, options: AudioCopyOptions) -> Result<()>;
}

impl AudioAppend for Vec<f32> {
	fn append_to(&mut self, data: &AudioData, channel: usize, options: AudioCopyOptions) -> Result<()> {
		// TODO do unsafe stuff to avoid zeroing the buffer.
		let grow = options.count.unwrap_or(data.number_of_frames() as _) - options.offset;
		let offset = self.len();
		self.resize(offset + grow, 0.0);

		let options = options.into_web_sys(channel);
		let bytes = bytemuck::cast_slice_mut(&mut self[offset..]);
		data.0.as_ref().unwrap().copy_to_with_u8_slice(bytes, &options)?;

		Ok(())
	}
}

#[derive(Debug, Default)]
pub struct AudioCopyOptions {
	pub offset: usize,        // defaults to 0
	pub count: Option<usize>, // defaults to remainder
}

impl AudioCopyOptions {
	fn into_web_sys(self, channel: usize) -> web_sys::AudioDataCopyToOptions {
		let options = web_sys::AudioDataCopyToOptions::new(channel as _);
		options.set_frame_offset(self.offset as _);
		if let Some(count) = self.count {
			options.set_frame_count(count as _);
		}
		options
	}
}
