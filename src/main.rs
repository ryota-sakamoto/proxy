extern crate reqwest;

use std::{
    env::{
        args,
        Args
    },
    io::prelude::*,
    net::{
        Shutdown,
        TcpListener,
        TcpStream
    }
};

fn main() {
    let port = get_port(&mut args()).expect("Invalid Port");
    let server_ip = format!("127.0.0.1:{}", port);
    let server = TcpListener::bind(server_ip.clone()).expect("Can not bind");
    println!("Start Proxy Server: {}", server_ip);
    for stream in server.incoming() {
        let mut stream = stream.unwrap();
        std::thread::spawn(move || {
            match handle(&mut stream) {
                Err(e) => {
                    println!("Err = {:?}", e);
                },
                _ => {}
            }
        });
    }
}

fn get_port(arg: &mut Args) -> Result<u16, std::num::ParseIntError> {
    let mut port = 8080;
    let mut use_env = false;
    while let Some(elem) = arg.next() {
        if elem == "-p" {
            use_env = true;
        } else if use_env {
            port = elem.parse()?;
            break;
        }
    }
    Ok(port)
}

fn handle(stream: &mut TcpStream) -> Result<(), Box<std::error::Error>> {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let data = std::str::from_utf8(&buf[0..n])?;
    let header = data.split("\r\n")
        .filter(|s| s != &"")
        .collect::<Vec<&str>>();

    let request_header = header[0].split(" ").collect::<Vec<&str>>();
    let method = request_header[0];
    let mut uri = request_header[1].to_string();
    println!("{} {}", method, uri);
    if uri.contains(":443") {
        uri = format!("https://{}", uri.replace(":443", ""));
    }
    let uri = &uri;
    // let protcol = request_header[2];

    if start_with(uri, "http://") || start_with(uri, "https://") {
        match method {
            "GET" => {
                let mut r = reqwest::get(uri)?;
                stream.write(b"HTTP/1.1 200 OK\r\n")?;
                for item in r.headers().iter() {
                    if !item.name().contains("Transfer-Encoding") {
                        stream.write(format!("{}", item).as_bytes())?;
                    }
                }
                let mut buf = Vec::new();
                r.copy_to(&mut buf)?;
                stream.write(format!("Content-Length: {}\r\n", buf.len()).as_bytes())?;
                stream.write(b"\r\n")?;
                stream.write(&buf)?;
                stream.write(b"\r\n")?;
                stream.shutdown(Shutdown::Both)?;
            }
            "CONNECT" => {
                stream.write(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;

                let mut ssl_stream =
                    TcpStream::connect(request_header[1])?;

                let mut a = stream.try_clone()?;
                let mut b = ssl_stream.try_clone()?;

                let mut a2 = stream.try_clone()?;
                let mut b2 = ssl_stream.try_clone()?;

                std::thread::spawn(move || {
                    let _ = std::io::copy(&mut b, &mut a);
                });
                std::thread::spawn(move || {
                    let _ = std::io::copy(&mut a2, &mut b2);
                });
            }
            m => {
                println!("unknown method: {}", m);
            }
        }
    } else {
        let res = local_route(&uri);
        stream.write(b"HTTP/1.1 200 OK\r\n\r\n")?;
        stream.write(res.as_bytes())?;
        stream.write(b"\r\n")?;
        stream.shutdown(Shutdown::Both)?;
    }
    Ok(())
}

fn local_route(uri: &str) -> String {
    format!("{}", uri)
}

fn start_with(elem: &str, t: &str) -> bool {
    if elem.len() < t.len() {
        false
    } else {
        let mut _elem = elem.chars();
        let mut result = true;
        for _t in t.chars() {
            if _elem.next().unwrap() != _t {
                result = false;
                break;
            }
        }
        result
    }
}

#[test]
fn start_with_test() {
    assert!(start_with("http://example.com", "http://"));
    assert!(!start_with("http://example.com", "https://"));
    assert!(!start_with("abcde", "abcd"));
}
