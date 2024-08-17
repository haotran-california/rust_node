use rusty_v8 as v8;

//Declare internal modules 
mod helper; 
mod console; 
mod os; 

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
    let filepath: &str = "src/examples/03.txt"; 

    //Rust Notes: 
    //How to handle result types in main? 
    //1. match, 2. if let, 3. unwrap_or_else
    let file_contents = match helper::read_file(filepath){
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
