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

#[derive(Debug)]
struct ClientHello {
    msg_type: u8,
    lengh: [u8; 3],
    client_version: [u8; 2],
    random: Vec<u8>,
    session_id: Vec<u8>,
    cipher_suite: Vec<u8>,
    compression_method : Vec<u8>,
    extensions: Vec<u8>,
}

impl ClientHello {
    fn get_handshake_type<'a>(&self) -> &'a str {
        match self.msg_type {
            0 => "HelloRequest",
            1 => "ClientHello",
            2 => "ServerHello",
            11 => "Certificate",
            12 => "ServerKeyExchange",
            13 => "CertificateRequest",
            14 => "ServerHelloDone",
            15 => "CertificateVerify",
            16 => "ClientKeyEnchange",
            _ => "Unknown",
        }
    }

    fn get_tls_version<'a>(&self) -> &'a str {
        match (self.client_version[0], self.client_version[1]) {
            (3, 0) => "SSLV3",
            (3, 1) => "TLSv1.0",
            (3, 2) => "TLSv1.1",
            (3, 3) => "TLSv1.2",
            _ => "Unknown",
        }
    }
}

// TODO refactoring
impl From<Vec<u8>> for ClientHello {
    fn from(vec: Vec<u8>) -> Self {
        let random = &vec[11..43];
        let session_id_end = 44 + vec[43] as usize;
        let session_id = &vec[44..session_id_end];
        let cipher_suite_length = [vec[session_id_end] as u64, vec[session_id_end + 1] as u64];
        let cipher_suite_length = (cipher_suite_length[0] << 8) + cipher_suite_length[1];
        let cipher_suite = &vec[session_id_end + 2 .. session_id_end + 2 + cipher_suite_length as usize];
        let compression_method_length_position = (session_id_end + 2 + cipher_suite.len()) as usize;
        let compression_method_length = vec[compression_method_length_position] as usize;
        let compression_method_end = compression_method_length_position + 1 + compression_method_length;
        let compression_method = &vec[compression_method_length_position + 1 .. compression_method_end];
        let extensions_length = [vec[compression_method_end] as u64, vec[compression_method_end + 1] as u64];
        let extensions_length = (extensions_length[0] << 8) + extensions_length[1];
        let extensions_start = compression_method_end + 2;
        let extensions = &vec[extensions_start..extensions_start + extensions_length as usize];

        ClientHello {
            msg_type: vec[5],
            lengh: [vec[6], vec[7], vec[8]],
            client_version: [vec[9], vec[10]],
            random: random.to_vec(),
            session_id: session_id.to_vec(),
            cipher_suite: cipher_suite.to_vec(),
            compression_method : compression_method.to_vec(),
            extensions: extensions.to_vec(),
        }
    }
}

// TODO refactoring
#[derive(Debug)]
struct ServerHello {
    msg_type: u8,
    lengh: [u8; 3],
    version: [u8; 2],
    random: Vec<u8>,
    session_id: Vec<u8>,
    cipher_suites: [u8; 2],
    compression_methods: u8,
    extensions: Vec<u8>,
}

impl From<Vec<u8>> for ServerHello {
    fn from(vec: Vec<u8>) -> Self {
        let random = &vec[11..43];
        let session_id_length = vec[43] as usize;
        let session_id = &vec[44 .. 44 + session_id_length];
        let cipher_suites_start = 44 + session_id_length;
        let extensions_length = [vec[cipher_suites_start + 3] as u64, vec[cipher_suites_start + 4] as u64];
        let extensions_length = (extensions_length[0] << 8) + extensions_length[1];
        let extensions = &vec[cipher_suites_start + 5 .. cipher_suites_start + 5 + extensions_length as usize];

        ServerHello {
            msg_type: vec[5],
            lengh: [vec[6], vec[7], vec[8]],
            version: [vec[9], vec[10]],
            random: random.to_vec(),
            session_id: session_id.to_vec(),
            cipher_suites: [vec[cipher_suites_start], vec[cipher_suites_start + 1]],
            compression_methods: vec[cipher_suites_start + 2],
            extensions: extensions.to_vec(),
        }
    }
}

// TODO refactoring
#[derive(Debug)]
struct CertificateMessage {
    msg_type: u8,
    lengh: [u8; 3],
    // first_certificate: Vec<u8>,
    // second_certificate: Vec<u8>,
    certificate: Vec<u8>,
}

impl From<Vec<u8>> for CertificateMessage {
    fn from(vec: Vec<u8>) -> Self {
        let certificate_length = [vec[9] as u64, vec[10] as u64, vec[11] as u64];
        let certificate_length = ((certificate_length[0] << 16) + (certificate_length[1] << 8) + certificate_length[2]) as usize;
        let certificate = &vec[12 .. 12 + certificate_length];

        CertificateMessage{
            msg_type: vec[5],
            lengh: [vec[6], vec[7], vec[8]],
            certificate: certificate.to_vec(),
        }
    }
}

// TODO refactoring
#[derive(Debug)]
struct ClientKeyEnchange {
    data: Vec<u8>,
}

impl From<Vec<u8>> for ClientKeyEnchange {
    fn from(vec: Vec<u8>) -> Self {
        let length = [vec[6] as u64, vec[7] as u64, vec[8] as u64];
        let length = ((length[0] << 16) + (length[1] << 8) + length[2]) as usize;
        let data = &vec[9 .. length + 9];

        ClientKeyEnchange {
            data: data.to_vec(),
        }
    }
}

#[derive(Debug)]
struct ChangeCipherSpec {
    record_type: u8,
    version: [u8; 2],
    data: u8,
}

impl From<Vec<u8>> for ChangeCipherSpec {
    fn from(vec: Vec<u8>) -> Self {
        ChangeCipherSpec {
            record_type: vec[0],
            version: [vec[1], vec[2]],
            data: vec[4],
        }
    }
}

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

                let mut buf = [0u8; 1024];
                let n = stream.read(&mut buf)?;
                ssl_stream.write(&buf[..n])?;

                let mut res = [0u8; 32 * 1024];
                let n = ssl_stream.read(&mut res)?;
                stream.write(&res[..n])?;

                let mut buf = [0u8; 1024];
                let n = stream.read(&mut buf)?;
                ssl_stream.write(&buf[..n])?;

                let mut buf = [0u8; 1024];
                let n = ssl_stream.read(&mut buf)?;
                stream.write(&buf[..n])?;

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
