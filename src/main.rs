use std::io::{Read, Write};
#[allow(unused_imports)]
#[allow(dead_code)]
use std::net::TcpListener;

const VERBS: [&str; 6] = ["GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"];
const HOST: &str = "localhost:4221";
const HEADERS: [&str; 5] = [
    "Content-Type",
    "Content-Length",
    "User-Agent",
    "Accept",
    "Host",
];
const PROTOCOL: &str = "HTTP/1.1";
const RESPONSE_404: &str = "HTTP/1.1 404 Not Found\r\n\r\n";
const RESPONSE_200: &str = "HTTP/1.1 200 OK\r\n\r\n";

#[derive(Debug)]
struct HTTPRequest {
    verb: String,
    path: String,
    host: String,
    user_agent: Option<String>,
    content_type: Option<String>,
    content_length: Option<u32>,
    accept: Option<String>,
    body: String,
}

fn extract_parts_and_body(request: &str) -> Option<HTTPRequest> {
    // Debug
    println!("Request: {}", request);
    // Split different elements
    let elements: Vec<&str> = request.split("\r\n\r\n").collect();

    // We expect at least the parts and body (even if empty)
    if elements.len() < 2 {
        println!("Invalid request format. Couldn't find parts and body");
        return None;
    }
    // parts and body
    let parts = elements[0];
    let body = elements[1].to_string();

    // remove the HTTP version
    let parts_elements: Vec<&str> = parts.split(" HTTP/1.1\r\n").collect();
    if parts_elements.len() < 2 {
        println!("Invalid request format. Couldn't find HTTP version");
        return None;
    }
    // handle incorrect request
    if parts_elements[0].split(" ").count() != 2 {
        println!("Invalid request format. Incorrect request");
        return None;
    }
    // Exctract verb and path
    let verb_and_path: Vec<&str> = parts_elements[0].split(" ").collect();
    if verb_and_path.len() != 2 {
        println!("Invalid request format. Couldn't find verb and path");
        return None;
    }
    let verb = verb_and_path[0].to_string();
    let path = verb_and_path[1].to_string();

    // Handle the case where we have a GET request with a body
    // if verb.eq("GET") && !body.is_empty() {
    //     println!("Invalid request format. GET request with body: {}", body);
    //     return None;
    // }

    // Now headers
    let mut host: String = String::new();
    let mut user_agent: Option<String> = None;
    let mut content_length: Option<u32> = None;
    let mut content_type: Option<String> = None;
    let mut accept: Option<String> = None;

    // Raw headers
    let headers_raw = parts_elements[1];

    // Split each header
    let headers_split: Vec<&str> = headers_raw.split("\r\n").collect();

    // No match, then check for host
    if headers_split.len() < 2 {
        let headers_single: Vec<&str> = headers_raw.split(":").collect();
        // No host, not normal
        if headers_single.len() != 2 {
            println!("No host header found !");
            return None;
        } else {
            if !headers_single[0].eq("Host") {
                println!(
                    "Invalid header format. Expected 'Host', found '{}'",
                    headers_single[0]
                );
                return None;
            }
            host = headers_single[1].to_string();
        }
    }
    let mut has_invalid_header: bool = false;
    let mut has_content_length = false;

    let _map = headers_split.iter().map(|header| {
        let parts: Vec<&str> = header.split(": ").collect();
        if parts.len() != 2 {
            has_invalid_header = true;
        }

        match parts[0] {
            "Host" => host = parts[1].to_string(),
            "User-Agent" => user_agent = Some(parts[1].to_string()),
            "Content-Length" => {
                has_content_length = true;
                content_length = Some(parts[1].parse().unwrap_or(0));
            }
            "Content-Type" => content_type = Some(parts[1].to_string()),
            "Accept" => accept = Some(parts[1].to_string()),
            _ => {}
        }
    });
    // We have an invalid header
    if has_invalid_header {
        println!("Invalid header format found among headers !");
        return None;
    }
    // Host was not set
    if host.is_empty() {
        println!("Host header not found among headers !");
        return None;
    }
    // Unknown verb
    if !VERBS.contains(&verb.as_str()) {
        println!("Unknown verb found used in request !");
        return None;
    }
    // Body sent without Content-Length header
    if !body.is_empty() && !has_content_length {
        println!("Body sent without Content-Length header !");
        return None;
    }

    Some(HTTPRequest {
        verb,
        path,
        host,
        user_agent,
        content_type,
        content_length,
        accept,
        body,
    })
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                // Read the request data
                let mut request: [u8; 1024] = [0; 1024];
                let _size = match stream.read(&mut request) {
                    Ok(size) => size,
                    Err(e) => {
                        println!("error reading stream: {}", e);
                        continue;
                    }
                };
                // convert to string
                let request_string = match String::from_utf8(request.to_vec()) {
                    Ok(request_string) => request_string,
                    Err(e) => {
                        println!("error converting request to string: {}", e);
                        continue;
                    }
                };
                // Get HTTP Request struct
                let request: HTTPRequest = match extract_parts_and_body(request_string.as_str()) {
                    Some(request) => request,
                    None => {
                        println!("error extracting parts and body from request");
                        continue;
                    }
                };
                // Current exercise: / is 200 OK and /user-agent is 200 OK with the user agent
                match request.verb.as_str() {
                    "GET" => match request.path.as_str() {
                        "/" => match stream.write(RESPONSE_200.as_bytes()) {
                            Ok(_) => {
                                continue;
                            }
                            Err(e) => {
                                println!("error writing response: {}", e);
                                continue;
                            }
                        },
                        "/user-agent" => {
                            // Extract user agent
                            let user_agent: String = match request.user_agent {
                                Some(user_agent) => user_agent,
                                None => "Unknown".to_string(),
                            };
                            // Respond 200 OK with user_agent
                            match stream.write(format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}", user_agent.len(), user_agent).as_bytes()) {
                                Ok(_) => {
                                    continue;
                                }
                                Err(e) => {
                                    println!("error writing response: {}", e);
                                    continue;
                                }
                            }
                        }
                        _ => {
                            // Response 404 Not Found
                            match stream.write("HTTP/1.1 404 Not Found\r\n\r\n".as_bytes()) {
                                Ok(_) => {
                                    continue;
                                }
                                Err(e) => {
                                    println!("error writing response: {}", e);
                                    continue;
                                }
                            }
                        }
                    },
                    _ => continue,
                }
            }
            Err(e) => {
                println!("error in incoming stream: {}", e);
            }
        }
    }
}
