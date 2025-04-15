pub use web_message_derive::Message;

#[cfg(test)]
mod test {
	use web_sys::js_sys::{Array, ArrayBuffer};

	use crate::Message;

	#[test]
	fn enum_msg() {
		#[derive(Message, Clone, Debug, PartialEq, Eq)]
		enum Command {
			Connect { url: String },
			Frame { name: Option<String>, payload: ArrayBuffer },
			Close,
		}

		let command = Command::Frame {
			name: Some("test".to_string()),
			payload: ArrayBuffer::new(100),
		};

		let mut transferable = Array::new();
		let obj = command.clone().into_message(&mut transferable);
		let out = Command::from_message(obj).unwrap();

		assert_eq!(command, out);
		assert_eq!(transferable.length(), 1);
	}

	#[test]
	fn struct_msg() {
		#[derive(Message, Clone, Debug, PartialEq, Eq)]
		struct Event {
			payload: ArrayBuffer,
			width: u64,
			name: String,
		}

		let event = Event {
			payload: ArrayBuffer::new(100),
			width: 100,
			name: "test".to_string(),
		};

		let mut transferable = Array::new();
		let obj = event.clone().into_message(&mut transferable);
		let out = Event::from_message(obj).unwrap();

		assert_eq!(event, out);
		assert_eq!(transferable, [event.payload].iter().collect());
	}

	#[test]
	fn enum_variant() {
		#[derive(Message, Clone, Debug, PartialEq, Eq)]
		struct Config {
			width: u32,
			height: u32,
		}

		#[derive(Message, Clone, Debug, PartialEq, Eq)]
		enum Command {
			Connect { url: String },
			Config(Config),
			Close,
		}

		let command = Command::Config(Config {
			width: 100,
			height: 100,
		});

		let mut transferable = Array::new();
		let obj = command.clone().into_message(&mut transferable);
		let out = Command::from_message(obj).unwrap();

		assert_eq!(command, out);
		assert_eq!(transferable.length(), 1);
	}
}
