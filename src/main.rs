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
    helper::print_type_of(handle_scope);
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
    // let function_template_console = v8::FunctionTemplate::new(scope, console::console_log_callback);
    // let log_function = function_template_console.get_function(scope).unwrap();

    // let function_template_os = v8::FunctionTemplate::new(scope, os::home_dir_callback);
    // let home_dir_function = function_template_os.get_function(scope).unwrap();

    let console = v8::Object::new(scope);
    let callback = console::console_log_callback;
    assign_callback_to_object(scope, console, "console", "log", callback);
    // let key = v8::String::new(scope, "log").unwrap(); 
    // console.set(scope, key.into(), log_function.into());

    // let os = v8::Object::new(scope);
    // let key = v8::String::new(scope, "homedir").unwrap();
    // os.set(scope, key.into(), home_dir_function.into()).unwrap();

    // let console_key = v8::String::new(scope, "console").unwrap();
    // let os_key = v8::String::new(scope, "os").unwrap();
    // global.set(scope, console_key.into(), console.into());
    // global.set(scope, os_key.into(), os.into());

    // let fs = fs::NodeFS::new(scope, global); 
    // fs.setup(handle_scope); 

    // let fs: fs::NodeFS; 
    // fs.initialize(scope, global); 

    //EXECUTE CODE
    let code = v8::String::new(scope, &file_contents).unwrap(); 

    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.to_string(scope).unwrap();
    println!("Results: {}", result.to_rust_string_lossy(scope));
}

//Lets create a function to automate the assignment of callback functions to objects 
//I suppose a new handle scope can be created to get access to the global context 
//The contextScope will have to be passed in because it is needed to created local handles 
//But it seems like neither handle_scope or isolate (which is needed to make a handle scope) can be passed in the function due to ownership?

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
