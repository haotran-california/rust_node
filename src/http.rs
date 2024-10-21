use rusty_v8 as v8;
use crate::helper::retrieve_tx;
use crate::helper::print_type_of;
use crate::interface::Operations;
use crate::interface::HttpOperation;
use crate::net::Request; 
use crate::net::Response;
use crate::net::send_response;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncReadExt;

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

pub fn create_request_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Create a request object (as a JS object in V8)
    let request_options = v8::Object::new(scope);

    // Get the JavaScript callback for handling requests
    let js_callback = args.get(1);
    let js_callback_function = v8::Local::<v8::Function>::try_from(js_callback).unwrap();

    // Create TcpStream socket and send over to event loop for sending, implement buffered sending

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

    // Now you can access the Response's status code
    rv.set(v8::Number::new(scope, response.status_code as f64).into());
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

pub async fn hello_world(){
    println!("Hello World from error points");
}