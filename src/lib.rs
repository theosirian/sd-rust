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
				let size = 2u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(0u8); // OPCODE
				bytes.put(id); // POINTER ID
			}
			&EventData::PointerDisconnected(id) => {
				let size = 2u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(1u8); // OPCODE
				bytes.put(id); // POINTER ID
			}
			&EventData::PointerDown { pointer,
			                          button,
			                          x,
			                          y, } => {
				let size = 7u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(2u8); // OPCODE
				bytes.put(pointer); // POINTER ID
				bytes.put(button); // BUTTON INDEX
				bytes.put_i16::<BigEndian>(x); // X COORD
				bytes.put_i16::<BigEndian>(y); // Y COORD
			}
			&EventData::PointerUp { pointer,
			                        button,
			                        x,
			                        y, } => {
				let size = 7u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(3u8); // OPCODE
				bytes.put(pointer); // POINTER ID
				bytes.put(button); // BUTTON INDEX
				bytes.put_i16::<BigEndian>(x); // X COORD
				bytes.put_i16::<BigEndian>(y); // Y COORD
			}
			&EventData::PointerMove { pointer,
			                          button,
			                          x,
			                          y, } => {
				let size = 7u8;
				bytes = BytesMut::with_capacity((size + 1) as usize);
				bytes.put(size); // SIZE
				bytes.put(4u8); // OPCODE
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
