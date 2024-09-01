use rusty_v8 as v8;



//Declare internal modules 
mod helper; 
mod console; 
mod os; 
mod fs; 

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
    //let filepath: &str = "src/examples/04.txt"; 
    let filepath: &str = "src/console.js";

    let file_contents = match helper::read_file(filepath){
        Ok(contents) => contents, 
        Err (e) => {
            eprintln!("ERROR: {}", e);
            return; 
        }
    };

    //ADD: log to global scope 
    let function_template_console = v8::FunctionTemplate::new(scope, console::console_log_callback);
    let log_function = function_template_console.get_function(scope).unwrap();

    let key = v8::String::new(scope, "log").unwrap();
    global.set(scope, key.into(), log_function.into());
    println!("SET: log function");



    //ADD: console to global scope
    let code = v8::String::new(scope, &file_contents).unwrap(); 

    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();

    if result.is_object(){
        println!("SET: object console");
        let key = v8::String::new(scope, "console").unwrap();
        global.set(scope, key.into(), result.into());
    }

    //TEST: console.print() 
    let filepath2: &str = "src/examples/02.txt"; 
    let file_contents2 = match helper::read_file(filepath2){
        Ok(contents) => contents, 
        Err (e) => {
            eprintln!("ERROR: {}", e);
            return; 
        }
    };

    let code2 = v8::String::new(scope, &file_contents2).unwrap();
    let script2 = v8::Script::compile(scope, code2, None).unwrap();
    let result2 = script2.run(scope).unwrap();

    let result2 = result2.to_string(scope).unwrap();

    println!("Results: {}", result2.to_rust_string_lossy(scope));
}


// fn initialize_module_loader(module_cache, scope){
//     //Attach a function template for require() into the global scope 

//     //reqiure(modulePath) -> moduleObject 

//     //

// }

// fn load_module(){

// }
