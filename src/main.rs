extern crate hyper;
extern crate futures;
extern crate reqwest;

use futures::future::{Future, ok};
use hyper::{
    header::{ContentLength, ContentType},
    Method,
    server::{Http, Request, Response, Service}
};

struct ProxyResponse {
    method: Method,
    body: String,
}
impl ProxyResponse {
    fn new(text: &str, method: Method) -> Self {
        ProxyResponse {
            method: method,
            body: text.to_string(),
        }
    }

    fn parse_response(res: &mut reqwest::Response, method: Method) -> Self {
        let h = res.headers().get::<ContentType>();
        println!("{:?}", h);
        ProxyResponse {
            method: method,
            body: res.text().unwrap(),
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
        println!("{} {}", req.method(), uri);
        let method = req.method().clone();
        if uri.contains("http://") || uri.contains("https://") {
            match method {
                Method::Get => {
                    let mut res = reqwest::get(&uri).unwrap();
                    ProxyResponse::parse_response(&mut res, method)
                },
                Method::Connect => {
                    println!("{:?}", req);
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
                    .with_header(ContentLength(body.len() as u64))
                    .with_body(body)
            )
        )
    }
}

fn main() {
    let addr = "192.168.3.5:8080".parse().unwrap();
    let server = Http::new().bind(&addr, || Ok(Server)).unwrap();
    server.run().unwrap();
}