use rusty_v8 as v8;
use std::collections::HashMap;
use std::ffi::c_void;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;

extern crate httparse;

//Import V8 Callback Functions
use crate::http::request_method_callback;
use crate::http::request_url_callback;
use crate::http::request_headers_callback;
use crate::http::request_end_callback;
use crate::http::response_set_status_code_callback;
use crate::http::response_set_header_callback;
use crate::http::response_end_callback;

pub struct Request {
    pub method: String,                    // HTTP method (e.g., GET, POST)
    pub url: String,                       // The requested URL
    pub headers: HashMap<String, String>,    // List of headers as (key, value) pairs
    pub body: String,                      // Request body (we'll use String for simplicity)
}

pub struct Response {
    pub status_code: u16,                  // HTTP status code (e.g., 200 for OK)
    pub headers: HashMap<String, String>,  // List of headers as (key, value) pairs
    pub body: String,                      // Response body
}

impl Request {
    pub fn new(method: String, url: String, headers: HashMap<String, String>, body: String) -> Self {
        Request {
            method,
            url,
            headers,
            body,
        }
    }

    pub fn get_header(&self, key: &str) -> Option<&String> {
        for (header_key, header_value) in &self.headers {
            if header_key == key {
                return Some(header_value);
            }
        }
        None
    }

    pub fn get_method(&self) -> &String {
        &self.method
    }

    pub fn get_url(&self) -> &String {
        &self.url
    }

    pub async fn end(&mut self, stream_ptr: *mut TcpStream, data: Option<String>) {
        if stream_ptr.is_null(){
            println!("Error stream pointer is null");
        }
        // Convert raw pointer to mutable reference
        let stream = unsafe { &mut *stream_ptr };

        // Append any additional data to the body
        if let Some(additional_data) = data {
            println!("{}", additional_data);
            self.body.push_str(&additional_data);
        }

        // can't call the request header in the object as it complicates the initialization and passing of the socket 
        // let request_string = format!("{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", self.method, self.path, self.hostname);

        // // Send the HTTP status line
        // if let Err(e) = stream.write_all(request_string.as_bytes()).await {
        //     eprintln!("Failed to write status line: {}", e);
        //     return;
        // }

        // Send the headers
        for (key, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", key, value);
            if let Err(e) = stream.write_all(header_line.as_bytes()).await {
                eprintln!("Failed to write header: {}: {}", key, e);
                return;
            }
        }

        // End headers with an empty line
        if let Err(e) = stream.write_all(b"\r\n").await {
            eprintln!("Failed to write end of headers: {}", e);
            return;
        }

        // Flush and close the stream
        if let Err(e) = stream.flush().await {
            eprintln!("Failed to flush the stream: {}", e);
            return;
        }

        if let Err(e) = stream.shutdown().await {
            eprintln!("Failed to shutdown the stream: {}", e);
        }
    }
}

impl Response {
    pub fn new(status_code: u16, headers: HashMap<String, String>, body: String) -> Self {
        Response {
            status_code,
            headers,
            body,
        }
    }

    pub fn add_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }

    pub fn set_status_code(&mut self, code: u16) {
        self.status_code = code; 
    }

    pub async fn end(&mut self, stream_ptr: *mut TcpStream, data: Option<String>) {
        if stream_ptr.is_null(){
            println!("Stream pointer is null");
        }

        // Convert raw pointer to mutable reference
        let stream = unsafe { &mut *stream_ptr };

        // Append any additional data to the body
        if let Some(additional_data) = data {
            println!("{}", additional_data);
            self.body.push_str(&additional_data);
        }

        // Send the HTTP status line
        let status_line = format!("HTTP/1.1 {} OK\r\n", self.status_code);
        if let Err(e) = stream.write_all(status_line.as_bytes()).await {
            eprintln!("Failed to write status line: {}", e);
            return;
        }

        // Send the headers
        for (key, value) in &self.headers {
            let header_line = format!("{}: {}\r\n", key, value);
            if let Err(e) = stream.write_all(header_line.as_bytes()).await {
                eprintln!("Failed to write header: {}: {}", key, e);
                return;
            }
        }

        // End headers with an empty line
        if let Err(e) = stream.write_all(b"\r\n").await {
            eprintln!("Failed to write end of headers: {}", e);
            return;
        }

        // Send the body
        if let Err(e) = stream.write_all(self.body.as_bytes()).await {
            eprintln!("Failed to write body: {}", e);
            return;
        }

        // Flush and close the stream
        if let Err(e) = stream.flush().await {
            eprintln!("Failed to flush the stream: {}", e);
            return;
        }

        if let Err(e) = stream.shutdown().await {
            eprintln!("Failed to shutdown the stream: {}", e);
        }
    }
}

pub fn create_request_object<'s>(
    scope: &mut v8::HandleScope<'s>,
    request: Box<Request>, // Pass the Rust Request struct
    socket: Box<tokio::net::TcpStream>
) -> v8::Local<'s, v8::Object> {
    // Create the Request object template
    let request_template = v8::ObjectTemplate::new(scope);
    request_template.set_internal_field_count(2); // Store the Rust Request struct internally
    let request_obj = request_template.new_instance(scope).unwrap();

    // Add methods: .method(), .url(), .headers()
    let method_fn_template = v8::FunctionTemplate::new(scope, request_method_callback);
    let url_fn_template = v8::FunctionTemplate::new(scope, request_url_callback);
    let headers_fn_template = v8::FunctionTemplate::new(scope, request_headers_callback);
    let end_fn_template = v8::FunctionTemplate::new(scope, request_end_callback); 

    let method_fn = method_fn_template.get_function(scope).unwrap();
    let url_fn = url_fn_template.get_function(scope).unwrap();
    let header_fn = headers_fn_template.get_function(scope).unwrap();
    let end_fn = end_fn_template.get_function(scope).unwrap();

    let method_key = v8::String::new(scope, "method").unwrap();
    let url_key = v8::String::new(scope, "url").unwrap();
    let header_key = v8::String::new(scope, "headers").unwrap();
    let end_key = v8::String::new(scope, "end").unwrap();

    request_obj.set(scope, method_key.into(), method_fn.into());
    request_obj.set(scope, url_key.into(), url_fn.into());
    request_obj.set(scope, header_key.into(), header_fn.into());
    request_obj.set(scope, end_key.into(), end_fn.into());

    let external_request = v8::External::new(scope, Box::into_raw(request) as *const _ as *mut c_void);
    let external_socket = v8::External::new(scope, Box::into_raw(socket) as *const _ as *mut c_void);

    // Set the Rust Request object as an internal field of the JS object
    request_obj.set_internal_field(0, external_request.into());
    request_obj.set_internal_field(1, external_socket.into());

    request_obj
}

pub fn create_response_object<'s>(
    scope: &mut v8::HandleScope<'s>,
    response: Box<Response>, 
    socket: Box<tokio::net::TcpStream>
) -> v8::Local<'s, v8::Object> {
    // Create the Response object template
    let response_template = v8::ObjectTemplate::new(scope);
    response_template.set_internal_field_count(2); // Store the Rust Response struct and socket internally
    let response_obj = response_template.new_instance(scope).unwrap();

    let status_code_fn_template = v8::FunctionTemplate::new(scope, response_set_status_code_callback);
    let set_header_fn_template = v8::FunctionTemplate::new(scope, response_set_header_callback);
    let set_end_fn_template = v8::FunctionTemplate::new(scope, response_end_callback);

    let status_fn = status_code_fn_template.get_function(scope).unwrap();
    let set_header_fn = set_header_fn_template.get_function(scope).unwrap();
    let end_fn = set_end_fn_template.get_function(scope).unwrap();

    let status_key = v8::String::new(scope, "statusCode").unwrap();
    let set_header_key = v8::String::new(scope, "setHeader").unwrap();
    let end_key = v8::String::new(scope, "end").unwrap(); 

    response_obj.set(scope, status_key.into(), status_fn.into());
    response_obj.set(scope, set_header_key.into(), set_header_fn.into());
    response_obj.set(scope, end_key.into(), end_fn.into());

    // Create a Rust Response object and wrap it in External
    let external_response = v8::External::new(scope, Box::into_raw(response) as *const _ as *mut c_void);
    let external_socket = v8::External::new(scope, Box::into_raw(socket) as *const _ as *mut c_void);

    // Set the Rust Response object as an internal field of the JS object
    response_obj.set_internal_field(0, external_response.into());
    response_obj.set_internal_field(1, external_socket.into());

    response_obj
}

pub async fn send_response(stream: &mut TcpStream, response: &mut Response) {
    println!("Within send response");
    // Write the status line
    let status_line = format!("HTTP/1.1 {} OK\r\n", response.status_code);
    stream.write_all(status_line.as_bytes()).await.unwrap();

    // Write an empty line to separate headers and body
    stream.write_all(b"\r\n").await.unwrap();

    // Write the body in chunks (like Node.js res.write())
    stream.write_all(response.body.as_bytes()).await.unwrap();

    // Close the connection (like Node.js res.end())
    stream.flush().await.unwrap();
    stream.shutdown().await.unwrap();
}

pub fn parse_http_request(raw_data: &[u8]) -> Option<Request> {
    let mut headers = [httparse::EMPTY_HEADER; 16]; // A fixed size buffer for headers
    let mut req = httparse::Request::new(&mut headers);

    match req.parse(raw_data) {
        Ok(status) => {
            if status.is_partial() {
                println!("Partial HTTP request. Waiting for more data.");
                return None;
            }

            // Extract the method, path, and headers
            let method = req.method.unwrap_or("").to_string();
            let url = req.path.unwrap_or("").to_string();

            let headers = req.headers.iter().fold(HashMap::new(), |mut map, header| {
                map.insert(header.name.to_string(), std::str::from_utf8(header.value).unwrap_or("").to_string());
                map
            });

            // If there is a Content-Length header, use it to read the body
            let body_offset = status.unwrap();
            let body = if let Some(content_length) = headers.get("Content-Length") {
                let length: usize = content_length.parse().unwrap_or(0);
                if body_offset + length <= raw_data.len() {
                    Some(std::str::from_utf8(&raw_data[body_offset..(body_offset + length)]).unwrap_or("").to_string())
                } else {
                    None // Incomplete body
                }
            } else {
                None // No body
            };

            Some(Request{
                method,
                url,
                headers,
                body: body.unwrap_or("".to_string()),
            })
        }
        Err(e) => {
            println!("Error parsing HTTP request: {}", e);
            None
        }
    }
}

pub async fn handle_http_request(mut socket: tokio::net::TcpStream) {
    let mut buf = [0; 1024];  // A buffer for reading data in chunks
    let mut request_data = String::new();  // A string to store the full request data

    // Read data from the socket in a loop
    loop {
        let n = socket.read(&mut buf).await.unwrap();
        
        if n == 0 {
            // If no more data is read, break out of the loop
            break;
        }

        // Append the chunk of data to the request_data string
        request_data.push_str(&String::from_utf8_lossy(&buf[..n]));

        // Check if the request has been fully received (indicated by two CRLFs)
        if request_data.contains("\r\n\r\n") {
            break;
        }
    }

    // At this point, request_data contains the full request headers + body (if present)
    println!("Received request:\n{}", request_data);

    // Parse the request (you can write a parser or use a library for this)
    let request = parse_http_request(request_data.as_bytes());

    // Now we can create a Response
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "text/plain".to_string());

    let res = Response {
        status_code: 200,
        headers,
        body: "Hello from Rust HTTP server!".to_string()
    };

    // Send the response back to the client
    //send_response(socket, res).await;
}