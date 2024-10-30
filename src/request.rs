use rusty_v8 as v8;
use tokio;
use tokio::net::TcpStream; 
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
use url::Url;

use std::collections::HashMap;
use std::ffi::c_void;

use crate::interface::Operations; 
use crate::interface::HttpOperation; 

pub struct Request {
    pub method: String,                    
    pub url: String,                       
    pub headers: HashMap<String, String>,    
    pub body: String,                      
    pub tx_request: Option<tokio::sync::mpsc::UnboundedSender<Operations>>
}

impl Request {
    pub fn new( method: String, 
                url: String, 
                headers: HashMap<String, String>, 
                body: String, 
                tx_request: Option<tokio::sync::mpsc::UnboundedSender<Operations>>) 
        -> Self {
            Request {
                method,
                url,
                headers,
                body,
                tx_request
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

    pub async fn end(&mut self, stream_ptr: *mut TcpStream, data: Option<String>, callback: v8::Global::<v8::Function>) {
        if stream_ptr.is_null(){
            println!("Error stream pointer is null");
        }
        // Convert raw pointer to mutable reference
        println!("Checkpoint before stream pointer is dereferenced");
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

        // Convert the mutable reference `stream` back to an owned `TcpStream`
        println!("Checkpoint: Before Req.end() successful preformed stream/pointer conversion");
        let owned_stream = unsafe { Box::from_raw(stream_ptr) };
        println!("Checkpoint: Req.end() successful preformed stream/pointer conversion");

        if let Some(tx) = &self.tx_request {
            // Send the `HttpOperation::Request` using the owned `TcpStream`
            if tx.send(Operations::Http(HttpOperation::Request(*owned_stream, callback))).is_err() {
                eprintln!("Failed to send the request operation");
            }
        } else {
            eprintln!("tx_request is None");
        }
        // if let Err(e) = stream.shutdown().await {
        //     eprintln!("Failed to shutdown the stream: {}", e);
        // }
    }
}

// Request Methods 
pub fn request_method_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
){
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_request_obj = args.this();

    // Get the internal field (the Rust Request struct)
    let internal_field = js_request_obj.get_internal_field(scope, 0).unwrap();
    let external_request = v8::Local::<v8::External>::try_from(internal_field).unwrap();

    // Cast the external pointer back to the Rust Request object
    let request = unsafe { &*(external_request.value() as *mut Request) };

    // Now you can access the Request's method
    let method = request.get_method();
    rv.set(v8::String::new(scope, method.as_str()).unwrap().into());
}

pub fn request_url_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
){
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_request_obj = args.this();

    // Get the internal field (the Rust Request struct)
    let internal_field = js_request_obj.get_internal_field(scope, 0).unwrap();
    let external_request = v8::Local::<v8::External>::try_from(internal_field).unwrap();

    // Cast the external pointer back to the Rust Request object
    let request = unsafe { &*(external_request.value() as *mut Request) };

    // Now you can access the Request's URL
    let url = request.get_url();
    rv.set(v8::String::new(scope, url.as_str()).unwrap().into());
}

pub fn request_headers_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
){
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_request_obj = args.this();

    // Get the internal field (the Rust Request struct)
    let internal_field = js_request_obj.get_internal_field(scope, 0).unwrap();
    let external_request = v8::Local::<v8::External>::try_from(internal_field).unwrap();

    // Cast the external pointer back to the Rust Request object
    let request = unsafe { &*(external_request.value() as *mut Request) };

    // Now you can access the headers
    let headers = &request.headers;

    // Create a new JavaScript object for headers
    let js_headers = v8::Object::new(scope);

    // Iterate over the headers HashMap and insert them into the JavaScript object
    for (key, value) in headers {
        let js_key = v8::String::new(scope, key.as_str()).unwrap();
        let js_value = v8::String::new(scope, value.as_str()).unwrap();
        js_headers.set(scope, js_key.into(), js_value.into());
    }

    // Return the JavaScript object with the headers
    rv.set(js_headers.into());
}

pub fn request_end_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
){
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_response_obj = args.this();

    // Get the internal field (the Rust Response struct)
    let internal_field_response = js_response_obj.get_internal_field(scope, 0).unwrap();
    let external_response = v8::Local::<v8::External>::try_from(internal_field_response).unwrap();
    let request_ptr = unsafe { &mut *(external_response.value() as *mut Request) };

    // Get the internal field (the Tokio TcpStream Socket)
    let internal_field_socket = js_response_obj.get_internal_field(scope, 1).unwrap();
    let external_socket = v8::Local::<v8::External>::try_from(internal_field_socket).unwrap();
    let socket_ptr = unsafe { external_socket.value() as *mut tokio::net::TcpStream };

    // Get the internal field (the V8 callback)
    let internal_field_callback = js_response_obj.get_internal_field(scope, 2).unwrap();
    let external_callback = v8::Local::<v8::External>::try_from(internal_field_callback).unwrap();
    let callback_ptr = unsafe { external_callback.value() as *mut v8::Global<v8::Function> };

    // Optional: Get the final data to be appended to the body (if provided)
    let mut final_chunk = String::from("");
    if args.length() > 0 && args.get(0).is_string() {
        final_chunk = args.get(0).to_rust_string_lossy(scope);
    }

    tokio::task::spawn_local(async move {
        let socket = unsafe { &mut *socket_ptr };
        let request = unsafe { &mut *request_ptr };
        //This might not work here
        let callback = unsafe { &*callback_ptr };
        request.end(socket, Some(final_chunk), callback.clone()).await;
        println!("Checkpoint: req.end() is done");

    });

    rv.set(v8::undefined(scope).into());
}

pub fn create_request_object<'s>(
    scope: &mut v8::HandleScope<'s>,
    request: Box<Request>, // Pass the Rust Request struct
    socket: Box<tokio::net::TcpStream>,
    optional_callback: Option<v8::Global<v8::Function>>
) -> v8::Local<'s, v8::Object> {
    // Create the Request object template
    let request_template = v8::ObjectTemplate::new(scope);
    request_template.set_internal_field_count(3); // Store the Rust Request struct internally
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

    if let Some(callback) = optional_callback {
        let boxed_callback = Box::new(callback);
        let external_callback = v8::External::new(scope, Box::into_raw(boxed_callback) as *const _ as *mut c_void);
        request_obj.set_internal_field(2, external_callback.into());
    }

    request_obj
}