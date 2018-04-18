use std::io;
use std::net::TcpStream;

fn main() {
	let stdin = io::stdin();
	if let Some(port) = std::env::args().nth(1) {
		if let Ok(port) = port.parse::<u16>() {
			if let Ok(stream) = TcpStream::connect(format!("127.0.0.1:{port}", port = port)) {
				let res = stream.read(&[0u8; 1]);
				println!("DATA: {:?}", res);
			} else {
				println!("ERROR: Failed to connect to server!");
			}
		} else {
			println!("ERROR: First parameter is not a valid port number!");
		}
	} else {
		println!("ERROR: First parameter is not present!");
	}
}
