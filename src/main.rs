#[allow(dead_code)]
//use regex::Regex;
use clap::Parser;
use std::sync::Arc;
use std::{fs, io};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

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
// Not found
const RESPONSE_404: &str = "HTTP/1.1 404 Not Found\r\n\r\n";
// All ok
const RESPONSE_200: &str = "HTTP/1.1 200 OK\r\n\r\n";
// bad request
const RESPONSE_400: &str = "HTTP/1.1 400 Bad Request\r\n\r\n";

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value = "public")]
    directory: String,
}

#[derive(Debug, Clone)]
struct HTTPRequest {
    verb: String,
    path: String,
    host: String,
    user_agent: Option<String>,
    content_type: Option<String>,
    content_length: Option<u32>,
    accept: Option<String>,
    body: Option<Vec<u8>>,
}

fn extract_parts_and_body(mut request: Vec<u8>) -> Option<HTTPRequest> {
    // Check if we found the CRLF in the request
    let crlf_pattern = [13, 10, 13, 10];
    let crlf_index = match request
        .windows(crlf_pattern.len())
        .position(|window| window == crlf_pattern)
    {
        Some(index) => index + 3,
        None => {
            tracing::error!("No CRLF found");
            return None;
        }
    };
    let body: Option<Vec<u8>>;
    let parts_raw: Vec<u8> = request.drain(..crlf_index - 3).collect();
    let _crlf_raw: Vec<u8> = request.drain(..4).collect();
    let body_raw = request;
    let parts = match String::from_utf8(parts_raw) {
        Ok(parts_str) => parts_str,
        Err(_) => {
            tracing::error!("Invalid parts");
            return None;
        }
    };

    if body_raw.is_empty() {
        body = None;
    } else {
        body = Some(body_raw.clone());
    }

    // // Split different elements
    // let elements: Vec<&str> = request.split("\r\n\r\n").collect();

    // // We expect at least the parts and body (even if empty)
    // if elements.len() < 2 {
    //     return None;
    // }
    // parts and body
    // let parts = elements[0];
    // let body = elements[1].to_string();

    // remove the HTTP version
    let parts_elements: Vec<&str> = parts.as_str().split(" HTTP/1.1\r\n").collect();
    if parts_elements.len() < 2 {
        return None;
    }
    // handle incorrect request
    if parts_elements[0].split(" ").count() != 2 {
        return None;
    }
    // Exctract verb and path
    let verb_and_path: Vec<&str> = parts_elements[0].split(" ").collect();
    if verb_and_path.len() != 2 {
        return None;
    }
    let verb = verb_and_path[0].to_string();
    let path = verb_and_path[1].to_string();

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
        let headers_single: Vec<&str> = headers_raw.split(": ").collect();
        // No host, not normal
        if headers_single.len() != 2 {
            return None;
        } else {
            if !headers_single[0].eq("Host") {
                return None;
            }
            host = headers_single[1].to_string();
        }
    } else {
        for header in headers_split {
            let parts: Vec<&str> = header.split(": ").collect();
            if parts.len() != 2 {
                return None;
            }

            match parts[0] {
                "Host" => {
                    host = parts[1].to_string();
                }
                "User-Agent" => {
                    user_agent = Some(parts[1].to_string());
                }
                "Content-Length" => {
                    content_length = Some(parts[1].parse().unwrap_or(0));
                }
                "Content-Type" => {
                    content_type = Some(parts[1].to_string());
                }
                "Accept" => {
                    accept = Some(parts[1].to_string());
                }
                _ => {}
            }
        }
    }
    // Host was not set
    if host.is_empty() {
        return None;
    }
    // Unknown verb
    if !VERBS.contains(&verb.as_str()) {
        return None;
    }

    // Checks on body and content length
    match content_length {
        Some(length) => match body.clone() {
            Some(body_value) => {
                if body_value.len() != length as usize {
                    tracing::error!(
                        "Body length: {} does not match Content-Length header: {}",
                        body_value.len(),
                        length
                    );
                    return None;
                }
            }
            None => {
                if length != 0 {
                    tracing::error!("No body found but found content length: {}", length);
                    return None;
                }
            }
        },
        None => match body.clone() {
            Some(body_value) => {
                if body_value.len() != 0 {
                    tracing::error!(
                        "Body found of length {} but no Content-Length header",
                        body_value.len()
                    );
                    return None;
                }
            }
            None => {}
        },
    }

    let req = HTTPRequest {
        verb,
        path,
        host,
        user_agent,
        content_type,
        content_length,
        accept,
        body,
    };

    Some(req)
}

async fn handle_connection(mut stream: TcpStream, serve_dir: Arc<String>) -> io::Result<()> {
    // Read the request data
    //let mut request: Vec<u8> = Vec::new();
    let mut request: [u8; 16384] = [0; 16384];
    let size = stream.read(&mut request).await?;
    let actual_size = size.min(request.len());

    // Get HTTP Request struct
    let request: HTTPRequest = match extract_parts_and_body(Vec::from(&request[..actual_size])) {
        Some(request) => request,
        None => {
            tracing::error!("error extracting parts and body from request");
            return Ok(());
        }
    };

    // Handle the request based on the path and verb
    match request.verb.as_str() {
        "GET" => match request.path.as_str() {
            "/" => {
                stream.write_all(RESPONSE_200.as_bytes()).await?;
            }
            "/user-agent" => {
                // Extract user agent
                let user_agent: String = match request.user_agent {
                    Some(user_agent) => user_agent,
                    None => "Unknown".to_string(),
                };
                // Respond 200 OK with user_agent
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                    user_agent.len(),
                    user_agent
                );
                stream.write_all(response.as_bytes()).await?;
            }
            other => {
                if other.starts_with("/echo/") {
                    let message = other.split_at(6).1;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                        message.len(),
                        message
                    );
                    stream.write_all(response.as_bytes()).await?;
                } else if other.starts_with("/files/") {
                    let file = other.split_at(7).1;
                    match fs::read(format!("{}/{}", serve_dir, file)) {
                        Ok(data) => {
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                                data.len()
                            );
                            stream.write_all(response.as_bytes()).await?;
                            stream.write_all(&data).await?;
                        }
                        Err(_) => {
                            stream.write_all(RESPONSE_404.as_bytes()).await?;
                        }
                    }
                } else {
                    // Response 404 Not Found
                    stream.write_all(RESPONSE_404.as_bytes()).await?;
                }
            }
        },
        "POST" => {
            if request.path.starts_with("/files/") {
                let filename = request.path.split_at(7).1;
                let path_value = format!("{}{}", serve_dir, filename);
                let path = std::path::Path::new(&path_value);
                if let Some(parent) = path.parent() {
                    if !parent.exists() {
                        std::fs::create_dir_all(parent)?;
                    }
                }
                match request.body {
                    Some(data) => match fs::write(path, &data) {
                        Ok(_) => {
                            stream
                                .write_all("HTTP/1.1 201 Created\r\n\r\n".as_bytes())
                                .await?;
                        }
                        Err(e) => {
                            tracing::error!("Failed to save file: {:?}", e);
                            stream.write_all(RESPONSE_404.as_bytes()).await?;
                        }
                    },
                    None => {
                        stream.write_all(RESPONSE_404.as_bytes()).await?;
                    }
                }
            } else {
                // Response 404 Not Found
                stream.write_all(RESPONSE_404.as_bytes()).await?;
            }
        }
        _ => {
            tracing::error!("Unknown request method");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse command-line arguments
    let args = Args::parse();
    let serve_dir = Arc::new(args.directory);

    // Bind to the address using Tokio's TcpListener
    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    tracing::info!("Server listening on 127.0.0.1:4221");

    // Accept connections and process them concurrently
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                tracing::info!("New connection from: {}", addr);
                let dir_clone = Arc::clone(&serve_dir);

                // Spawn a new task for each connection
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, dir_clone).await {
                        tracing::error!("Error handling connection: {}", e);
                    }
                });
            }
            Err(e) => {
                tracing::error!("Error accepting connection: {}", e);
            }
        }
    }
}
