use std::{
	ops::{Deref, DerefMut},
	time::Duration,
};

use derive_more::From;

use crate::Timestamp;

use super::Dimensions;

#[derive(Debug, From)]
pub struct VideoFrame(web_sys::VideoFrame);

impl VideoFrame {
	pub fn timestamp(&self) -> Timestamp {
		Timestamp::from_micros(self.0.timestamp().unwrap() as _)
	}

	pub fn duration(&self) -> Option<Duration> {
		Some(Duration::from_micros(self.0.duration()? as _))
	}

	pub fn dimensions(&self) -> Dimensions {
		Dimensions {
			width: self.0.coded_width(),
			height: self.0.coded_height(),
		}
	}
}

// Avoid closing the video frame on transfer by cloning it.
impl From<VideoFrame> for web_sys::VideoFrame {
	fn from(this: VideoFrame) -> Self {
		this.0.clone().expect("detached")
	}
}

impl Clone for VideoFrame {
	fn clone(&self) -> Self {
		Self(self.0.clone().expect("detached"))
	}
}

impl Deref for VideoFrame {
	type Target = web_sys::VideoFrame;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for VideoFrame {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

// Make sure we close the frame on drop.
impl Drop for VideoFrame {
	fn drop(&mut self) {
		self.0.close();
	}
}
