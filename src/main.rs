use rusty_v8 as v8;
use rusty_v8::script_compiler;

use std::rc::Rc;
use std::cell::RefCell;

use std::collections::HashMap;
use std::fs::read_to_string;

//Declare internal modules 
mod helper; 
mod console; 
mod os; 

fn main() {
    //INITIALIZE V8
    v8::V8::set_flags_from_string("--harmony-import-assertions");
    let platform: v8::SharedRef<v8::Platform>  = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate: &mut v8::OwnedIsolate = &mut v8::Isolate::new(Default::default()); 
    //Isolates have some relationship with imports
    // .set_host_import_module_dynamically_callback()

    let handle_scope = &mut v8::HandleScope::new(isolate);
    let context: v8::Local<v8::Context> = v8::Context::new(handle_scope);
    let global = context.global(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    //READ FILE
    let filepath: &str = "src/index.mjs"; 

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

    //EXECUTE MODULE 
    let code = v8::String::new(scope, &file_contents).unwrap(); 

    //source map: maps back to the source code from the current transformed code
    let source_map_url = v8::Local::<v8::Value>::from(v8::undefined(scope)); 
    let resource_name = v8::String::new(scope, "index.js").unwrap();
    let origin = v8::ScriptOrigin::new(
        scope,
        resource_name.into(),
        0,     // line_offset
        0,     // column_offset
        false, // is_cross_origin
        0,     // script_id
        source_map_url, // source_map_url = undefined
        false, // is_opaque, effects whether debugging output can be seen
        false, // is_wasm
        true,  // is_module
    );

    let tc = &mut v8::TryCatch::new(scope);
    let source = v8::script_compiler::Source::new(code, Some(&origin));
    let maybe_module = script_compiler::compile_module(tc, source);
   
    let module = match maybe_module {
        Some(m) => m,
        None => {
            if tc.has_caught(){
                let exception = tc.exception().unwrap();
                let exception_str = exception.to_string(tc).unwrap();
                let msg = exception_str.to_rust_string_lossy(tc);
                println!("Compile-Time Error: {}", msg);
            }
            return;
        }
    };

    // Instantiate the module
    let result = module.instantiate_module(tc, resolve_module_callback);

    if result.is_none() {
        if tc.has_caught(){
            let exception = tc.exception().unwrap();
            let exception_str = exception.to_string(tc).unwrap();
            let msg = exception_str.to_rust_string_lossy(tc);
            println!("Runtime Error: {}", msg);
        }
        eprintln!("Failed to instantiate module. Module not returned successfully.");
        return;
    }

    // Evaluate the module
    let result = module.evaluate(tc);
    if result.is_some(){
        let r = result.unwrap();
        let result_str = r.to_string(tc).unwrap();

        println!("RESULT: {}", result_str.to_rust_string_lossy(tc))
    }
    println!("Done evaluating module");
}

// Helper function to load the module and instantiate it
fn load_and_instantiate_module<'s>(
    scope: &mut v8::CallbackScope<'s>,
    filename: &str,
) -> Option<v8::Local<'s, v8::Module>> {

    // Load the JavaScript file
    let filename = "./src/coin.mjs";
    let code = match helper::read_file(filename) {
        Ok(contents) => contents, 
        Err (e) => {
            eprintln!("ERROR: {}", e);
            "".to_string()
        } 
    }; 
        

    if code.is_empty() {
        return None;
    }

    // println!("COIN.JS");
    // println!("{}", code);

    // Compile the module
    let source_code = v8::String::new(scope, &code).unwrap();
    let source_map_url = v8::Local::<v8::Value>::from(v8::undefined(scope)); 
    let resource_name = v8::String::new(scope, "coin.js").unwrap();
    let origin = v8::ScriptOrigin::new(
        scope,
        resource_name.into(),
        0,     // line_offset
        0,     // column_offset
        false, // is_cross_origin
        1,     // script_id
        source_map_url, // source_map_url
        false, // is_opaque
        false, // is_wasm
        true,  // is_module
    );

    let script_source = script_compiler::Source::new(source_code, Some(&origin));
    let maybe_module = script_compiler::compile_module(scope, script_source);

    if let Some(module) = maybe_module {
        // Instantiate the module and resolve imports using `resolve_module_callback`
        let success = module.instantiate_module(scope, |_, _, _, _| {
            None
        });
        if success.is_some(){
            return Some(module);
        } else {
            eprintln!("Failed to instantiate module: {}", filename);
        }
    } else {
        eprintln!("Failed to compile module: {}", filename);
    }

    None
}

//This callback function does not get a handle_scope passed into it due to ABI compatability reasons
fn resolve_module_callback<'s>(
    context: v8::Local<'s, v8::Context>,
    specifier: v8::Local<'s, v8::String>,
    import_assertions: v8::Local<'s, v8::FixedArray>,
    _referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {

    unsafe{
        let scope = &mut v8::CallbackScope::new(context);
        let module_name = specifier.to_rust_string_lossy(scope);
        //The variablity in filename comes from what type of import statement is used in index.js
        let filename = format!("src/{}", module_name);
        //println!("Resolver Filename: {}", filename);

        let module = load_and_instantiate_module(scope, &filename);

        match module {
            Some(module) => {
                println!("Successfully loaded module {} with resolve function", filename);
                Some(module)
            }

            None => {
                println!("Failed to load module {} with resolve function", filename);
                None
            }
        }
    }

}
