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
    let persistent_callback = v8::Global::new(scope, js_callback_function);

    // Create a server object (as a JS object in V8)
    let obj_template = v8::ObjectTemplate::new(scope);

    // Attach the listen function to this object
    let listen_fn = v8::FunctionTemplate::new(scope, http_server_listen);  // Assuming listen is already defined
    let listen_key = v8::String::new(scope, "listen").unwrap();
    let listen_func = listen_fn.get_function(scope).unwrap();
    obj_template.set(listen_key.into(), listen_func.into());

    // Store the callback in the server object
    let server_obj = obj_template.new_instance(scope).unwrap();

    // Open the persistent callback in the current HandleScope before setting it
    let local_callback = persistent_callback.open(scope);

    let callback_key = v8::String::new(scope, "requestHandler").unwrap();
    server_obj.set(scope, callback_key.into(), local_callback.into());

    // Return the server object to JavaScript
    rv.set(server_obj.into());
}

pub fn http_server_listen(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {
    // Extract the port number from the first argument
    let port = 3000;
    if args.length() > 0 && args.get(0).is_number() {
        let port = args.get(0)
        .integer_value(scope)
        .unwrap() as u16;
    } 

    // Extract the host from the second arguement
    let host = "127.0.0.1".to_string();
    if args.length() > 1 && args.get(1).is_string() {
        host = args.get(0)
        .to_rust_string_lossy(scope)
    }

    let tx_http = retrieve_tx(scope, "http").unwrap(); // Assuming this function returns the channel sender

    tokio::task::spawn_local(async move {
        // Bind to the specified host and port
        let listener = match tokio::net::TcpListener::bind((host.as_str(), port)).await {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("Failed to bind to {}:{}", host, port);
                return;
            }
        };

        println!("Server is listening on port {}", port);


        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    let http_operation = Operations::Http(HttpOperation::Listen(socket));
                    if let Err(e) = tx_http.send(http_operation) {
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

