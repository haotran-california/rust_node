use rusty_v8 as v8;
use std::fs::File; 
use std::io::prelude::*;
use std::any::type_name;
use rusty_v8::MapFnTo;

fn print_type_of<T>(_: &T) {
    println!("Type: {}", type_name::<T>());
}

//Rust Notes: 
//std::io::Result<> is the same as Result<, std::io::Error>
//? either unwraps OK or SOME, or returns error to function 
fn read_file(filepath: &str) -> std::io::Result<String> {
    let mut file = File::open(filepath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

//How to make a callback function in V8? 
//Arguements are automatically passed in callback functions, like React
fn console_log_callback(
   handle_scope: &mut v8::HandleScope, 
   args: v8::FunctionCallbackArguments, 
   mut returnObject: v8::ReturnValue 
){

    //convert from V8 string local handle to Rust String
    let inputStr = args
        .get(0) 
        .to_string(handle_scope)
        .unwrap()
        .to_rust_string_lossy(handle_scope);

    println!("Console log: {}", inputStr);
}

fn main() {
    //What is a platform? 
    //Abstraction layer which provides interface between V8 and underlying OS

    //Inheritance Chart: Platform <--- DefaultPlatform
    //The factory pattern is used to produce/initalize a class at runtime  
    let platform: v8::SharedRef<v8::Platform>  = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    //Isolates are instances of V8: separate tabs or web workers 
    let isolate: &mut v8::OwnedIsolate = &mut v8::Isolate::new(Default::default()); 

    //Handle Scopes contain references to all local handles in a particular isolate 
    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context: v8::Local<v8::Context> = v8::Context::new(handle_scope);
    let global = context.global(handle_scope);

    //RAII
    //Aquires the resources necessary to excute JavaScript code
    //Automatically dropped by the Drop Trait in Rust 
    //Scope = (Handle_Scope + Context)
    //Every time a Handle is created it must be defined in a scope 
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    //Read File
    let filepath: &str = "src/examples/02.txt"; 

    //Rust Notes: 
    //How to handle result types in main? 
    //1. match, 2. if let, 3. unwrap_or_else
    let file_contents = match read_file(filepath){
        Ok(contents) => contents, 
        Err(e) => {
            eprintln!("ERROR: {}", e);
            return; 
        }
    };
    
    println!("File contents: {}", &file_contents);


    //How to create a function? 
    //Why is a function template needed for this instead of a function?
    //Function will only create a single instance of console
    let function_template = v8::FunctionTemplate::new(scope, console_log_callback);
    let log_function = function_template.get_function(scope).unwrap();

    let console = v8::Object::new(scope);
    let key = v8::String::new(scope, "log").unwrap(); 
    console.set(scope, key.into(), log_function.into());

    let key = v8::String::new(scope, "console").unwrap();
    global.set(scope, key.into(), console.into());

    //ALTERNATIVELY
    // let object_template = v8::ObjectTemplate::new(scope);
    // let function_template = v8::FunctionTemplate::new(scope, console_log_callback);
    // let name = v8::String::new(scope, "console").unwrap();
    // object_template.set(name.into(), function_template.into()); 

    // let context = v8::Context::new_from_template(scope, object_template);
    // let scope = &mut v8::ContextScope::new(scope, context); 

    //Execute Code
    let code = v8::String::new(scope, &file_contents).unwrap(); 

    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.to_string(scope).unwrap();
    println!("Results: {}", result.to_rust_string_lossy(scope));
}
