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
                let response_200 = "HTTP/1.1 200 OK\r\n\r\n";
                let response_404 = "HTTP/1.1 404 Not Found\r\n\r\n";
                let request_200_partial = "GET / HTTP/1.1\r\n";
                let mut request: [u8; 1024] = [0; 1024];

                // Read the request data
                let _size = stream.read(&mut request).unwrap();

                // Compare the requests data with the partial expected for a 200, otherwise respond with a 404
                if request.starts_with(&request_200_partial.as_bytes()) {
                    stream.write(response_200.as_bytes()).unwrap();
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
