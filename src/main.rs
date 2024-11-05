use rusty_v8 as v8;
use tokio; 
use tokio::sync::mpsc::UnboundedSender;
use tokio::io::AsyncReadExt;
use std::ffi::c_void;
use std::collections::HashMap;
use std::path::PathBuf;

//Declare internal modules 
mod console; 
mod timer;
mod fs; 
mod http;
mod request; 
mod response;
mod emitter;

mod helper; 
mod interface;
mod net; 

use crate::request::create_request_object;
use crate::request::Request;
use crate::response::create_response_object;
use crate::response::Response; 
use crate::fs::initialize_fs;
use crate::http::initialize_http;
use crate::http::incoming_message_on_callback;

use std::sync::Arc;
use std::sync::Mutex;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    //INITIALIZE V8
    let platform: v8::SharedRef<v8::Platform>  = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate = &mut v8::Isolate::new(Default::default()); 
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope); 
    let scope = &mut v8::ContextScope::new(handle_scope, context);
    let global = context.global(scope);

    //READ FILE
    let filepath: &str = "src/testing/07.js"; 
    let file_contents = match helper::read_file(filepath){
        Ok(contents) => contents, 
        Err (e) => {
            eprintln!("ERROR: {}", e);
            return; 
        }
    };
    //println!("FILE CONTENTS: \n{}", &file_contents);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<interface::Operations>();
    assign_tx_to_global(scope, &tx, "channel");

    let (tx_http, mut rx_http) = tokio::sync::mpsc::unbounded_channel::<interface::Operations>();
    assign_tx_to_global(scope, &tx_http, "http");

    //Console Operations
    let console = v8::Object::new(scope);
    let callback = console::console_log_callback; // Your existing console.log implementation
    assign_callback_to_object(scope, console, "console", "log", callback);
    
    //Timer Operations
    assign_callback_to_global(scope, "setTimeout", timer::set_timeout_callback);
    assign_callback_to_global(scope, "setInterval", timer::set_interval_callback);

    //File Operations
    initialize_fs(scope, tx);

    //Http Operations
    initialize_http(scope, tx_http);

    // Run the event loop within the LocalSet
    let local = tokio::task::LocalSet::new();

    local.run_until(async move {

        // Compile and execute the JavaScript code
        let code = v8::String::new(scope, &file_contents).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        script.run(scope);

        // Enter the event loop
        loop {
            let mut pending = false;

            tokio::select! {
                // Recieve http operations
                Some(operation) = rx_http.recv() => {
                    pending = true;
                    match operation {
                        interface::Operations::Http(http_op) => {
                            match http_op {
                                interface::HttpOperation::Listen(request, socket, callback) => {
                                    let response = Response {
                                        status_code: 200, 
                                        headers: HashMap::new(),
                                        body: String::new(),
                                    };

                                    let boxed_socket = Box::new(socket);
                                    let request_obj = create_request_object(scope, Box::new(request), None, None);
                                    let response_obj = create_response_object(scope, Box::new(response), boxed_socket);

                                    let request_value: v8::Local<v8::Value> = request_obj.into();
                                    let response_value: v8::Local<v8::Value> = response_obj.into();

                                    let args = vec![request_value, response_value];
                                    
                                    let undefined = v8::undefined(scope).into();
                                    let callback = callback.open(scope);
                                    callback.call(scope, undefined, &args).unwrap();
                                }

                                interface::HttpOperation::Get(res, callback, tx) => {
                                    //let mut incoming_message = res.lock().unwrap();
                                    let object_template = v8::ObjectTemplate::new(scope);
                                    object_template.set_internal_field_count(2);
                                    let incoming_message_obj = object_template.new_instance(scope).unwrap();

                                    let on_fn_template = v8::FunctionTemplate::new(scope, incoming_message_on_callback);
                                    let on_fn = on_fn_template.get_function(scope).unwrap();
                                    let on_fn_key = v8::String::new(scope, "on").unwrap();
                                    incoming_message_obj.set(scope, on_fn_key.into(), on_fn.into());

                                    let external_incoming_message = v8::External::new(scope, Arc::into_raw(res) as *const _ as *mut c_void);
                                    incoming_message_obj.set_internal_field(0, external_incoming_message.into());

                                    let incoming_message_value: v8::Local<v8::Value> = incoming_message_obj.into();
                                    let args = vec![incoming_message_value];
                                    
                                    let undefined = v8::undefined(scope).into();
                                    let callback = callback.open(scope);
                                    callback.call(scope, undefined, &args).unwrap();

                                    tx.send(true);
                                }

                                interface::HttpOperation::Request(mut socket, callback) => {
                                    //parse response into object from socket
                                    let response = match parse_http_response(&mut socket).await{
                                        Ok(response) => response, 
                                        Err(e) => {
                                            eprintln!("Failed to parse HTTP response: {}", e);
                                            return;
                                        }
                                    };

                                    let boxed_response = Box::new(response);
                                    let boxed_socket = Box::new(socket);

                                    let response_obj = create_response_object(scope, boxed_response, boxed_socket);
                                    let response_value: v8::Local<v8::Value> = response_obj.into();

                                    let args = vec![response_value];
                                    
                                    let undefined = v8::undefined(scope).into();
                                    let callback = callback.open(scope);
                                    callback.call(scope, undefined, &args).unwrap();
                                }
                            } 
                        }, 

                        interface::Operations::Response(response_op) => {
                            match response_op {
                                interface::ResponseEvent::Data{ res, chunk } => {
                                    let mut incoming_message = res.lock().unwrap();
                                    let chunk_str = String::from_utf8_lossy(&chunk);
                                    let chunk_value = v8::String::new(scope, &chunk_str).unwrap().into();
                                    incoming_message.event_emitter.emit(scope, "data".to_string(), &[chunk_value]);
                                },

                                interface::ResponseEvent::End{ res } => {
                                    let mut incoming_message = res.lock().unwrap();
                                    incoming_message.event_emitter.emit(scope, "end".to_string(), &[])
                                },

                                interface::ResponseEvent::Error{ res, error_message } => {
                                    let mut incoming_message = res.lock().unwrap();
                                    let error_value = v8::String::new(scope, &error_message).unwrap();
                                    incoming_message.event_emitter.emit(scope, "error".to_string(), &[error_value.into()])
                                },
                            }
                        },

                        interface::Operations::Timer(timer_op) => {
                            continue;
                        }

                        interface::Operations::Fs(fs_op) => {
                            continue;
                        }
                    }
                }

                // Receive an operation (either Timer or Fs)
                Some(operation) = rx.recv() => {
                    pending = true;
                    match operation {

                        // Handle TimerOperation (from setTimeout or another async task)
                        interface::Operations::Timer(timer_op) => {
                            match timer_op{
                                interface::TimerOperation::Timeout { callback } => {
                                    let callback = callback.open(scope);
                                    let undefined = v8::undefined(scope).into();
                                    callback.call(scope, undefined, &[]).unwrap();
                                }

                                interface::TimerOperation::Interval { callback } => {
                                    let callback = callback.open(scope);
                                    let undefined = v8::undefined(scope).into();
                                    callback.call(scope, undefined, &[]).unwrap();
                                }

                            }
                       }

                        // Handle FsOperation 
                        interface::Operations::Fs(fs_operation) => {
                            match fs_operation {
                                    // Success for ReadFile
                                    interface::FsOperation::ReadFileSuccess { callback, contents } => {
                                        let undefined = v8::undefined(scope).into();
                                        let contents = v8::String::new(scope, &contents).unwrap();
                                        let null_value = v8::null(scope).into(); 
                                        let args = &[null_value, contents.into()];
                                        let callback_fn = callback.open(scope);
                                        callback_fn.call(scope, undefined, args).unwrap();
                                    }

                                    // Error for ReadFile
                                    interface::FsOperation::ReadFileError { callback, error_message } => {
                                        let undefined = v8::undefined(scope).into();
                                        let error_message = v8::String::new(scope, &error_message.to_string()).unwrap();
                                        let args = &[error_message.into(), v8::undefined(scope).into()];
                                        let callback_fn = callback.open(scope);
                                        callback_fn.call(scope, undefined, args).unwrap();
                                    }

                                    // Success for WriteFile
                                    interface::FsOperation::WriteFileSuccess { callback } => {
                                        let undefined = v8::undefined(scope).into();
                                        let null_value = v8::null(scope).into(); 
                                        let args = &[null_value, undefined];
                                        let callback_fn = callback.open(scope);
                                        callback_fn.call(scope, undefined, args).unwrap();
                                    }

                                    // Error for WriteFile
                                    interface::FsOperation::WriteFileError { callback, error_message } => {
                                        let undefined = v8::undefined(scope).into();
                                        let error_message = v8::String::new(scope, &error_message.to_string()).unwrap();
                                        let args = &[error_message.into()];
                                        let callback_fn = callback.open(scope);
                                        callback_fn.call(scope, undefined, args).unwrap();
                                    }
                                }
                            }
                        

                        //Handle Erronous Case
                        interface::Operations::Http(http_ops) => {
                            continue;
                        }

                        //Handle Erronous Case
                        interface::Operations::Response(response_ops) => {
                            continue;
                        }
                    }

                }
            
                else => {
                    // No tasks to process, continue
                }
            }
            


            // 2. Check if there are pending tasks in Tokio
            // if !pending && rx.is_empty() && rx_http.is_empty() {
            //     // No pending tasks, exit the loop
            //     break;
            // }
    
            // Yield control to allow other Tokio tasks to run
            tokio::task::yield_now().await;
        }
    
        println!("Exiting Event Loop");
    
    }).await;

}

pub fn assign_callback_to_object(
    scope: &mut v8::ContextScope<'_, v8::HandleScope<'_>>, 
    obj: v8::Local<'_, v8::Object>, 
    object_name: &str,
    method_name: &str, 
    callback: impl v8::MapFnTo<v8::FunctionCallback>
){
    let function_template = v8::FunctionTemplate::new(scope, callback);
    let function = function_template.get_function(scope).unwrap();

    let function_key = v8::String::new(scope, method_name).unwrap();
    obj.set(scope, function_key.into(), function.into());

    let context = scope.get_current_context();
    let global = context.global(scope);
    let object_key = v8::String::new(scope, object_name).unwrap();
    global.set(scope, object_key.into(), obj.into());
}

pub fn assign_callback_to_global(
    scope: &mut v8::ContextScope<'_, v8::HandleScope<'_>>, 
    callback_name: &str, 
    callback: impl v8::MapFnTo<v8::FunctionCallback>
){
    let context = scope.get_current_context();
    let global = context.global(scope);

    let function_template = v8::FunctionTemplate::new(scope, callback);
    let function = function_template.get_function(scope).unwrap();
    let key = v8::String::new(scope, callback_name).unwrap();
    global.set(scope, key.into(), function.into());

}

pub fn assign_tx_to_global(
    scope: &mut v8::ContextScope<'_, v8::HandleScope<'_>>, 
    tx: &UnboundedSender<interface::Operations>, 
    channel_name: &str 
){
    let context = scope.get_current_context();
    let global = context.global(scope);

    let external = v8::External::new(scope, tx as *const _ as *mut c_void); //raw pointer -> c pointer

    let obj_template = v8::ObjectTemplate::new(scope);
    obj_template.set_internal_field_count(1);

    let obj = obj_template.new_instance(scope).unwrap();
    obj.set_internal_field(0, external.into());

    let key = v8::String::new(scope, channel_name).unwrap();
    global.set(scope, key.into(), obj.into());
}

pub async fn parse_http_response(
    socket: &mut tokio::net::TcpStream,
) -> Result<Response, Box<dyn std::error::Error>> {
    let mut response_data = String::new();

    // Read data from the socket into response_data
    socket.read_to_string(&mut response_data).await?;

    // Prepare to parse with httparse
    let mut headers = [httparse::EMPTY_HEADER; 32]; // Fixed-size buffer for headers
    let mut http_parse_response = httparse::Response::new(&mut headers);

    let response_data_bytes = response_data.as_bytes();
    let parsed_len = match http_parse_response.parse(response_data_bytes)? {
        httparse::Status::Complete(len) => len,
        httparse::Status::Partial => {
            return Err("Incomplete HTTP response".into());
        }
    };

    // Get the status code
    let status_code = http_parse_response.code.ok_or("Missing status code")?;

    // Convert headers to a HashMap<String, String>
    let headers_map: HashMap<String, String> = http_parse_response.headers.iter().map(|h| {
        (
            h.name.to_string(),
            String::from_utf8_lossy(h.value).to_string(),
        )
    }).collect();

    // Extract the body from the remaining bytes
    let body = String::from_utf8_lossy(&response_data_bytes[parsed_len..]).to_string();

    // Construct the Response object
    let response = Response {
        status_code,
        headers: headers_map,
        body,
    };

    Ok(response)
}


// pub fn retrieve_global_object<'s>(
//     scope: &mut v8::ContextScope<'s, v8::HandleScope<'_>>, 
//     name: &str
// ) -> Option<v8::Local<'s, v8::Object>> {
//     // Get the global object
//     let context = scope.get_current_context();
//     let global = context.global(scope);

//     // Convert the name (key) to a V8 string
//     let key = v8::String::new(scope, name).unwrap();

//     // Retrieve the object from the global scope
//     match global.get(scope, key.into()) {
//         Some(value) => {
//             if value.is_object() {
//                 Some(v8::Local::<v8::Object>::try_from(value).unwrap())
//             } else {
//                 None
//             }
//         }
//         None => None,
//     }
// }




