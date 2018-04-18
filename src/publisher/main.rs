#![feature(vec_remove_item)]

extern crate bytes;
extern crate either;
#[macro_use]
extern crate futures;
extern crate tokio;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use futures::sync::mpsc;
use bytes::{BigEndian, BufMut, Bytes, BytesMut};
use either::*;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::io::{Error, ErrorKind};
use std::fmt;
use std::thread;

type Tx = mpsc::UnboundedSender<Bytes>;
type Rx = mpsc::UnboundedReceiver<Bytes>;
struct Shared {
    subscribers: HashMap<SocketAddr, Tx>,
}
#[derive(Debug, Copy, Clone, PartialEq)]
enum EventType {
    PointerConnected,
    PointerDisconnected,
    PointerDown,
    PointerUp,
    PointerMove,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum EventData {
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
    fn bytes(&self) -> Bytes {
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
            &EventData::PointerDown {
                pointer,
                button,
                x,
                y,
            } => {
                let size = 7u8;
                bytes = BytesMut::with_capacity((size + 1) as usize);
                bytes.put(size); // SIZE
                bytes.put(2u8); // OPCODE
                bytes.put(pointer); // POINTER ID
                bytes.put(button); // BUTTON INDEX
                bytes.put_i16::<BigEndian>(x); // X COORD
                bytes.put_i16::<BigEndian>(y); // Y COORD
            }
            &EventData::PointerUp {
                pointer,
                button,
                x,
                y,
            } => {
                let size = 7u8;
                bytes = BytesMut::with_capacity((size + 1) as usize);
                bytes.put(size); // SIZE
                bytes.put(3u8); // OPCODE
                bytes.put(pointer); // POINTER ID
                bytes.put(button); // BUTTON INDEX
                bytes.put_i16::<BigEndian>(x); // X COORD
                bytes.put_i16::<BigEndian>(y); // Y COORD
            }
            &EventData::PointerMove {
                pointer,
                button,
                x,
                y,
            } => {
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

struct Subscriber {
    subscriptions: Vec<EventType>,
    frames: Frames,
    state: Arc<Mutex<Shared>>,
    rx: Rx,
    addr: SocketAddr,
}

#[derive(Debug)]
struct Frames {
    socket: TcpStream,
    rd: BytesMut,
    wr: BytesMut,
}

impl Shared {
    fn new() -> Self {
        Shared {
            subscribers: HashMap::new(),
        }
    }
}

struct PrintBytes(BytesMut);

impl fmt::UpperHex for PrintBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

impl Subscriber {
    fn new(subscriptions: Vec<EventType>, state: Arc<Mutex<Shared>>, frames: Frames) -> Subscriber {
        let addr = frames.socket.peer_addr().unwrap();
        let (tx, rx) = mpsc::unbounded();
        state.lock().unwrap().subscribers.insert(addr, tx);

        Subscriber {
            subscriptions,
            frames,
            state,
            rx,
            addr,
        }
    }
}

impl Future for Subscriber {
    type Item = ();
    type Error = io::Error;
    fn poll(&mut self) -> Poll<(), io::Error> {
        const MAX_FRAMES_PER_TICK: usize = 8;
        for i in 0..MAX_FRAMES_PER_TICK {
            match self.rx.poll().unwrap() {
                Async::Ready(Some(v)) => {
                    match v[1] {
                        0xFF => {
                            println!("[server] QUIT to {:?}", self.addr);
                            self.frames.buffer(&v);
                            let _ = self.frames.poll_flush()?;
                            return Ok(Async::Ready(()));
                        }
                        0x00 | 0x01 | 0x02 | 0x03 | 0x04 => {
                            println!("[server] EVENT to {:?}", self.addr);
                            self.frames.buffer(&v);
                        }
                        _ => {
                            println!("[server] ERROR: wrong internal message {:?}", v);
                        }
                    }
                    if i + 1 == MAX_FRAMES_PER_TICK {
                        task::current().notify();
                    }
                }
                _ => break,
            }
        }
        let _ = self.frames.poll_flush()?;

        while let Async::Ready(frame) = self.frames.poll()? {
            if let Some(message) = frame {
                match protocol(message.clone()) {
                    Ok(Either::Left(event)) => {
                        self.subscriptions.push(event.clone());
                        println!(
                            "[{:?}] {:X} => add = {:?}, subs = {:?}",
                            self.addr,
                            PrintBytes(message),
                            event,
                            self.subscriptions
                        );
                    }
                    Ok(Either::Right(event)) => {
                        self.subscriptions.remove_item(&event);
                        println!(
                            "[{:?}] {:X} => rmv = {:?}, subs = {:?}",
                            self.addr,
                            PrintBytes(message),
                            event,
                            self.subscriptions
                        );
                    }
                    Err(e) => {
                        eprintln!("[{:?}] ERROR: {:?}", self.addr, e);
                    }
                };
            } else {
                return Ok(Async::Ready(()));
            }
        }
        Ok(Async::NotReady)
    }
}

impl Drop for Subscriber {
    fn drop(&mut self) {
        self.state.lock().unwrap().subscribers.remove(&self.addr);
    }
}

impl Frames {
    fn new(socket: TcpStream) -> Self {
        Frames {
            socket,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
        }
    }

    fn buffer(&mut self, frame: &[u8]) {
        self.wr.reserve(frame.len());
        self.wr.put(frame);
    }

    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        while !self.wr.is_empty() {
            let n = try_ready!(self.socket.poll_write(&self.wr));
            assert!(n > 0);
            let _ = self.wr.split_to(n);
        }
        Ok(Async::Ready(()))
    }

    fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
        loop {
            self.rd.reserve(4);
            let n = try_ready!(self.socket.read_buf(&mut self.rd));
            if n == 0 {
                return Ok(Async::Ready(()));
            } else {
                println!(
                    "[{:?}] BUFFER: {} bytes",
                    self.socket.peer_addr().unwrap(),
                    n
                );
            }
        }
    }
}

impl Stream for Frames {
    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let sock_closed = self.fill_read_buf()?.is_ready();

        if !self.rd.is_empty() {
            let len = self.rd[0] as usize;
            if self.rd.len() >= len {
                let _ = self.rd.split_to(1);
                let frame = self.rd.split_to(len);
                return Ok(Async::Ready(Some(frame)));
            } else {
                return Ok(Async::NotReady);
            }
        }

        if sock_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

fn protocol(frame: BytesMut) -> Result<Either<EventType, EventType>, io::Error> {
    let event = match frame[1] {
        0 => EventType::PointerConnected,
        1 => EventType::PointerDisconnected,
        2 => EventType::PointerDown,
        3 => EventType::PointerUp,
        4 => EventType::PointerMove,
        _ => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("[server] ERROR: Invalid frame received: {:?}", frame),
            ))
        }
    };
    match frame[0] {
        0 => Ok(Either::Left(event)),
        1 => Ok(Either::Right(event)),
        _ => Err(Error::new(
            ErrorKind::Other,
            format!("[server] ERROR: Invalid frame received: {:?}", frame),
        )),
    }
}

fn process(socket: TcpStream, state: Arc<Mutex<Shared>>) {
    let frames = Frames::new(socket);
    let subscriber = Subscriber::new(Vec::new(), state, frames);
    tokio::spawn(subscriber.map_err(|e| {
        eprintln!("[server] ERROR: Connection error = {:?}", e);
    }));
}

fn main() {
    let stdin = std::io::stdin();
    let state = Arc::new(Mutex::new(Shared::new()));
    let main_state = state.clone();

    let handle = thread::spawn(move || {
        let port = std::env::args()
            .nth(1)
            .expect("[server] ERROR: First parameter is not present!");

        let port = port.parse::<u16>()
            .expect("[server] ERROR: First parameter is not a valid port number!");

        let addr = format!("127.0.0.1:{port}", port = port)
            .parse()
            .expect("[server] ERROR: Failed to parse listener address!");

        let listener =
            TcpListener::bind(&addr).expect("[server] ERROR: Failed to bind TCP Listener!");

        let server = listener
            .incoming()
            .for_each(move |socket| {
                println!(
                    "[server] ACCEPT: {:?}",
                    socket
                        .peer_addr()
                        .expect("[server] ERROR: Failed to read peer addr!")
                );

                process(socket, main_state.clone());
                Ok(())
            })
            .map_err(|e| {
                eprintln!("[server] ERROR: Accept error = {:?}", e);
            });

        tokio::run(server);
    });

    println!("[server] INSTRUCTIONS: Press (1) to generate a random PointerConnected event.");
    println!("[server] INSTRUCTIONS: Press (2) to generate a random PointerDisconnected event.");
    println!("[server] INSTRUCTIONS: Press (3) to generate a random PointerDown event.");
    println!("[server] INSTRUCTIONS: Press (4) to generate a random PointerUp event.");
    println!("[server] INSTRUCTIONS: Press (5) to generate a random PointerMove event.");
    println!("[server] INSTRUCTIONS: Press (Q) to finish execution.");

    let send = move |msg: Bytes| {
        let my_state = state.clone();
        for (_addr, tx) in &my_state.lock().unwrap().subscribers {
            tx.unbounded_send(msg.clone()).unwrap();
        }
    };

    'input: loop {
        let mut buf: [u8; 1] = [0];
        stdin.lock().read_exact(&mut buf);
        match buf[0] {
            b'1' => {
                send(EventData::PointerConnected(0).bytes());
                println!("[server] PRESSED (1)!");
            }
            b'2' => {
                send(EventData::PointerDisconnected(0).bytes());
                println!("[server] PRESSED (2)!");
            }
            b'3' => {
                send(
                    EventData::PointerDown {
                        pointer: 0,
                        button: 0,
                        x: 16,
                        y: 16,
                    }.bytes(),
                );
                println!("[server] PRESSED (3)!");
            }
            b'4' => {
                send(
                    EventData::PointerUp {
                        pointer: 0,
                        button: 0,
                        x: 32,
                        y: 32,
                    }.bytes(),
                );
                println!("[server] PRESSED (4)!");
            }
            b'5' => {
                send(
                    EventData::PointerMove {
                        pointer: 0,
                        button: 0,
                        x: 24,
                        y: 24,
                    }.bytes(),
                );
                println!("[server] PRESSED (5)!");
            }
            b'q' | b'Q' => {
                let mut quit = BytesMut::new();
                quit.extend_from_slice(b"\xFF");
                send(quit.freeze());
                println!("[server] PRESSED (Q)!");
                //break 'input;
            }
            _ => {
                continue;
            }
        }
    }

    let mut quit = BytesMut::new();
    quit.extend_from_slice(b"\xFF");
    send(quit.freeze());

    handle.join().unwrap();
}
