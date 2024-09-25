use rusty_v8 as v8;



//Declare internal modules 
mod helper; 
mod console; 
mod os; 
mod fs; 

fn main() {
    startup_v8_platform();

    let isolate: &mut v8::OwnedIsolate = &mut v8::Isolate::new(Default::default()); 

    {

        let handle_scope = &mut v8::HandleScope::new(isolate);
        let context: v8::Local<v8::Context> = v8::Context::new(handle_scope);
        let global = context.global(handle_scope);
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

        //ADD GLOBAL OBJECTS
        let function_template_console = v8::FunctionTemplate::new(scope, console::console_log_callback);
        let log_function = function_template_console.get_function(scope).unwrap();

        let console = v8::Object::new(scope);
        let key = v8::String::new(scope, "log").unwrap(); 
        console.set(scope, key.into(), log_function.into());


        let console_key = v8::String::new(scope, "console").unwrap();
        global.set(scope, console_key.into(), console.into());

        //EXECUTE CODE
        let code = v8::String::new(scope, &file_contents).unwrap(); 
        let tc = &mut v8::TryCatch::new(scope); //Note TryCatch::new() -> Handle Scope or Context Scope
        let maybe_script = v8::Script::compile(tc, code, None);

        if let Some(script) = maybe_script {
            let result = script.run(tc);

            if result.is_none() && tc.has_caught() {
                let exception = tc.exception().unwrap();
                let exception_str = exception.to_string(tc).unwrap();
                let msg = exception_str.to_rust_string_lossy(tc);
                println!("Runtime Error: {}", msg);

            }

        } else if tc.has_caught(){
            let exception = tc.exception().unwrap();
            let exception_str = exception.to_string(tc).unwrap();
            let msg = exception_str.to_rust_string_lossy(tc);
            println!("Compile-Time Error: {}", msg);
        }
        // let result = result.to_string(scope).unwrap();


    }

    // DISPOSE V8 RESOURCES
    // unsafe{
    //     v8::V8::dispose();
    // }

    // All isolates should be out of scope before this method is called
    // Generally disposal happens automatically

    // v8::V8::shutdown_platform();
}

fn startup_v8_platform(){
    //INITIALIZE V8
    let platform: v8::SharedRef<v8::Platform>  = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
}

//Seems like try catch comes after a script is run or a module is compiled 
//Otherwise conflict with mutable borrow
//let try_catch = &mut v8::TryCatch::new(scope);

//script_compiler API has slightly move advanced compilation features

//local handles can created either within the context scope or handle scope depending whether you want to tie a context to the handle or not
//will the local handle interact with Javascript? 

//Rust -> Javascript (requires ContextScope)
//Javascript -> Rust (HandleScope is more optimal)