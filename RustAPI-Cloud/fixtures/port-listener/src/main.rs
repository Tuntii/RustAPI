use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&addr).expect("bind failed");
    println!("listening on {}", addr);

    if let Ok((mut stream, _)) = listener.accept() {
        let _ = stream.write_all(b"ok");
        let mut buf = [0u8; 16];
        let _ = stream.read(&mut buf);
    }

    loop {
        if let Ok((_, _)) = listener.accept() {
            // keep process alive for deploy status checks
        }
    }
}