use derive_more::{Deref, DerefMut, From};

#[derive(Debug, From, Deref, DerefMut)]
pub struct VideoFrame(pub web_sys::VideoFrame);

// Make sure we close the frame on drop.
impl Drop for VideoFrame {
    fn drop(&mut self) {
        self.0.close();
    }
}
