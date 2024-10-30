use rusty_v8 as v8;
use tokio;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use std;
use std::io;    
use std::ffi::c_void;
use std::collections::HashMap;
use url::Url;

use crate::interface::{HttpOperation, Operations};
use crate::request::create_request_object;
use crate::request::Request;
use crate::helper::retrieve_tx;

pub struct Http {
    pub tx: UnboundedSender<Operations>,
}

impl Http {
    pub fn new(tx: UnboundedSender<Operations>) -> Self {
        Self { tx }
    }

    pub fn listen(&self, host: String, port: u16, js_callback_global: v8::Global<v8::Function>) {
        let tx = self.tx.clone();
        
        tokio::task::spawn_local(async move {
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
                        let http_operation = Operations::Http(HttpOperation::Listen(socket, js_callback_global.clone()));
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

    pub fn get_request(&self, url: String, callback: v8::Global<v8::Function>) {
        let tx = self.tx.clone();

        tokio::task::spawn_local(async move {
            match Url::parse(&url) {
                Ok(parsed_url) => {
                    let hostname = match parsed_url.host_str() {
                        Some(host) => host.to_string(),
                        None => {
                            eprintln!("Invalid hostname in URL");
                            return;
                        }
                    };
                    let port = parsed_url.port_or_known_default().unwrap_or(80);
                    let path = parsed_url.path().to_owned();

                    match tokio::net::TcpStream::connect((hostname.as_str(), port)).await {
                        Ok(mut socket) => {
                            let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, hostname);
                            if let Err(e) = socket.write_all(request.as_bytes()).await {
                                eprintln!("Failed to send HTTP request: {}", e);
                                return;
                            }
                            let http_operation = Operations::Http(HttpOperation::Get(socket, callback));
                            tx.send(http_operation).unwrap();
                        }
                        Err(e) => {
                            eprintln!("Failed to connect to {}:{} - {}", hostname, port, e);
                        }
                    }
                }
                Err(e) => eprintln!("Failed to parse URL: {}", e),
            }
        });
    }

    //add support for headers later...
    pub fn request(
        &self, 
        options: HashMap<String, String>, 
        callback: v8::Global<v8::Function>
    ) -> Option<(Request, std::net::TcpStream)> {
        let tx = self.tx.clone();
        let hostname = options.get("hostname").unwrap().to_string();
        let port = options.get("port").unwrap().parse::<u16>().unwrap_or(80);
        let method = options.get("method").unwrap_or(&"GET".to_string()).to_string();
        let path = options.get("path").unwrap().to_string();
        let headers: HashMap<String, String> = HashMap::new();

        let request = Request {
            method,
            url: format!("{}{}", hostname, path),
            headers,
            body: String::new(),
            tx_request: Some(tx.clone()),
        };

        match std::net::TcpStream::connect((hostname.as_str(), port)){
            Ok(socket) => {
                Some((request, socket))
            }

            Err(e) => {
                eprintln!("Failed to connect to {}:{} - {}", hostname, port, e);
                None
            }
        }

    }
}

// V8 Callbacks
pub fn create_server_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let server_obj = v8::Object::new(scope);
    let js_callback = args.get(0);
    let js_callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();

    // Store the server callback as requestHandler within the server object
    let callback_key = v8::String::new(scope, "requestHandler").unwrap();
    server_obj.set(scope, callback_key.into(), js_callback.into());

    // Attach the listen function to this object
    let listen_fn = v8::FunctionTemplate::new(scope, http_server_listen_callback);
    let listen_key = v8::String::new(scope, "listen").unwrap();
    let listen_func = listen_fn.get_function(scope).unwrap();
    server_obj.set(scope, listen_key.into(), listen_func.into());

    rv.set(server_obj.into());
}

pub fn http_server_listen_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    rv : v8::ReturnValue,
) {
    // Retrieve pointer to Rust Http Struct
    let js_server_obj = args.this();
    let internal_field = js_server_obj.get_internal_field(scope, 0).unwrap();
    let external_http = v8::Local::<v8::External>::try_from(internal_field).unwrap();
    let http_ptr = unsafe { &*(external_http.value() as *mut Http) };

    // Get the 'requestHandler' callback function from the server object
    let callback_key = v8::String::new(scope, "requestHandler").unwrap();
    let callback_value = js_server_obj.get(scope, callback_key.into()).unwrap();
    let js_callback = v8::Local::<v8::Function>::try_from(callback_value).unwrap();
    let js_callback_global = v8::Global::new(scope, js_callback);

    // Parse arguements (with defaults) 
    let port = if args.length() > 0 && args.get(0).is_number() {
        args.get(0).integer_value(scope).unwrap() as u16
    } else {
        8000
    };

    let host = if args.length() > 1 && args.get(1).is_string() {
        args.get(1).to_rust_string_lossy(scope)
    } else {
        "127.0.0.1".to_string()
    };


    // Call the listen method on the Http instance
    http_ptr.listen(host, port, js_callback_global);
}

pub fn get_request_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    rv: v8::ReturnValue,
) {
    // Retrieve pointer to Rust Http Struct
    let js_http_obj = args.this();
    let internal_field = js_http_obj.get_internal_field(scope, 0).unwrap();
    let external_http = v8::Local::<v8::External>::try_from(internal_field).unwrap();
    let http_ptr = unsafe { &*(external_http.value() as *mut Http) };

    // Parse arguements
    let url = args.get(0).to_rust_string_lossy(scope);
    let js_callback = args.get(1);

    // Convert callback to persistent handle
    let callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();
    let persistent_callback = v8::Global::new(scope, callback_function);

    http_ptr.get_request(url, persistent_callback);
}

pub fn create_request_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Retrieve pointer to Rust Http Struct
    let js_http_obj = args.this();
    let internal_field = js_http_obj.get_internal_field(scope, 0).unwrap();
    let external_http = v8::Local::<v8::External>::try_from(internal_field).unwrap();
    let http_ptr = unsafe { &*(external_http.value() as *mut Http) };

    // Parse arguements
    let options_obj = args.get(0).to_object(scope).unwrap();
    let js_callback = args.get(1);

    // Convert callback to persistent handle
    let callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();
    let persistent_callback = v8::Global::new(scope, callback_function);

    // Parse Options object
    let mut options = HashMap::new();
    let keys = vec!["hostname", "method", "port", "path"];

    for key in &keys {
        let key_str = v8::String::new(scope, key).unwrap();
        let value = options_obj.get(scope, key_str.into()).unwrap().to_rust_string_lossy(scope);
        options.insert(key.to_string(), value);
    }

    match http_ptr.request(options, persistent_callback.clone()) {
        Some((request, socket)) => {
            // Proceed if `request` was successful
            let tokio_socket = tokio::net::TcpStream::from_std(socket).unwrap();
            let boxed_socket = Box::new(tokio_socket);
            let boxed_request = Box::new(request);
            let request_obj = create_request_object(scope, boxed_request, boxed_socket, Some(persistent_callback));
            rv.set(request_obj.into());
        }
        None => {
            
            // Optionally set the return value to undefined or a placeholder object
            println!("Failed to return request object");
            rv.set(v8::undefined(scope).into());
        }
    }

}

pub fn initialize_http(
    scope: &mut v8::ContextScope<'_, v8::HandleScope<'_>>,
    tx: UnboundedSender<Operations>
){
    let http_template = v8::ObjectTemplate::new(scope);
    http_template.set_internal_field_count(2); // Store the Rust Response struct and socket internally
    let http_obj = http_template.new_instance(scope).unwrap();

    let create_server_template = v8::FunctionTemplate::new(scope, create_server_callback);
    let get_template = v8::FunctionTemplate::new(scope, get_request_callback);
    let request_template = v8::FunctionTemplate::new(scope, create_request_callback);

    let create_server_fn = create_server_template.get_function(scope).unwrap();
    let get_fn = get_template.get_function(scope).unwrap();
    let request_fn = request_template.get_function(scope).unwrap();

    let create_server_key = v8::String::new(scope, "createServer").unwrap();
    let get_key = v8::String::new(scope, "get").unwrap();
    let request_key = v8::String::new(scope, "request").unwrap();

    http_obj.set(scope, create_server_key.into(), create_server_fn.into());
    http_obj.set(scope, get_key.into(), get_fn.into());
    http_obj.set(scope, request_key.into(), request_fn.into());

    let context = scope.get_current_context();
    let global = context.global(scope);
    let global_key = v8::String::new(scope, "http").unwrap();

    // Create a Rust File object and wrap it in External
    let http = Http::new(tx.clone());
    let boxed_http = Box::new(http);
    let external_http = v8::External::new(scope, Box::into_raw(boxed_http) as *const _ as *mut c_void);

    // Set the Rust Response object as an internal field of the JS object
    http_obj.set_internal_field(0, external_http.into());
    global.set(scope, global_key.into(), http_obj.into());
}