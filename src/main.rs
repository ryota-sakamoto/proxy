extern crate reqwest;

use std::{
    net::{TcpStream, TcpListener, Shutdown},
    io::prelude::*,
};

fn main() {
    let server = TcpListener::bind("192.168.3.3:8080").expect("Can not bind");
    for stream in server.incoming() {
        let mut stream = stream.unwrap();
        let mut buf = [0u8; 1024];
        match stream.read(&mut buf) {
            Ok(n) => {
                if n == 0 {
                    continue;
                }
                match std::str::from_utf8(&buf[0..n]) {
                    Ok(data) => {
                        let header = data.split("\r\n").filter(|s| s != &"").collect::<Vec<&str>>();

                        let request_header = header[0].split(" ").collect::<Vec<&str>>();
                        let method = request_header[0];
                        let mut uri = request_header[1].to_string();
                        println!("{} {}", method, uri);
                        if uri.contains(":443") {
                            uri = format!("https://{}", uri.replace(":443", ""));
                        }
                        let uri = &uri;
                        let protcol = request_header[2];

                        if start_with(uri, "http://") || start_with(uri, "https://") {
                            match method {
                                "GET" => {
                                    let r = reqwest::get(uri).unwrap();
                                    stream.write(b"HTTP/1.1 200 OK\r\nConnection: Close\r\n\r\n").expect("stream write error");
                                    stream.shutdown(Shutdown::Both).expect("stream shutdown error");
                                },
                                "CONNECT" => {
                                    stream.write(b"HTTP/1.1 200 Connection Established\r\n\r\n").expect("stream write error");

                                    let mut ssl_stream = TcpStream::connect(request_header[1]).expect("open stream connect error");

                                    let mut a = stream.try_clone().unwrap();
                                    let mut b = ssl_stream.try_clone().unwrap();
                                    
                                    std::thread::spawn(move || {
                                        std::io::copy(&mut b, &mut a).unwrap();
                                    });
                                    std::thread::spawn(move || {
                                        std::io::copy(&mut stream, &mut ssl_stream).unwrap();
                                    });
                                },
                                m => {
                                    println!("unknown method: {}", m);
                                },
                            }
                        } else {
                            println!("local {:?}", uri);
                        }
                    },
                    _ => {},
                }
            },
            Err(e) => panic!(e),
        }
    }
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