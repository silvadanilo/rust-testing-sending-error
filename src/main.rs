extern crate futures;
extern crate tokio_core;
extern crate tokio_line;
extern crate tokio_timer;
extern crate tokio_io;

use futures::future::{Future};
use futures::{Sink, Stream};
use futures::sync::mpsc::{self, UnboundedSender};
use tokio_timer::*;
use tokio_core::io::{Io};
use tokio_core::net::{TcpStream};
use tokio_core::reactor::{Core, Handle};
use std::time::Duration;
use std::{io, str};

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let (tx, rx) = mpsc::unbounded();
    simulating_messaging_receiving_from_clients(tx, &handle);

    let remote_addr = "127.0.0.1:9876".parse().unwrap();
    let tcp = TcpStream::connect(&remote_addr, &handle);

    let client = tcp.and_then(move |stream| {
        let (receiver, sender) = stream.split();

        let writer = rx
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "..."))
            .fold(sender, |sender, msg| {
                println!("before sending `{}`", msg);
                let msg = format!("{}\n", msg);
                tokio_core::io::write_all(sender, msg.into_bytes())
                    .map(|(sender, _)| {
                        println!("sent");
                        sender
                    })
            })
        ;

        writer
    });

    core.run(client).unwrap();
}

fn simulating_messaging_receiving_from_clients(buftx: UnboundedSender<String>,
                                              handle: &Handle)
                                              -> () {

    let timer = Timer::default();
    let wakeups = timer.interval(Duration::new(2, 0));
    let mut i = 0;
    let background_tasks = wakeups.for_each(move |_| {
        println!("Interval");
        i = i + 1;
        let f = buftx.clone()
            .send(format!("Message -> {}", i).to_string())
            .map(|_| ())
            .map_err(|_| TimerError::NoCapacity);

        f
    });

    handle.spawn(background_tasks.map(|_| ()).map_err(|_| ()));
}
