use rusty_v8 as v8;
use std::collections::HashMap;
use std::fs;

fn main() {
    // Initialize V8 platform and create a new isolate
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
    let mut create_params = v8::Isolate::CreateParams::default();
    let isolate = v8::Isolate::new(create_params);

    {
        let isolate_scope = v8::Isolate::Scope::new(&isolate);
        let handle_scope = &mut v8::HandleScope::new(&isolate);

        // Create a new context
        let context = v8::Context::new(handle_scope);
        let context_scope = &mut v8::ContextScope::new(handle_scope, context);

        // Create a module cache to avoid loading the same module multiple times
        let mut module_cache: HashMap<String, v8::Global<v8::Module>> = HashMap::new();

        // Load and run the main script (index.js)
        let main_module = load_and_instantiate_module(context_scope, "index.js", &mut module_cache);

        if let Some(main_module) = main_module {
            if let Some(result) = main_module.evaluate(context_scope) {
                println!("Main module executed successfully.");
            } else {
                eprintln!("Failed to evaluate main module.");
            }
        }
    }

    // Dispose of the isolate and shutdown V8
    isolate.dispose();
    v8::V8::dispose();
    v8::V8::shutdown_platform();
}

// Helper function to load the module and instantiate it
fn load_and_instantiate_module<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope<'s>>,
    filename: &str,
    module_cache: &'s mut HashMap<String, v8::Global<v8::Module>>,
) -> Option<v8::Local<'s, v8::Module>> {
    let isolate = scope.get_isolate();
    let handle_scope = &mut v8::HandleScope::new(isolate);

    // Check if the module is already cached
    if let Some(cached_module) = module_cache.get(filename) {
        return Some(v8::Local::new(handle_scope, cached_module));
    }

    // Load the JavaScript file
    let code = load_file(filename).unwrap_or_else(|_| {
        eprintln!("Failed to load file: {}", filename);
        return "".to_string();
    });
    
    if code.is_empty() {
        return None;
    }

    // Compile the module
    let source_code = v8::String::new(handle_scope, &code).unwrap();
    let script_source = v8::ScriptCompiler::Source::new(source_code, None);
    let maybe_module = v8::Module::create(handle_scope, &script_source);

    if let Some(module) = maybe_module {
        // Instantiate the module and resolve imports using `resolve_module_callback`
        let success = module.instantiate_module(scope, resolve_module_callback);
        if success {
            // Cache the module after loading it
            let global_module = v8::Global::new(handle_scope, module);
            module_cache.insert(filename.to_string(), global_module);
            return Some(module);
        } else {
            eprintln!("Failed to instantiate module: {}", filename);
        }
    } else {
        eprintln!("Failed to compile module: {}", filename);
    }

    None
}

// Callback to resolve imports (this will load the requested module)
fn resolve_module_callback<'s>(
    context: v8::Local<'s, v8::Context>,
    specifier: v8::Local<'s, v8::String>,
    _referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {
    let isolate = context.get_isolate();
    let handle_scope = &mut v8::HandleScope::new(isolate);

    let module_name = specifier.to_rust_string_lossy(handle_scope);
    let filename = format!("{}.js", module_name);

    // Load and instantiate the module
    let module_cache: &mut HashMap<String, v8::Global<v8::Module>> = isolate.get_slot_mut().unwrap();
    load_and_instantiate_module(handle_scope, &filename, module_cache)
}

// Helper function to load JavaScript code from a file
fn load_file(filename: &str) -> std::io::Result<String> {
    fs::read_to_string(filename)
}
