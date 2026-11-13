use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use hull::Hull;
use hull::server;

#[test]
fn tcp_health_answers() {
    let listener = server::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hull = Arc::new(Mutex::new(Hull::boot_embedded()));
    thread::spawn(move || server::serve(listener, hull));
    thread::sleep(Duration::from_millis(50));
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: hull\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    let text = String::from_utf8_lossy(&buf);
    assert!(text.contains("200 OK"), "{text}");
    assert!(text.contains("\"service\":\"hull\""), "{text}");
    assert!(text.contains("nosniff"), "{text}");
}
