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

    //READ FILE
    let filepath: &str = "src/examples/02.txt"; 
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
    let external = v8::External::new(scope, tx_ref as *const _ as *mut c_void);

    let obj_template = v8::ObjectTemplate::new(scope);
    obj_template.set_internal_field_count(1);

    let obj = obj_template.new_instance(scope).unwrap();
    obj.set_internal_field(0, external.into());

    let function_name = v8::String::new(scope, "setTimeout").unwrap();
    let function_template = v8::FunctionTemplate::new(scope, timer::set_timeout_callback);
    obj_template.set(function_name.into(), function_template.into());
    
    let global = context.global(scope);
    let key = v8::String::new(scope, "timer").unwrap();
    global.set(scope, key.into(), obj.into());
    
    println!("Enter Event Loop");
    // Run the event loop within the LocalSet
    let local = tokio::task::LocalSet::new();
    local.run_until(async move {
        // Event loop to process scheduled callbacks and V8 microtasks
        loop {
            let mut pending = false;

        }
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
