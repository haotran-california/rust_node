//V8
use rusty_v8 as v8;

//Libuv 
use libuv_sys2::{uv_loop_init, uv_run, uv_loop_close, uv_run_mode_UV_RUN_DEFAULT};

//Declare internal modules 
mod helper; 
mod console; 
mod os; 

fn main() {
    //INITIALIZE V8
    let platform: v8::SharedRef<v8::Platform>  = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate: &mut v8::OwnedIsolate = &mut v8::Isolate::new(Default::default()); 

    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context: v8::Local<v8::Context> = v8::Context::new(handle_scope);
    let global = context.global(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    //READ FILE
    let filepath: &str = "src/examples/03.txt"; 

    let file_contents = match helper::read_file(filepath){
        Ok(contents) => contents, 
        Err (e) => {
            eprintln!("ERROR: {}", e);
            return; 
        }
    };
    println!("File contents: {}", &file_contents);

    //ADD GLOBAL OBJECTS
    let function_template_console = v8::FunctionTemplate::new(scope, console::console_log_callback);
    let log_function = function_template_console.get_function(scope).unwrap();

    let function_template_os = v8::FunctionTemplate::new(scope, os::home_dir_callback);
    let home_dir_function = function_template_os.get_function(scope).unwrap();

    let console = v8::Object::new(scope);
    let key = v8::String::new(scope, "log").unwrap(); 
    console.set(scope, key.into(), log_function.into());

    let os = v8::Object::new(scope);
    let key = v8::String::new(scope, "homedir").unwrap();
    os.set(scope, key.into(), home_dir_function.into()).unwrap();

    let console_key = v8::String::new(scope, "console").unwrap();
    let os_key = v8::String::new(scope, "os").unwrap();
    global.set(scope, console_key.into(), console.into());
    global.set(scope, os_key.into(), os.into());

    //EVENT LOOP
    unsafe {
        let mut event_loop = std::mem::MaybeUninit::uninit();
        uv_loop_init(event_loop.as_mut_ptr());
    
        //RUN EVENT LOOP
        uv_run(event_loop.as_mut_ptr(), uv_run_mode_UV_RUN_DEFAULT);
    
        //EXECUTE CODE
        let code = v8::String::new(scope, &file_contents).unwrap(); 
    
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();
        let result = result.to_string(scope).unwrap();
        println!("Results: {}", result.to_rust_string_lossy(scope));
    }
}
