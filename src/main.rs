extern crate futures;
extern crate tokio_core;
extern crate tokio_line;
extern crate tokio_timer;
extern crate tokio_io;

use futures::future::{Future};
use futures::{Sink, Stream};
use futures::sync::mpsc::{self, UnboundedSender};
use tokio_timer::*;
use std::{io, str};
use tokio_core::io::{Codec, EasyBuf,Io};
use tokio_core::net::{TcpStream};
use tokio_core::reactor::{Core, Handle};
use std::time::Duration;

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let (tx, rx) = mpsc::unbounded();
    simulated_messaging_receiving_from_clients(tx, &handle);

    let remote_addr = "127.0.0.1:9876".parse().unwrap();
    let tcp = TcpStream::connect(&remote_addr, &handle);

    let client = tcp.and_then(move |stream| {
        let (sender, _receiver) = stream.framed(LineCodec).split();

        let writer = rx
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "..."))
            .fold(sender, |sender, msg| {
                println!("before sending `{}`", msg);
                sender.send(msg)
                    .map(|sender| {
                        println!("sent");
                        sender
                    })
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, "..."))
            })
            .map(|_| {
                ()
            })
            .map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "...")
            });

        writer
    });

    core.run(client).unwrap();
}

fn simulated_messaging_receiving_from_clients(buftx: UnboundedSender<String>,
                                              handle: &Handle)
                                              -> () {

    let timer = Timer::default();
    let wakeups = timer.interval(Duration::new(2, 0));
    let mut i = 0;
    let background_tasks = wakeups.for_each(move |_| {
        println!("Interval");
        i = i + 1;
        let f = buftx.clone()
            .send(format!("Messagio {}", i).to_string())
            .map(|_| ())
            .map_err(|_| TimerError::NoCapacity);

        f
    });

    handle.spawn(background_tasks.map(|_| ()).map_err(|_| ()));
}



struct LineCodec;
impl Codec for LineCodec {
    type In = String;
    type Out = String;

    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, io::Error> {
        // If our buffer contains a newline...
        if let Some(n) = buf.as_ref().iter().position(|b| *b == b'\n') {
            // remove this line and the newline from the buffer.
            let line = buf.drain_to(n);
            buf.drain_to(1); // Also remove the '\n'.

            // Turn this data into a UTF-8 string and return it
            return match str::from_utf8(line.as_ref()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "invalid string")),
            }
        }

        // Otherwise, we don't have enough data for a full message yet
        Ok(None)
    }

    fn encode(&mut self, msg: Self::Out, buf: &mut Vec<u8>) -> io::Result<()> {
        for byte in msg.as_bytes() {
            buf.push(*byte);
        }

        buf.push(b'\n');
        Ok(())
    }
}
