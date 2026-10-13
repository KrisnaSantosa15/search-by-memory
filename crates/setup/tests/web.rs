//! The web adapter, against a real HTTP server on this machine.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use sbm_setup::{Http, Web};

const CONTENT: &[u8] = b"0123456789abcdefghij";

/// Serves CONTENT on a local port for one request; `ranges` decides whether it
/// honours a Range header. Returns the base URL.
fn serve_once(status_line: &'static str, ranges: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/model.onnx", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut from = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let lower = line.to_ascii_lowercase();
            if let Some(range) = lower.strip_prefix("range: bytes=") {
                from = range.trim().trim_end_matches('-').parse().unwrap();
            }
            if line == "\r\n" {
                break;
            }
        }
        let (status, body) = match status_line {
            "404" => ("404 Not Found", &b""[..]),
            _ if ranges && from > 0 => ("206 Partial Content", &CONTENT[from..]),
            _ => ("200 OK", CONTENT),
        };
        let mut stream = stream;
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(body).unwrap();
    });
    url
}

fn body(http: &Web, url: &str, from: u64) -> Vec<u8> {
    let mut body = Vec::new();
    http.get(url, from).unwrap().read_to_end(&mut body).unwrap();
    body
}

#[test]
fn web_download_asks_for_the_rest_of_the_file() {
    let url = serve_once("ok", true);

    assert_eq!(body(&Web::new(), &url, 15), b"fghij");
}

#[test]
fn server_that_ignores_ranges_still_yields_only_the_rest_of_the_file() {
    let url = serve_once("ok", false);

    assert_eq!(body(&Web::new(), &url, 15), b"fghij");
}

#[test]
fn missing_file_is_an_error() {
    let url = serve_once("404", true);

    assert!(Web::new().get(&url, 0).is_err());
}
