use std::time::Duration;

use derive_more::From;

use crate::Timestamp;

#[derive(Debug, From)]
pub struct AudioData(web_sys::AudioData);

impl AudioData {
	pub fn timestamp(&self) -> Timestamp {
		Timestamp::from_micros(self.0.timestamp() as _)
	}

	pub fn duration(&self) -> Duration {
		Duration::from_micros(self.0.duration() as _)
	}

	pub fn format(&self) -> Option<web_sys::AudioSampleFormat> {
		self.0.format()
	}

	pub fn sample_rate(&self) -> u32 {
		self.0.sample_rate() as u32
	}

	pub fn frame_count(&self) -> u32 {
		self.0.number_of_frames()
	}

	pub fn channel_count(&self) -> u32 {
		self.0.number_of_channels()
	}

	pub fn inner(&self) -> &web_sys::AudioData {
		&self.0
	}
}

// Make sure we close the frame on drop.
impl Drop for AudioData {
	fn drop(&mut self) {
		self.0.close();
	}
}
