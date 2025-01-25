use std::time::Duration;

use derive_more::From;

#[derive(Debug, From)]
pub struct VideoFrame(web_sys::VideoFrame);

impl VideoFrame {
	pub fn timestamp(&self) -> Duration {
		Duration::from_micros(self.0.timestamp().unwrap() as _)
	}

	pub fn duration(&self) -> Option<Duration> {
		Some(Duration::from_micros(self.0.duration()? as _))
	}

	pub fn display_width(&self) -> u32 {
		self.0.display_width()
	}

	pub fn display_height(&self) -> u32 {
		self.0.display_height()
	}

	pub fn coded_width(&self) -> u32 {
		self.0.coded_width()
	}

	pub fn coded_height(&self) -> u32 {
		self.0.coded_height()
	}

	pub fn inner(&self) -> &web_sys::VideoFrame {
		&self.0
	}
}

// Make sure we close the frame on drop.
impl Drop for VideoFrame {
	fn drop(&mut self) {
		self.0.close();
	}
}
