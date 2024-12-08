#[derive(Debug, Clone)]
pub struct VideoColorSpaceConfig {
    inner: web_sys::VideoColorSpaceInit,
}

impl Default for VideoColorSpaceConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoColorSpaceConfig {
    pub fn new() -> Self {
        Self {
            inner: web_sys::VideoColorSpaceInit::new(),
        }
    }

    pub fn full_range(self, enabled: bool) -> Self {
        self.inner.set_full_range(enabled);
        self
    }

    pub fn matrix(self, matrix: VideoMatrixCoefficients) -> Self {
        self.inner.set_matrix(matrix);
        self
    }

    pub fn primaries(self, primaries: VideoColorPrimaries) -> Self {
        self.inner.set_primaries(primaries);
        self
    }

    pub fn transfer(self, transfer: VideoTransferCharacteristics) -> Self {
        self.inner.set_transfer(transfer);
        self
    }
}

impl From<&VideoColorSpaceConfig> for web_sys::VideoColorSpaceInit {
    fn from(this: &VideoColorSpaceConfig) -> Self {
        this.inner.clone()
    }
}

impl From<web_sys::VideoColorSpaceInit> for VideoColorSpaceConfig {
    fn from(inner: web_sys::VideoColorSpaceInit) -> Self {
        Self { inner }
    }
}

pub type VideoMatrixCoefficients = web_sys::VideoMatrixCoefficients;
pub type VideoColorPrimaries = web_sys::VideoColorPrimaries;
pub type VideoTransferCharacteristics = web_sys::VideoTransferCharacteristics;
