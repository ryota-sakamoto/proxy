extern crate hyper;
extern crate futures;
extern crate reqwest;

use std::{
    net::{TcpListener, Shutdown},
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
impl Service for Server {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let res = self.send_request(req);
        let body = res.body;
        Box::new(
            ok(
                Response::new()
                    .with_headers(res.headers)
                    .with_body(body)
            )
        )
    }
}

fn main() {
    let server = TcpListener::bind("192.168.3.5:8080").expect("Can not bind");
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
                            let uri = request_header[1];
                            let protcol = request_header[2];

                            if start_with(uri, "http://") || start_with(uri, "https://") {
                                match method {
                                    "GET" => {
                                        let r = reqwest::get(uri).unwrap();
                                        println!("{:?}", r);
                                    },
                                    "CONNECT" => {
                                        println!("{} {}", method, uri);
                                    },
                                    m => {
                                        println!("unknown method: {}", m);
                                    },
                                }
                            } else {
                                println!("local");
                            }
                            stream.write(b"HTTP/1.1 200 OK\r\nConnection: Close\r\n\r\n").expect("stream write error");
                            stream.shutdown(Shutdown::Both).expect("stream shutdown error");
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