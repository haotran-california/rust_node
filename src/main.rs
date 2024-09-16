use rusty_v8 as v8;
use tokio; 

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

    // Create isolate
    let isolate = &mut v8::Isolate::new(Default::default()); 

    // Set up communication channels for task management (e.g., for timers)
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<v8::Global<v8::Function>>();

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

    // Create LocalSet for the event loop
    let local = tokio::task::LocalSet::new();

    //Initialize code
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    // Set up `console.log`
    let console = v8::Object::new(scope);
    let callback = console::console_log_callback; // Your existing console.log implementation
    assign_callback_to_object(scope, console, "console", "log", callback);

    // Add `setTimeout` to the global object
    let tx_clone = tx.clone();
    let set_timeout_func = v8::Function::new(scope, move |scope, args, _rv| {
        timer::set_timeout_callback(scope, args, tx_clone.clone());
    })
    .unwrap();
    let set_timeout_name = v8::String::new(scope, "setTimeout").unwrap();
    let global = context.global(scope);
    global.set(scope, set_timeout_name.into(), set_timeout_func.into());

    // Compile and run the script
    let code = v8::String::new(scope, &file_contents).unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    script.run(scope).unwrap();

    // // Run the event loop within the LocalSet
    // local.run_until(async move {
    //     // Event loop to process scheduled callbacks and V8 microtasks
    //     loop {
    //         let mut pending = false;

    //         // Process any callbacks from `setTimeout` or other async tasks
    //         tokio::select! {
    //             Some(callback) = rx.recv() => {
    //                 pending = true;
    //                 let handle_scope = &mut v8::HandleScope::new(&mut isolate);
    //                 let context = isolate.get_current_context();
    //                 let scope = &mut v8::ContextScope::new(handle_scope, context);

    //                 let callback = callback.open(scope);
    //                 let undefined = v8::undefined(scope).into();
    //                 callback.call(scope, undefined, &[]).unwrap();
    //             }
    //             else => {
    //                 // No more callbacks
    //             }
    //         }

    //         // Run V8 microtasks (e.g., promises)
    //         {
    //             let handle_scope = &mut v8::HandleScope::new(&mut isolate);
    //             if isolate.has_pending_microtasks() {
    //                 pending = true;
    //                 isolate.perform_microtask_checkpoint();
    //             }
    //         }

    //         // Check if there are any pending tasks in Tokio
    //         if !pending && rx.is_empty() {
    //             // No pending tasks in V8 or Tokio, exit the loop
    //             break;
    //         }

    //         // Yield control to allow Tokio to process other tasks
    //         tokio::task::yield_now().await;
    //     }
    // }).await;

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
