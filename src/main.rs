extern crate hyper;
extern crate futures;
extern crate reqwest;

use futures::future::{Future, ok};
use hyper::{
    header::ContentLength,
    server::{Http, Request, Response, Service}
};

struct Server;
impl Server {
    fn send_request(&self, req: Request) -> Option<String> {
        let uri = format!("{}", req.uri());
        println!("{}", uri);
        // TODO
        if !uri.contains(":443") {
            let text = reqwest::get(&uri).unwrap().text().unwrap();
            Some(text)
        } else {
            None
        }
    }
}
impl Service for Server {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let body = self.send_request(req);
        let body = body.map_or("".to_string(), |s| s);
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