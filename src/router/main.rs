#![feature(vec_remove_item)]

extern crate bytes;
extern crate either;
#[macro_use]
extern crate futures;
extern crate tokio;

use bytes::{BufMut, Bytes, BytesMut};
use either::*;
use futures::sync::mpsc;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use std::collections::HashMap;
use std::fmt;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;

extern crate sd_rust;

type Tx = mpsc::UnboundedSender<Bytes>;
type Rx = mpsc::UnboundedReceiver<Bytes>;

struct Routing {}

struct Subscriber {
    subscriptions: Vec<sd_rust::EventType>,
    frames: Frames,
    state: Arc<Mutex<Shared>>,
    rx: Rx,
    addr: SocketAddr,
}

fn main() {
    let stdin = std::io::stdin();

    let (tx, rs) = mpsc::channel(1);

    let handle = thread::spawn(move || {
        let port = std::env::args()
            .nth(1)
            .expect("[router] ERROR: first parameter is not present!");

        let port = port.parse::<u16>()
            .expect("[router] ERROR: first parameter is invalid!");

        let addr = format!("127.0.0.1:{port}", port = port)
            .parse()
            .expect("[router] ERROR: failed to parse router addr!");

        let listener =
            TcpListener::bind(&addr).expect("[router] ERROR: failed to bind TCP Listener!");

        let server = listener
            .incoming()
            .for_each(move |socket| {
                println!(
                    "[router] ACCEPT: {:?}",
                    socket
                        .peer_addr()
                        .expect("[router] ERROR: failed to read peer addr!")
                );

                process(socket, main_state.clone());
                Ok(())
            })
            .map_err(|e| {
                println!("[router] ERROR: {:?}", e);
            });

        tokio::run(server);
    });

    // TODO REDIRECT

    handle.join().unwrap();
}
