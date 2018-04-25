extern crate bytes;

use bytes::{BigEndian, BufMut, Bytes, BytesMut};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EventType {
	PointerConnected,
	PointerDisconnected,
	PointerDown,
	PointerUp,
	PointerMove,
}

impl EventType {
	pub fn from_opcode(opcode: u8) -> Option<EventType> {
		match opcode {
			0x00 => Some(EventType::PointerConnected),
			0x01 => Some(EventType::PointerDisconnected),
			0x02 => Some(EventType::PointerDown),
			0x03 => Some(EventType::PointerUp),
			0x04 => Some(EventType::PointerMove),
			_ => None,
		}
	}

	pub fn bytes(&self,
	             opcode: u8)
	             -> Bytes {
		let mut bytes: BytesMut;
		match self {
			&EventType::PointerConnected => {
				let size = 2u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(opcode); // OPCODE
				bytes.put(0u8); // EVENT
			}
			&EventType::PointerDisconnected => {
				let size = 2u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(opcode); // OPCODE
				bytes.put(1u8); // EVENT
			}
			&EventType::PointerDown => {
				let size = 2u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(opcode); // OPCODE
				bytes.put(2u8); // EVENT
			}
			&EventType::PointerUp => {
				let size = 2u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(opcode); // OPCODE
				bytes.put(3u8); // EVENT
			}
			&EventType::PointerMove => {
				let size = 2u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(opcode); // OPCODE
				bytes.put(4u8); // EVENT
			}
			_ => {
				unreachable!();
			}
		};
		bytes.freeze()
	}
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum EventData {
	PointerConnected(u8),
	PointerDisconnected(u8),
	PointerDown {
		pointer: u8,
		button: u8,
		x: i16,
		y: i16,
	},
	PointerUp {
		pointer: u8,
		button: u8,
		x: i16,
		y: i16,
	},
	PointerMove {
		pointer: u8,
		button: u8,
		x: i16,
		y: i16,
	},
}

impl EventData {
	pub fn bytes(&self) -> Bytes {
		let mut bytes: BytesMut;
		match self {
			&EventData::PointerConnected(id) => {
				let size = 3u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(3u8); // OPCODE
				bytes.put(0u8); // EVENT
				bytes.put(id); // POINTER ID
			}
			&EventData::PointerDisconnected(id) => {
				let size = 3u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(3u8); // OPCODE
				bytes.put(1u8); // EVENT
				bytes.put(id); // POINTER ID
			}
			&EventData::PointerDown { pointer,
			                          button,
			                          x,
			                          y, } => {
				let size = 8u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(3u8); // OPCODE
				bytes.put(2u8); // EVENT
				bytes.put(pointer); // POINTER ID
				bytes.put(button); // BUTTON INDEX
				bytes.put_i16::<BigEndian>(x); // X COORD
				bytes.put_i16::<BigEndian>(y); // Y COORD
			}
			&EventData::PointerUp { pointer,
			                        button,
			                        x,
			                        y, } => {
				let size = 8u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(3u8); // OPCODE
				bytes.put(3u8); // EVENT
				bytes.put(pointer); // POINTER ID
				bytes.put(button); // BUTTON INDEX
				bytes.put_i16::<BigEndian>(x); // X COORD
				bytes.put_i16::<BigEndian>(y); // Y COORD
			}
			&EventData::PointerMove { pointer,
			                          button,
			                          x,
			                          y, } => {
				let size = 8u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(3u8); // OPCODE
				bytes.put(4u8); // EVENT
				bytes.put(pointer); // POINTER ID
				bytes.put(button); // BUTTON INDEX
				bytes.put_i16::<BigEndian>(x); // X COORD
				bytes.put_i16::<BigEndian>(y); // Y COORD
			}
			_ => {
				unreachable!();
			}
		};
		bytes.freeze()
	}
}
