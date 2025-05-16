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
                //let response_404 = "HTTP/1.1 404 Not Found\r\n\r\n";
                let mut request = [0u8; 1024];
                //let mut request_string: String = String::new();
                let request_head = "GET /echo/";
                //let request_tail = "HTTP/1.1\r\nHost: localhost:4221\r\n\r\n";

                // Read the request data
                let _size = stream.read(&mut request).unwrap();
                let request_string = String::from_utf8(request.to_vec()).unwrap();
                println!("Request String: {}", request_string);

                match request_string.strip_prefix(request_head) {
                    Some(partial) => {
                        let (head, _tail) = partial.split_once(" ").unwrap();
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                            head.len(),
                            head
                        );
                        stream.write(response.as_bytes()).unwrap();
                    }
                    None => {
                        if request_string.starts_with("GET / ") {
                            let response = "HTTP/1.1 200 OK\r\n\r\n";
                            stream.write(response.as_bytes()).unwrap();
                        } else {
                            let response = "HTTP/1.1 404 Not Found\r\n\r\n";
                            stream.write(response.as_bytes()).unwrap();
                        }
                    }
                }
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
