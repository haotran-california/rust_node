use rusty_v8 as v8;
use tokio;
use tokio::sync::oneshot;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use std;
use std::io;    
use std::ffi::c_void;
use std::collections::HashMap;
use url::Url;

use crate::interface::{ResponseEvent, HttpOperation, Operations};
use crate::request::create_request_object;
use crate::request::Request;
use crate::emitter::EventEmitter;
use crate::helper::print_type_of;
use crate::helper::retrieve_tx;

use std::sync::Arc;
use std::sync::Mutex;

pub struct IncomingMessage {
    pub event_emitter: EventEmitter
}

impl IncomingMessage {
    pub fn new() -> Self {
        Self {
            event_emitter: EventEmitter::new(),
        }
    }
}

pub struct Http {
    pub tx: UnboundedSender<Operations>,
}

impl Http {
    pub fn new(tx: UnboundedSender<Operations>) -> Self {
        Self { 
            tx,
        }
    }

    pub fn server_listen(&self, host: String, port: u16, js_callback_global: v8::Global<v8::Function>) {
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
                    Ok((mut socket, _)) => {
                        if let Some(req) = parse_http_request(&mut socket).await {
                            let http_operation = Operations::Http(HttpOperation::Listen(req, socket, js_callback_global.clone()));
                            tx.send(http_operation).unwrap();
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

                            //Register the callback
                            let incoming_message = Arc::new(Mutex::new(IncomingMessage::new()));
                            let res = incoming_message.clone();
                            
                            let (sender, receiver) = oneshot::channel::<bool>();

                            let http_operation = Operations::Http(HttpOperation::Get(res, callback, sender));
                            tx.send(http_operation).unwrap();

                            //wait for onfirmation with oneshot channel
                            //need to wait here until callback executes 
                            match receiver.await {
                                Ok(value) => {
                                    println!("We have completed the callback function");
                                }
                                Err(e) => {
                                    println!("Sender dropped");
                                    eprintln!("Failed to send HTTP request: {}", e);
                                    return;
                                }
                            }

                            //need to handle getting the headers here

                            println!("Starting to read from the socket");
                            //Read data from socket (on callback side)
                            let mut buffer = [0u8, 255];
                            loop{
                                match socket.read(&mut buffer).await {
                                    //EOF
                                    Ok(0) => {
                                        let res = incoming_message.clone();
                                        let op = Operations::Response(ResponseEvent::End{ res }); 
                                        tx.send(op);
                                    }

                                    //Data recieved
                                    Ok(n) => {
                                        let res = incoming_message.clone();
                                        let chunk = buffer[..n].to_vec();
                                        let op = Operations::Response(ResponseEvent::Data{ res, chunk }); 
                                        tx.send(op);
                                    }

                                    //Error
                                    Err(e) => {
                                        eprintln!("Error reading from socket: {}", e);
                                        let res = incoming_message.clone();
                                        let error_message = e.to_string();
                                        let op = Operations::Response(ResponseEvent::Error{ res, error_message });
                                        tx.send(op);
                                    }
                                }
                            }

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
    // Retrieve pointer to Rust Http Struct
    let js_server_obj = args.this(); // we are in server object not http
    let internal_field = js_server_obj.get_internal_field(scope, 0).unwrap();
    let external_http = v8::Local::<v8::External>::try_from(internal_field).unwrap();

    // Parse Callback
    let js_callback = args.get(0);
    let js_callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();

    let object_template = v8::ObjectTemplate::new(scope);
    object_template.set_internal_field_count(1);
    let server_obj = object_template.new_instance(scope).unwrap(); 

    // Store Http within the server object
    server_obj.set_internal_field(0, external_http.into());

    // Store callback within the server object 
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
    // let raw_ptr = retrieve_tx(scope, "channel").unwrap();
    // let tx = unsafe { &*raw_ptr };   // Retrieve pointer to Rust Http Struct

    let js_server_obj = args.this(); // we are in server object not http
    let internal_field = js_server_obj.get_internal_field(scope, 0).unwrap();
    let external_http = v8::Local::<v8::External>::try_from(internal_field).unwrap();
    let http_ptr = unsafe { &*(external_http.value() as *mut Http) };

    // Get the 'requestHandler' callback function from the server object
    let callback_key = v8::String::new(scope, "requestHandler").unwrap();
    let callback_value = js_server_obj.get(scope, callback_key.into()).unwrap();
    let js_callback = v8::Local::<v8::Function>::try_from(callback_value).unwrap();
    let js_callback_global= v8::Global::new(scope, js_callback);

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

    http_ptr.server_listen(host, port, js_callback_global);
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
            let request_obj = create_request_object(scope, boxed_request, Some(boxed_socket), Some(persistent_callback));
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
    http_template.set_internal_field_count(1); // Store the Rust Response struct and socket internally
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

pub fn incoming_message_on_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    println!("Incoming message on callback");
    // Retrieve the 'this' object
    let js_response_obj = args.this();

    // Get the internal field (the Rust Response struct)
    let internal_field = js_response_obj.get_internal_field(scope, 0).unwrap();
    let external_response = v8::Local::<v8::External>::try_from(internal_field).unwrap();
    let incoming_message_ptr = external_response.value() as *const Mutex<IncomingMessage>;
    let incoming_message: Arc<Mutex<IncomingMessage>> = unsafe { Arc::from_raw(incoming_message_ptr) };
    let incoming_message_clone = incoming_message.clone();
    // println!("Got message pointer");
    // let incoming_message_ref = unsafe { &*incoming_message_ptr };
    // println!("Got message ref");
    // let incoming_message = Arc::clone(incoming_message_ref);
    // println!("Cloned incoming message");



    // Parse arguements 
    let event = args.get(0).to_rust_string_lossy(scope);
    let callback = v8::Local::<v8::Function>::try_from(args.get(1)).unwrap();
    let global_callback = v8::Global::new(scope, callback);

    // Register the callback with the event emitter
    let mut incoming_message_gaurd = incoming_message.lock().unwrap();
    incoming_message_gaurd.event_emitter.on(event, global_callback);

    rv.set(v8::undefined(scope).into())
}

pub async fn parse_http_request(socket: &mut TcpStream) -> Option<Request> {
    let mut buffer = [0u8; 1024]; // A fixed size buffer for reading data
    let mut raw_data = Vec::new();

    // Read data from the socket into the buffer
    match socket.read(&mut buffer).await {
        Ok(n) if n > 0 => {
            raw_data.extend_from_slice(&buffer[..n]); // Collect the data into raw_data
        }
        Ok(_) => {
            println!("No data read from socket.");
            return None;
        }
        Err(e) => {
            println!("Failed to read from socket: {}", e);
            return None;
        }
    }

    let mut headers = [httparse::EMPTY_HEADER; 16]; // A fixed size buffer for headers
    let mut req = httparse::Request::new(&mut headers);

    match req.parse(&raw_data) {
        Ok(status) => {
            if status.is_partial() {
                println!("Partial HTTP request. Waiting for more data.");
                return None;
            }

            // Extract the method, path, and headers
            let method = req.method.unwrap_or("").to_string();
            let url = req.path.unwrap_or("").to_string();

            let headers = req.headers.iter().fold(HashMap::new(), |mut map, header| {
                map.insert(
                    header.name.to_string(),
                    std::str::from_utf8(header.value).unwrap_or("").to_string(),
                );
                map
            });

            // If there is a Content-Length header, use it to read the body
            let body_offset = status.unwrap();
            let body = if let Some(content_length) = headers.get("Content-Length") {
                let length: usize = content_length.parse().unwrap_or(0);
                if body_offset + length <= raw_data.len() {
                    Some(
                        std::str::from_utf8(&raw_data[body_offset..(body_offset + length)])
                            .unwrap_or("")
                            .to_string(),
                    )
                } else {
                    None // Incomplete body
                }
            } else {
                None // No body
            };

            Some(Request {
                method,
                url,
                headers,
                body: body.unwrap_or_default(),
                tx_request: None,
            })
        }
        Err(e) => {
            println!("Error parsing HTTP request: {}", e);
            None
        }
    }
}