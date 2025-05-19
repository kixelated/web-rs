#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Dimensions {
	pub width: u32,
	pub height: u32,
}

impl Dimensions {
	pub fn new(width: u32, height: u32) -> Self {
		Self { width, height }
	}
}
