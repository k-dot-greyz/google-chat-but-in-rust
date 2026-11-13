//! Inbound HTTP only. There is no client socket in this process.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::http::{self, HttpError, Parse, Response};
use crate::hull::Hull;

const MAX_CONNECTIONS: usize = 64;

pub fn bind(addr: &str) -> std::io::Result<TcpListener> {
    TcpListener::bind(addr)
}

pub fn serve(listener: TcpListener, hull: Arc<Mutex<Hull>>) {
    let slots = Arc::new(AtomicUsize::new(0));
    loop {
        let Ok((stream, _)) = listener.accept() else {
            continue;
        };
        if slots.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
            let _ = write_all(
                &stream,
                &Response::fail(503, "busy", "The pass is full. Try again.").encode(),
            );
            continue;
        }
        slots.fetch_add(1, Ordering::SeqCst);
        let hull = Arc::clone(&hull);
        let slots = Arc::clone(&slots);
        thread::spawn(move || {
            let _release = Release(slots);
            handle(stream, &hull);
        });
    }
}

struct Release(Arc<AtomicUsize>);

impl Drop for Release {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn handle(mut stream: TcpStream, hull: &Mutex<Hull>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let response = match read_request(&mut stream) {
        Ok(request) => {
            let mut guard = hull.lock().unwrap_or_else(|poison| poison.into_inner());
            let dispatched =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| guard.dispatch(&request)));
            match dispatched {
                Ok(response) => response,
                Err(_) => {
                    Response::fail(500, "panic", "The hull caught a panic and kept the rest.")
                }
            }
        }
        Err(error) => match error {
            HttpError::TooLarge => Response::fail(
                413,
                "too_large",
                "That body does not fit through the hatch.",
            ),
            HttpError::LengthRequired => {
                Response::fail(411, "length", "Content-Length is required.")
            }
            HttpError::Unsupported => {
                Response::fail(400, "encoding", "Chunked bodies are not accepted.")
            }
            HttpError::Incomplete => {
                Response::fail(400, "incomplete", "The request stopped short.")
            }
            HttpError::Bad(detail) => Response::fail(400, "bad_request", &detail),
        },
    };
    eprintln!("hull {} {}", response.status, response.body.len());
    let _ = write_all(&stream, &response.encode());
}

fn read_request(stream: &mut TcpStream) -> Result<http::Request, HttpError> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        match http::try_parse(&buf) {
            Parse::Done(request) => return Ok(request),
            Parse::Need => {}
            Parse::Err(error) => return Err(error),
        }
        if buf.len() > http::MAX_HEADER + http::MAX_BODY {
            return Err(HttpError::TooLarge);
        }
        let read = stream.read(&mut tmp).map_err(|_| HttpError::Incomplete)?;
        if read == 0 {
            return Err(HttpError::Incomplete);
        }
        buf.extend_from_slice(&tmp[..read]);
    }
}

fn write_all(mut stream: &TcpStream, bytes: &[u8]) -> std::io::Result<()> {
    stream.write_all(bytes)
}
