pub use web_message_derive::Message;

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn to_from_enum() {
		#[derive(Message, Clone, Debug, PartialEq, Eq)]
		#[msg(tag = "command")]
		enum Command {
			Init {
				width: u64,
				name: String,
			},
			Frame {
				#[msg(transferable)]
				payload: js_sys::ArrayBuffer,
			},
		}

		let command = Command::Init {
			width: 100,
			name: "test".to_string(),
		};

		let (obj, transferable) = command.clone().into_message();
		let out = Command::from_message(obj.into()).unwrap();

		assert_eq!(command, out);
		assert_eq!(transferable.length(), 0);
	}

	#[test]
	fn to_from_struct() {
		#[derive(Message, Clone, Debug, PartialEq, Eq)]
		struct Event {
			#[msg(transferable)]
			payload: js_sys::ArrayBuffer,
			width: u64,
			name: String,
		}

		let event = Event {
			payload: js_sys::ArrayBuffer::new(100),
			width: 100,
			name: "test".to_string(),
		};

		let (obj, transferable) = event.clone().into_message();
		let out = Event::from_message(obj.into()).unwrap();

		assert_eq!(event, out);
		assert_eq!(transferable, [event.payload].iter().collect());
	}
}
