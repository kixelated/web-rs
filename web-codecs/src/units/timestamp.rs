use std::{fmt, ops};

use super::Duration;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Timestamp(Duration);

impl Timestamp {
	pub fn from_micros(micros: u64) -> Self {
		Self(Duration::from_micros(micros))
	}

	pub fn from_millis(millis: u64) -> Self {
		Self(Duration::from_millis(millis))
	}

	pub fn from_seconds(seconds: u64) -> Self {
		Self(Duration::from_seconds(seconds))
	}

	pub fn from_minutes(minutes: u64) -> Self {
		Self(Duration::from_minutes(minutes))
	}

	pub fn from_hours(hours: u64) -> Self {
		Self(Duration::from_hours(hours))
	}

	pub fn from_units(value: u64, base: u64) -> Self {
		Self(Duration::from_units(value, base))
	}

	pub fn as_micros(self) -> u64 {
		self.0.as_micros()
	}

	pub fn as_millis(self) -> u64 {
		self.0.as_millis()
	}

	pub fn as_seconds(self) -> u64 {
		self.0.as_seconds()
	}

	pub fn as_minutes(self) -> u64 {
		self.0.as_minutes()
	}

	pub fn as_hours(self) -> u64 {
		self.0.as_hours()
	}

	pub fn as_units(self, base: u64) -> u64 {
		self.0.as_units(base)
	}
}

impl ops::Deref for Timestamp {
	type Target = Duration;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl ops::Add<Duration> for Timestamp {
	type Output = Timestamp;

	fn add(self, rhs: Duration) -> Self::Output {
		Timestamp(self.0 + rhs)
	}
}

impl ops::Sub<Duration> for Timestamp {
	type Output = Timestamp;

	fn sub(self, rhs: Duration) -> Self::Output {
		Timestamp(self.0 - rhs)
	}
}

impl ops::Sub<Timestamp> for Timestamp {
	type Output = Duration;

	fn sub(self, rhs: Timestamp) -> Self::Output {
		self.0 - rhs.0
	}
}

impl fmt::Debug for Timestamp {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}
