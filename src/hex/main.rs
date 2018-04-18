use std::fmt;
use std::io;
use std::io::Read;

struct Printable(Vec<u8>);

impl fmt::UpperHex for Printable {
	fn fmt(&self,
	       f: &mut fmt::Formatter)
	       -> fmt::Result {
		if let Err(e) = write!(f, "[ ") {
			return Err(e);
		}
		for i in 0..self.0.len() {
			if let Err(e) = write!(f, "{:02X} ", self.0[i] as u8) {
				return Err(e);
			}
		}
		write!(f, "]")
	}
}

fn main() {
	let mut len: u8 = 0;
	let mut packet: Vec<u8> = Vec::new();
	let mut count = 0usize;
	let stdin = io::stdin();
	for byte in stdin.lock().bytes() {
		let byte = byte.unwrap();
		if len == 0 {
			len = byte;
		} else {
			packet.push(byte);
			if packet.len() == len as usize {
				let hex = Printable(packet);
				println!("#{:04} {:X}", count, hex);
				packet = hex.0;
				packet.clear();
				len = 0;
				count += 1;
			}
		}
	}
}
