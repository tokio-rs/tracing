/*
This example has been taken and modified from here :
https://raw.githubusercontent.com/tokio-rs/tokio/master/tokio/examples/proxy.rs
*/

#![deny(warnings)]

extern crate futures;
extern crate tokio;
#[macro_use]
extern crate tokio_trace;
extern crate tokio_trace_fmt;
extern crate tokio_trace_futures;

use tokio_trace::field;
use tokio_trace_futures::Instrument;

use std::env;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr};
use std::sync::{Arc, Mutex};

use tokio::io::{copy, shutdown};
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

fn main() -> Result<(), Box<std::error::Error>> {
    let listen_addr = env::args().nth(1).unwrap_or("127.0.0.1:8081".to_string());
    let listen_addr = listen_addr.parse::<SocketAddr>()?;

    let server_addr = env::args().nth(2).unwrap_or("127.0.0.1:3000".to_string());
    let server_addr = server_addr.parse::<SocketAddr>()?;

    // Create a TCP listener which will listen for incoming connections.
    let socket = TcpListener::bind(&listen_addr)?;
    println!("Listening on: {}", listen_addr);
    println!("Proxying to: {}", server_addr);

    let done = socket
        .incoming()
        .map_err(|e| debug!(msg = "error accepting socket", error = field::display(e)))
        .for_each(move |client| {
            let server = TcpStream::connect(&server_addr);
            match client.peer_addr() {
                Ok(x) => info!(
                    message = "client connected",
                    client_addr = field::display(x)
                ),
                Err(e) => debug!(
                    message = "could not get client info",
                    error = field::display(e)
                ),
            }

            let amounts = server.and_then(move |server| {
                // Create separate read/write handles for the TCP clients that we're
                // proxying data between. Note that typically you'd use
                // `AsyncRead::split` for this operation, but we want our writer
                // handles to have a custom implementation of `shutdown` which
                // actually calls `TcpStream::shutdown` to ensure that EOF is
                // transmitted properly across the proxied connection.
                //
                // As a result, we wrap up our client/server manually in arcs and
                // use the impls below on our custom `MyTcpStream` type.
                let client_reader = MyTcpStream(Arc::new(Mutex::new(client)));
                let client_writer = client_reader.clone();
                let server_reader = MyTcpStream(Arc::new(Mutex::new(server)));
                let server_writer = server_reader.clone();

                // Copy the data (in parallel) between the client and the server.
                // After the copy is done we indicate to the remote side that we've
                // finished by shutting down the connection.
                let client_to_server = copy(client_reader, server_writer)
                    .and_then(|(n, _, server_writer)| {
                        info!(size = n);
                        shutdown(server_writer).map(move |_| n)
                    })
                    .instrument(span!("client_to_server"));

                let server_to_client = copy(server_reader, client_writer)
                    .and_then(|(n, _, client_writer)| {
                        info!(size = n);
                        shutdown(client_writer).map(move |_| n)
                    })
                    .instrument(span!("server_to_client"));

                client_to_server.join(server_to_client)
            });

            let msg = amounts
                .map(move |(from_client, from_server)| {
                    info!(
                        client_to_server = from_client,
                        server_to_client = from_server
                    );
                })
                .map_err(|e| {
                    // Don't panic. Maybe the client just disconnected too soon.
                    debug!(error = field::display(e));
                })
                .instrument(span!("transfer"));

            tokio::spawn(msg);

            Ok(())
        })
        .instrument(span!("proxy", listen_addr = field::debug(&listen_addr)));

    let subscriber = tokio_trace_fmt::FmtSubscriber::builder().full().finish();
    tokio_trace::subscriber::with_default(subscriber, || {
        tokio::run(done);
    });

    Ok(())
}

// This is a custom type used to have a custom implementation of the
// `AsyncWrite::shutdown` method which actually calls `TcpStream::shutdown` to
// notify the remote end that we're done writing.
#[derive(Clone)]
struct MyTcpStream(Arc<Mutex<TcpStream>>);

impl Read for MyTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.lock().unwrap().read(buf)
    }
}

impl Write for MyTcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl AsyncRead for MyTcpStream {}

impl AsyncWrite for MyTcpStream {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        try!(self.0.lock().unwrap().shutdown(Shutdown::Write));
        Ok(().into())
    }
}
