extern crate hyper;
extern crate futures;
extern crate reqwest;

use std::{
    net::{TcpStream, TcpListener, Shutdown},
    io::prelude::*,
};
use futures::future::{Future, ok};
use hyper::{
    header::{Headers, ContentLength, ContentType},
    Method,
    server::{Http, Request, Response, Service}
};

#[derive(Debug)]
struct ProxyResponse {
    method: Method,
    headers: Headers,
    body: String,
}
impl ProxyResponse {
    fn new(text: &str, method: Method) -> Self {
        ProxyResponse {
            method: method,
            body: text.to_string(),
            headers: Headers::new(),
        }
    }

    fn parse_response(res: &mut reqwest::Response, method: Method) -> Self {
        let header = res.headers().clone();
        ProxyResponse {
            method: method,
            body: res.text().unwrap(),
            headers: header,
        }
    }
}
struct Server;
impl Server {
    fn send_request(&self, req: Request) -> ProxyResponse {
        let uri = if req.method() == &Method::Connect {
            format!("https://{}", format!("{}", req.uri()))            
        } else {
            format!("{}", req.uri())
        };
        let method = req.method().clone();
            if uri.contains("http://") || uri.contains("https://") {
            match method {
                Method::Get => {
                    let mut res = reqwest::get(&uri).unwrap();
                    ProxyResponse::parse_response(&mut res, method)
                },
                Method::Connect => {
                    ProxyResponse::new("", method)
                },
                _ => {
                    ProxyResponse::new("", method)
                }
            }
        } else {
            ProxyResponse::new("you mut use proxy", method)
        }
    }
}

fn main() {
    let server = TcpListener::bind("192.168.3.3:8080").expect("Can not bind");
    for stream in server.incoming() {
        let mut stream = stream.unwrap();
        let mut buf = [0u8; 1024];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
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
                                        // println!("{:?}", r);
                                        stream.write(b"HTTP/1.1 200 OK\r\nConnection: Close\r\n\r\n").expect("stream write error");
                                        stream.shutdown(Shutdown::Both).expect("stream shutdown error");
                                    },
                                    "CONNECT" => {
                                        stream.write(b"HTTP/1.1 200 Connection Established\r\n\r\n").expect("stream write error");

                                        let mut ssl_stream = TcpStream::connect(request_header[1]).expect("open stream connect error");

                                        let mut buf = [0u8; 1024];
                                        stream.read(&mut buf).expect("stream read error 2");
                                        ssl_stream.write(&buf).expect("stream weite error 2");

                                        let mut res = [0u8; 1024];
                                        ssl_stream.read(&mut res);
                                        stream.write(&res);
                                        
                                        let mut res2 = [0u8; 32 * 1024];
                                        while let Ok(n) = ssl_stream.read(&mut res2) {
                                            if n == 0 {
                                                break;
                                            }
                                            stream.write(&res2[..n]);
                                        }

                                        while let Ok(n) = stream.read(&mut res2) {
                                            if n == 0 {
                                                break;
                                            }
                                            ssl_stream.write(&res2[..n]);                                            
                                        }
                                    },
                                    m => {
                                        println!("unknown method: {}", m);
                                    },
                                }
                            } else {
                                println!("local {:?}", uri);
                            }
                        },
                        _ => break,
                    }
                },
                _ => break,
            }
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