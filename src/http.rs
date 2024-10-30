use rusty_v8 as v8;
use std::collections::HashMap;
use url::Url;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
use tokio::sync::{Mutex, oneshot};
use crate::helper::retrieve_tx;
use crate::helper::print_type_of;
use crate::interface::Operations;
use crate::interface::HttpOperation;
use crate::request::Request; 
use crate::response::Response;
use crate::request::create_request_object; 
use std::net::TcpStream;
use std::io::{self, Read, Write};

pub fn create_server_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Get the JavaScript callback for handling requests
    let js_callback = args.get(0);
    let js_callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();

    // Create a server object (as a JS object in V8)
    let server_obj = v8::Object::new(scope);

    // Store the callback in the server object
    let callback_key = v8::String::new(scope, "requestHandler").unwrap();
    server_obj.set(scope, callback_key.into(), js_callback_function.into());

    // Attach the listen function to this object
    let listen_fn = v8::FunctionTemplate::new(scope, http_server_listen_callback);  // Assuming listen is already defined
    let listen_key = v8::String::new(scope, "listen").unwrap();
    let listen_func = listen_fn.get_function(scope).unwrap();
    server_obj.set(scope, listen_key.into(), listen_func.into());


    // Return the server object to JavaScript
    // Note V8 internall promotes the local handle by moving it onto the Javascript heap so that it remains valid 
    rv.set(server_obj.into());
}

pub fn get_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    rv: v8::ReturnValue,
) {
    // Get the JavaScript callback for handling requests
    let url = args.get(0);
    let url_v8_string = v8::Local::<v8::String>::try_from(url).unwrap();
    let url_rust_string = url_v8_string.to_rust_string_lossy(scope);

    // Get the JavaScript callback for handling requests
    let js_callback = args.get(1);
    let js_callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();

    // Use a persistent handle to store the callback function for later use
    let js_callback_global = v8::Global::new(scope, js_callback_function);

    // Going to call the GET request within the callback as the logic is more simple
    // Attempt to parse the URL
    let parsed_url = match Url::parse(&url_rust_string) {
        Ok(url) => url,
        Err(e) => {
            eprintln!("Failed to parse URL: {}", e);
            return;
        }
    };

    // Attempt to retrieve the hostname
    let hostname = match parsed_url.host_str() {
        Some(host) => host.to_string(),
        None => {
            eprintln!("Invalid hostname in URL");
            return;
        }
    };

    let hostname_clone = hostname.clone();
    let port = parsed_url.port_or_known_default().unwrap_or(80);
    let path = parsed_url.path().to_owned();

    // Retrieve channel transmitter 
    let raw_ptr = retrieve_tx(scope, "http").unwrap(); // Assuming this function returns the channel sender
    let tx = unsafe { &*raw_ptr };

    tokio::task::spawn_local(async move {
        // Connect to the server and send the HTTP GET request
        match tokio::net::TcpStream::connect((hostname_clone, port)).await {
            Ok(mut socket) => {
                // Connection successful, send the HTTP GET request
                let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, hostname);
                if let Err(e) = socket.write_all(request.as_bytes()).await {
                    eprintln!("Failed to send HTTP request: {}", e);
                    return;
                }

                // Handle the HTTP operation with the channel transmitter
                let http_operation = Operations::Http(HttpOperation::Get(socket, js_callback_global));
                tx.send(http_operation);
            }
            Err(e) => {
                // Connection failed, print an error message
                eprintln!("Failed to connect to {}:{} - {}", hostname, port, e);
            }
        }
    });
}

pub fn create_request_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Get the JavaScript callback for handling requests
    let options_raw = args.get(0);
    let options = v8::Local::<v8::Object>::try_from(options_raw).unwrap();

    // Get the JavaScript callback for handling requests
    let js_callback = args.get(1);
    let js_callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();
    let js_callback_global = v8::Global::new(scope, js_callback_function);

    // Extract required fields from options object
    let hostname_key = v8::String::new(scope, "hostname").unwrap();
    let method_key = v8::String::new(scope, "method").unwrap();
    let port_key = v8::String::new(scope, "port").unwrap();
    let path_key = v8::String::new(scope, "path").unwrap();

    let hostname = options.get(scope, hostname_key.into()).unwrap().to_rust_string_lossy(scope);
    let method = options.get(scope, method_key.into()).unwrap().to_rust_string_lossy(scope);
    let port = options.get(scope, port_key.into()).unwrap().to_rust_string_lossy(scope);
    let path = options.get(scope, path_key.into()).unwrap().to_rust_string_lossy(scope);

    let mut headers_map = HashMap::new();
    let headers_key = v8::String::new(scope, "headers").unwrap();
    let headers_obj = options.get(scope, headers_key.into()).unwrap(); 
    let headers_obj = v8::Local::<v8::Object>::try_from(headers_obj).unwrap();
    let property_names = headers_obj.get_property_names(scope).unwrap();

    // Iterate over each key and get the corresponding value
    for i in 0..property_names.length() {
        // Get the key as a string
        let key = property_names.get_index(scope, i).unwrap();
        let key_str = key.to_rust_string_lossy(scope);

        // Get the value associated with the key
        let value = headers_obj.get(scope, key).unwrap();
        let value_str = value.to_rust_string_lossy(scope);

        // Insert into the HashMap
        headers_map.insert(key_str, value_str);
    }

    let raw_ptr = retrieve_tx(scope, "http").unwrap(); // Assuming this function returns the channel sender
    let tx = unsafe { &*raw_ptr }.clone();

    let full_url = format!("http://{}:{}{}", hostname, port, path); // Construct the full URL once
    let port: u16 = port.parse().unwrap(); // Parse the port once

    let request = Box::new(Request {
        method,
        url: full_url,
        headers: headers_map,
        body: String::new(),
        tx_request: Some(tx)
    });

    // Spawn the async task to handle the connection
    let hostname_clone = hostname.clone(); // Only clone once here

    // Perform the TCP connection synchronously
    match std::net::TcpStream::connect((hostname.as_str(), port)) {
        Ok(socket) => {
            let tokio_socket = tokio::net::TcpStream::from_std(socket).unwrap();

            let boxed_socket = Box::new(tokio_socket);
            let request_obj = create_request_object(scope, request, boxed_socket, Some(js_callback_global));
            let request_value: v8::Local<v8::Value> = request_obj.into();
            rv.set(request_value.into());
        }
        Err(e) => {
            eprintln!("Failed to connect to {}:{} - {}", hostname, port, e);
        }
    }
    
}
pub fn http_server_listen_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {
    // Extract the server object 
    let server_obj = args.this();

    // Get the 'requestHandler' function from the server object
    let callback_key = v8::String::new(scope, "requestHandler").unwrap();
    let callback_value = server_obj.get(scope, callback_key.into()).unwrap();
    let js_callback = v8::Local::<v8::Function>::try_from(callback_value).unwrap();
    let js_callback_global = v8::Global::new(scope, js_callback);

    // Extract the port number from the first argument
    let mut port = 8000;
    if args.length() > 0 && args.get(0).is_number() {
        let port = args.get(0)
        .integer_value(scope)
        .unwrap() as u16;
    } 

    // Extract the host from the second arguement
    let mut host = "127.0.0.1".to_string();
    if args.length() > 1 && args.get(1).is_string() {
        host = args.get(1)
        .to_rust_string_lossy(scope)
    }

    let raw_ptr = retrieve_tx(scope, "http").unwrap(); // Assuming this function returns the channel sender
    let tx = unsafe { &*raw_ptr };

    tokio::task::spawn_local(async move {
        // Bind to the specified host and port
        let listener = match tokio::net::TcpListener::bind((host.as_str(), port)).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind to {}:{}", host, port);
                eprintln!("{}", e);
                return;
            }
        };


        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    let js_callback_global_clone = js_callback_global.clone();
                    let http_operation = Operations::Http(HttpOperation::Listen(socket, js_callback_global_clone));
                    if let Err(e) = tx.send(http_operation ) {
                        eprintln!("Failed to send socket to main event loop: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to accept connection: {}", e);
                }
            }
        }
    });
}




