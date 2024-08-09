use rusty_v8 as v8;
use std::fs::File; 
use std::io::prelude::*;
use std::any::type_name;

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

    //RAII
    //Aquires the resources necessary to excute JavaScript code
    //Automatically dropped by the Drop Trait in Rust 
    //Scope = (Handle_Scope + Context)
    //Every time a Handle is created it must be defined in a scope 
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    //Read File
    let filepath: &str = "src/examples/01.txt"; 

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

    let ptr_file_content = &file_contents;   
    
    println!("{}", ptr_file_content);

    let code = v8::String::new(scope, ptr_file_content).unwrap(); 

    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.to_string(scope).unwrap();
    println!("result: {}", result.to_rust_string_lossy(scope));

    //This doesn't work because or_else() demands a default value

    // let content: String = read_file(filepath).unwrap_or_else(|e| {
    //     eprintln!("ERROR: file could not be read {}", e);
    //     return 1; 
    // });

    //v8::V8::dispose();

}
