use rusty_v8 as v8;
use tokio;
use tokio::net::TcpStream; 
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
use url::Url;

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::ffi::c_void;

use crate::interface::Operations; 
use crate::interface::HttpOperation; 

pub struct Response {
    pub status_code: u16,                  
    pub headers: HashMap<String, String>,  
    pub body: String,                      
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

// Response Methods
pub fn response_set_status_code_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
){
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_response_obj = args.this();

    // Get the internal field (the Rust Response struct)
    let internal_field = js_response_obj.get_internal_field(scope, 0).unwrap();
    let external_response = v8::Local::<v8::External>::try_from(internal_field).unwrap();

    // Cast the external pointer back to the Rust Response object
    let response = unsafe { &mut *(external_response.value() as *mut Response) };

    let status_code = args.get(0).to_rust_string_lossy(scope);

    response.set_status_code(status_code.parse::<u16>().unwrap_or(400));
    rv.set(v8::undefined(scope).into());
}

pub fn response_set_header_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
){
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_response_obj = args.this();

    // Get the internal field (the Rust Response struct)
    let internal_field = js_response_obj.get_internal_field(scope, 0).unwrap();
    let external_response = v8::Local::<v8::External>::try_from(internal_field).unwrap();

    // Cast the external pointer back to the Rust Response object
    let response = unsafe { &mut *(external_response.value() as *mut Response) };

    // Set a header in the Rust Response object
    let key = args.get(0).to_rust_string_lossy(scope);
    let value = args.get(1).to_rust_string_lossy(scope);

    response.add_header(key, value);

    rv.set(v8::undefined(scope).into());
}

pub fn response_end_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
){
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_response_obj = args.this();

    // Get the internal field (the Rust Response struct)
    let internal_field = js_response_obj.get_internal_field(scope, 0).unwrap();
    let external_response = v8::Local::<v8::External>::try_from(internal_field).unwrap();

    // Cast the external pointer back to the Rust Response object
    let response_ptr = unsafe { &mut *(external_response.value() as *mut Response) };

    // Optional: Get the final data to be appended to the body (if provided)
    let mut final_chunk = String::from("");
    if args.length() > 0 && args.get(0).is_string() {
        final_chunk = args.get(0).to_rust_string_lossy(scope);
    }

    // Get the internal field (the Tokio TcpStream Socket)
    let internal_field_socket = js_response_obj.get_internal_field(scope, 1).unwrap();
    let external_socket = v8::Local::<v8::External>::try_from(internal_field_socket).unwrap();
    let socket_ptr = unsafe { external_socket.value() as *mut tokio::net::TcpStream };

    tokio::task::spawn_local(async move {
        let socket = unsafe { &mut *socket_ptr };
        let response = unsafe { &mut *response_ptr };

        response.end(socket, Some(final_chunk)).await;
        //send_response(socket, response);
    });

    rv.set(v8::undefined(scope).into());
}

pub fn response_on_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
){
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_response_obj = args.this();

    // Get the internal field (the Rust Response struct)
    let internal_field = js_response_obj.get_internal_field(scope, 0).unwrap();
    let external_response = v8::Local::<v8::External>::try_from(internal_field).unwrap();

    // Get the internal field (the Tokio TcpStream Socket)
    let internal_field_socket = js_response_obj.get_internal_field(scope, 1).unwrap();
    let external_socket = v8::Local::<v8::External>::try_from(internal_field_socket).unwrap();
    let socket_ptr = unsafe { external_socket.value() as *mut tokio::net::TcpStream };

    // Cast the external pointer back to the Rust Response object
    let response_ptr = unsafe { &mut *(external_response.value() as *mut Response) };

    // Parse event name
    let event_name = args.get(0).to_rust_string_lossy(scope);

    // Parse callback function
    let js_callback = args.get(1);
    let js_callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();
    let js_callback_global = v8::Global::new(scope, js_callback_function);


    // tokio::task::spawn_local(async move {
    //     let socket = unsafe { &mut *socket_ptr };
    //     let response = unsafe { &mut *response_ptr };

    //     response.end(socket, Some(final_chunk)).await;
    //     //send_response(socket, response);
    // });

    rv.set(v8::undefined(scope).into());
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