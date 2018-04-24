extern crate bytes;
extern crate either;
extern crate futures;
extern crate tokio;
extern crate tokio_io;

use futures::Future;
use futures::stream::Stream;
use futures::sync::mpsc;
use tokio::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use std::net::SocketAddr;
use std::thread;
use bytes::Bytes;

extern crate sd_rust;

mod codec {
    use std::io;
    use bytes::{BufMut, Bytes, BytesMut};
    use tokio_io::codec::{Decoder, Encoder};
    pub struct Codec;

    impl Decoder for Codec {
        type Item = BytesMut;
        type Error = io::Error;

        fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<BytesMut>> {
            if buf.len() > 0 {
                let len = buf.len();
                Ok(Some(buf.split_to(len)))
            } else {
                Ok(None)
            }
        }
    }

    impl Encoder for Codec {
        type Item = Bytes;
        type Error = io::Error;

        fn encode(&mut self, data: Bytes, buf: &mut BytesMut) -> io::Result<()> {
            buf.put(&data[..]);
            Ok(())
        }
    }
}

fn main() {
    let stdin = std::io::stdin();

    let (mut tx, rx) = mpsc::channel(1);

    let handle = thread::spawn(move || {
        let port = std::env::args()
            .nth(1)
            .expect("[client] ERROR: first parameter is not present!");

        let port = port.parse::<u16>()
            .expect("[client] ERROR: first parameter is invalid!");

        let addr = format!("127.0.0.1:{port}", port = port)
            .parse()
            .expect("[client] ERROR: failed to parse server addr!");

        let tcp = TcpStream::connect(&addr);

        let client = tcp.and_then(|stream| {
            let (sending, receiving) = stream.framed(codec::Codec).split();

            let reader = receiving.for_each(move |msg| {
                println!("[client] RECV = {:?}", msg);
                Ok(())
            });

            let writer = rx.map_err(|()| unreachable!("rx cannot fail"))
                .fold(sending, |sending, msg| sending.send(msg))
                .map(|_| ());

            reader.select(writer).map(|_| ()).map_err(|(err, _)| err)
        }).map_err(|e| {
            eprintln!("[client] ERROR = {:?}", e);
        });

        tokio::run(client);
    });

    let send = move |tx: mpsc::Sender<Bytes>, msg: Bytes| -> mpsc::Sender<Bytes> {
        match tx.send(msg).wait() {
            Ok(s) => s,
            Err(_) => unreachable!("what the fuck"),
        }
    };

    let mut hello = bytes::BytesMut::new();
    hello.extend_from_slice(b"\x01");
    tx = send(tx, hello.freeze());

    let mut selection: Option<sd_rust::EventType> = None;
    'input: loop {
        let mut buf: [u8; 1] = [0];
        stdin.lock().read_exact(&mut buf);
        match buf[0] {
            b'1' => {
                selection = Some(sd_rust::EventType::PointerConnected);
                println!("[client] SELECT Pointer Connected");
            }
            b'2' => {
                selection = Some(sd_rust::EventType::PointerDisconnected);
                println!("[client] SELECT Pointer Disconnected");
            }
            b'3' => {
                selection = Some(sd_rust::EventType::PointerDown);
                println!("[client] SELECT Pointer Down");
            }
            b'4' => {
                selection = Some(sd_rust::EventType::PointerUp);
                println!("[client] SELECT Pointer Up");
            }
            b'5' => {
                selection = Some(sd_rust::EventType::PointerMove);
                println!("[client] SELECT Pointer Move");
            }
            b's' => {
                if let Some(event) = selection {
                    tx = send(tx, event.bytes(0));
                    println!("[client] SUB TO {:?}", event);
                } else {
                    println!("[client] NO EVENT SELECTED, CANNOT SUB");
                }
            }
            b'u' => {
                if let Some(event) = selection {
                    tx = send(tx, event.bytes(1));
                    println!("[client] UNSUB TO {:?}", event);
                } else {
                    println!("[client] NO EVENT SELECTED, CANNOT SUB");
                }
            }
            b'q' => {
                break 'input;
            }
            _ => {
                continue;
            }
        }
    }

    let mut quit = bytes::BytesMut::new();
    quit.extend_from_slice(b"\x00");
    tx.send(quit.freeze()).wait();
    handle.join().unwrap();
}
