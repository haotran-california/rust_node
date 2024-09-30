use rusty_v8 as v8;
use tokio; 
use std::ffi::c_void;

use std::sync::Arc;
use tokio::sync::Mutex; // Use async mutex in case of async tasks
use tokio::task;

//Declare internal modules 
mod helper; 
mod console; 
mod os; 
mod fs; 
mod timer; 

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
    let filepath: &str = "src/examples/05.txt"; 
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

    //CREATE OBJECT AND EXTERNAL
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<v8::Global<v8::Function>>();
    let tx_ref = &tx; 
    let external = v8::External::new(scope, tx_ref as *const _ as *mut c_void); //raw pointer -> c pointer

    let obj_template = v8::ObjectTemplate::new(scope);
    obj_template.set_internal_field_count(1);

    let obj = obj_template.new_instance(scope).unwrap();
    obj.set_internal_field(0, external.into());

    let key = v8::String::new(scope, "timer").unwrap();
    global.set(scope, key.into(), obj.into());

    let function_name = v8::String::new(scope, "setTimeout").unwrap();
    let function_template = v8::FunctionTemplate::new(scope, timer::set_timeout_callback);
    let set_timeout_function = function_template.get_function(scope).unwrap();
    global.set(scope, function_name.into(), set_timeout_function.into()); 

    let function_name = v8::String::new(scope, "setInterval").unwrap();
    let function_template = v8::FunctionTemplate::new(scope, timer::set_interval_callback);
    let set_interval_callback = function_template.get_function(scope).unwrap();
    global.set(scope, function_name.into(), set_interval_callback.into()); 
    
    // Run the event loop within the LocalSet
    println!("Enter Event Loop");
    let local = tokio::task::LocalSet::new();

    local.run_until(async move {
        // Compile and execute the JavaScript code
        let code = v8::String::new(scope, &file_contents).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        script.run(scope).unwrap();
    
        // Enter the event loop
        loop {
            let mut pending = false;
    
            // 1. Process Tokio tasks (e.g., `setTimeout`)
            tokio::select! {
                Some(callback) = rx.recv() => {
                    // We received a callback from `setTimeout` or another async task
                    pending = true;
    
                    // Enter a new V8 scope to run the callback
                    // let handle_scope = &mut v8::HandleScope::new(isolate);
                    // let context = isolate.get_current_context();
                    // let scope = &mut v8::ContextScope::new(handle_scope, context);
    
                    // Run the callback
                    let callback = callback.open(scope);
                    let undefined = v8::undefined(scope).into();
                    callback.call(scope, undefined, &[]).unwrap();
                }
    
                else => {
                    // No tasks to process, continue
                }
            }
    
            // 2. Check if there are pending tasks in Tokio
            if !pending && rx.is_empty() {
                // No pending tasks, exit the loop
                break;
            }
    
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
