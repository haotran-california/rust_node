use rusty_v8 as v8;



//Declare internal modules 
mod helper; 
mod console; 
mod os; 
mod fs; 

fn main() {
    startup_v8_platform();


    {
        let isolate: &mut v8::OwnedIsolate = &mut v8::Isolate::new(Default::default()); 

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
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();
        let result = result.to_string(scope).unwrap();

    }

    // DISPOSE V8 RESOURCES
    unsafe{
        v8::V8::dispose();
    }

    v8::V8::shutdown_platform();
}

fn startup_v8_platform(){
    //INITIALIZE V8
    let platform: v8::SharedRef<v8::Platform>  = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
}
