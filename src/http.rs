use rusty_v8 as v8;
use tokio::net::TcpStream;
use std::io::{Write, Read};
use std::str;
use crate::helper::retrieve_tx;
use crate::types::Operations;
use crate::types::HttpOperation;

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

    // Attach the listen function to this object
    let listen_fn = v8::FunctionTemplate::new(scope, http_server_listen);  // Assuming listen is already defined
    let listen_key = v8::String::new(scope, "listen").unwrap();
    let listen_func = listen_fn.get_function(scope).unwrap();
    server_obj.set(scope, listen_key.into(), listen_func.into());

    // Store the callback in the server object
    // let callback_key = v8::String::new(scope, "requestHandler").unwrap();
    // server_obj.set(scope, callback_key.into(), js_callback_function.into());

    // Return the server object to JavaScript
    // Note V8 internall promotes the local handle by moving it onto the Javascript heap so that it remains valid 
    rv.set(server_obj.into());
}

pub fn http_server_listen(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {
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

        println!("Server is listening on port {}", port);


        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    let http_operation = Operations::Http(HttpOperation::Listen(socket));
                    if let Err(e) = tx.send(http_operation) {
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

