use std::io::{Read, Write};
#[allow(unused_imports)]
use std::net::TcpListener;
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                // Hello World
                println!("accepted new connection");

                // Different resonses and requets
                let response_404 = "HTTP/1.1 404 Not Found\r\n\r\n";
                let mut request: Vec<u8> = Vec::new();
                let request_head = "GET /echo/";
                let request_tail = "HTTP/1.1\r\nHost: localhost:4221\r\n\r\n";

                // Read the request data
                let _size = stream.read(&mut request).unwrap();
                let request_string = String::from_utf8(request.clone()).unwrap();

                // Compare the requests data with the partial expected for a 200, otherwise respond with a 404
                if request_string.starts_with(request_head) {
                    let partial = request_string.strip_prefix(request_head).unwrap();
                    let (head, tail) = partial.split_once(" ").unwrap();
                    if request_tail.eq_ignore_ascii_case(tail) {
                        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", head.len(), head);
                        stream.write(response.as_bytes()).unwrap();
                    } else {
                        stream.write(response_404.as_bytes()).unwrap();
                    }
                } else {
                    stream.write(response_404.as_bytes()).unwrap();
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
