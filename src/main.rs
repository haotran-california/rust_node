use rusty_v8 as v8;
use tokio; 
use tokio::sync::mpsc::UnboundedSender;
use std::ffi::c_void;


//Declare internal modules 
mod helper; 
mod console; 
mod fs; 
mod timer; 
mod interface;
mod http;
mod net; 



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
    let filepath: &str = "src/examples/06.txt"; 
    let file_contents = match helper::read_file(filepath){
        Ok(contents) => contents, 
        Err (e) => {
            eprintln!("ERROR: {}", e);
            return; 
        }
    };
    //println!("FILE CONTENTS: \n{}", &file_contents);

    let console = v8::Object::new(scope);
    let callback = console::console_log_callback; // Your existing console.log implementation
    assign_callback_to_object(scope, console, "console", "log", callback);

    //REFACTOR
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<interface::Operations>();
    assign_tx_to_global(scope, &tx, "channel");

    let (tx_http, mut rx_http) = tokio::sync::mpsc::unbounded_channel::<interface::Operations>();
    assign_tx_to_global(scope, &tx_http, "http");

    assign_callback_to_global(scope, "createServer", http::create_server_callback);

    //Timer Operations
    assign_callback_to_global(scope, "setTimeout", timer::set_timeout_callback);
    assign_callback_to_global(scope, "setInternval", timer::set_interval_callback);

    //File Operations
    assign_callback_to_global(scope, "readFile", fs::fs_read_file_callback);
    assign_callback_to_global(scope, "writeFile", fs::fs_write_file_callback);

    // Run the event loop within the LocalSet
    let local = tokio::task::LocalSet::new();

    local.run_until(async move {

        // //LOAD HTTP MODULE
        // let module_filepath: &str = "src/http.js"; 
        // let module_file_contents = match helper::read_file(module_filepath){
        //     Ok(contents) => contents, 
        //     Err (e) => {
        //         eprintln!("ERROR: {}", e);
        //         return; 
        //     }
        // };

        // let code = v8::String::new(scope, &module_file_contents).unwrap();
        // let script = v8::Script::compile(scope, code, None).unwrap();
        // let result = script.run(scope).unwrap();

        // let module_name = v8::String::new(scope, "http").unwrap();
        // global.set(scope, module_name.into(), result.into());

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
                                interface::HttpOperation::Listen(socket) => {
                                    net::handle_http_request(socket);
                                }
                            } 
                        }, 

                        _ => {
                            println!("Unhandled operation");
                        }
                    }
                }

                // Receive an operation (either Timer or Fs)
                Some(operation) = rx.recv() => {
                    pending = true;
                    match operation {
                        // Handle TimerOperation (from setTimeout or another async task)
                        interface::Operations::Timer(timer_callback) => {
                            match timer_callback{
                                interface::TimerOperation::Timeout { callback } => {
                                    let callback = callback.open(scope);
                                    let undefined = v8::undefined(scope).into();
                                    callback.call(scope, undefined, &[]).unwrap();
                                }
                            }
                       }
            
                        // Handle FsOperation (ReadFile or WriteFile)
                        interface::Operations::Fs(fs_operation) => {
                            match fs_operation {
                                interface::FsOperation::ReadFile { callback, filename } => {
                                    let path_str = filename.open(scope).to_rust_string_lossy(scope);
                                    let path = std::path::Path::new(&path_str);
            
                                    match tokio::fs::read(path).await {
                                        Ok(contents) => {
                                            // Convert the file contents to a V8 string
                                            let contents_str = v8::String::new(scope, std::str::from_utf8(&contents).unwrap()).unwrap();
                                            // Call the callback with the file contents
                                            let null_value = v8::null(scope).into(); // No error
                                            let args = &[null_value, contents_str.into()];
                                            let callback = callback.open(scope);
                                            let undefined = v8::undefined(scope).into();
                                            callback.call(scope, undefined, args).unwrap();
                                        }
                                        Err(e) => {
                                            // If there was an error reading the file, pass the error message to the callback
                                            let error_message = v8::String::new(scope, &e.to_string()).unwrap();
                                            let args = &[error_message.into(), v8::undefined(scope).into()];
                                            let callback = callback.open(scope);
                                            let undefined = v8::undefined(scope).into();
                                            callback.call(scope, undefined, args).unwrap();
                                        }
                                    }
                                }
            
                                interface::FsOperation::WriteFile { callback, filename, contents } => {
                                    let path_str = filename.open(scope).to_rust_string_lossy(scope);
                                    let contents_str = contents.open(scope).to_rust_string_lossy(scope);
                                    let path = std::path::Path::new(&path_str);
                                    let undefined_value = v8::undefined(scope).into();
            
                                    match tokio::fs::write(path, contents_str).await {
                                        Ok(_) => {
                                            // Success: Call the callback with null (no error) and undefined
                                            let null_value = v8::null(scope).into();
                                            let args = &[null_value, undefined_value];
                                            let callback = callback.open(scope);
                                            callback.call(scope, undefined_value, args).unwrap();
                                        }
                                        Err(e) => {
                                            // Error: Call the callback with the error message
                                            let error_message = v8::String::new(scope, &e.to_string()).unwrap();
                                            let args = &[error_message.into(), v8::undefined(scope).into()];
                                            let callback = callback.open(scope);
                                            callback.call(scope, undefined_value, args).unwrap();
                                        }
                                    }
                                }


                            }
                        }

                        //Handle Erronous Case
                        interface::Operations::Http(http_ops) => {
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



