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
mod fs; 

fn main() {
    //INITIALIZE V8
    let platform: v8::SharedRef<v8::Platform>  = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate = Rc::new(RefCell::new(v8::Isolate::new(Default::default()))); 
    let mut isolate_borrow = isolate.borrow_mut();

    let handle_scope = &mut v8::HandleScope::new(&mut *isolate_borrow);
    let context: v8::Local<v8::Context> = v8::Context::new(handle_scope);
    let global = context.global(handle_scope);
    let scope = &mut v8::ContextScope::new(handle_scope, context);

    //READ FILE
    let filepath: &str = "src/index.js"; 

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

    let source_map_url = v8::Local::<v8::Value>::from(v8::undefined(scope)); 
    let resource_name = v8::String::new(scope, "index.js").unwrap();
    let origin = v8::ScriptOrigin::new(
        scope,
        resource_name.into(),
        0,     // line_offset
        0,     // column_offset
        false, // is_cross_origin
        0,     // script_id
        source_map_url, // source_map_url
        false, // is_opaque
        false, // is_wasm
        true,  // is_module
    );

    let source = v8::script_compiler::Source::new(code, Some(&origin));
    let maybe_module = script_compiler::compile_module(scope, source);
   
    let module = match maybe_module {
        Some(m) => m,
        None => {
            eprintln!("Failed to compile module");
            return;
        }
    };

    // Instantiate the module
    let isolate_handle = isolate.clone();
    // let result = module.instantiate_module(scope,
    // move |context, specifier, import_assertions, referrer|{
    //     resolve_module_callback(isolate_handle.clone(), context, specifier, import_assertions, referrer)
    // });
    let result = module.instantiate_module(scope, resolve_module_callback);

    if result.is_none() {
        eprintln!("Failed to instantiate module");
        return;
    }

    // Evaluate the module
    let result = module.evaluate(scope);
    println!("Done evaluating module");
}

// Helper function to load the module and instantiate it
fn load_and_instantiate_module<'s>(
    scope: &mut v8::CallbackScope<'s>,
    filename: &str,
) -> Option<v8::Local<'s, v8::Module>> {

    // Load the JavaScript file
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
        0,     // script_id
        source_map_url, // source_map_url
        false, // is_opaque
        false, // is_wasm
        true,  // is_module
    );

    let script_source = script_compiler::Source::new(source_code, Some(&origin));
    let maybe_module = script_compiler::compile_module(scope, script_source);

    if let Some(module) = maybe_module {
        // Instantiate the module and resolve imports using `resolve_module_callback`
        let success = module.instantiate_module(scope, |_, _, _, _,| {
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
//Check function signature
fn resolve_module_callback<'s>(
    context: v8::Local<'s, v8::Context>,
    specifier: v8::Local<'s, v8::String>,
    import_assertions: v8::Local<'s, v8::FixedArray>,
    _referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {

    unsafe{
        let scope = &mut v8::CallbackScope::new(context);
        let module_name = specifier.to_rust_string_lossy(scope);
        let filename = format!("src/{}", module_name);
        println!("{}", filename);

        let module = load_and_instantiate_module(scope, &filename).unwrap();
        return Some(module);
    }

    None
}
