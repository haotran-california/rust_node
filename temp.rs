//MODULE LOADER
// Create a module cache to avoid loading the same module multiple times
let mut module_cache: HashMap<String, v8::Global<v8::Module>> = HashMap::new();

// Load and run the main script (index.js)
let main_module = load_and_instantiate_module(scope, "index.js", &mut module_cache);


// Helper function to load the module and instantiate it
fn load_and_instantiate_module<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope<'s>>,
    filename: &str,
    module_cache: &'s mut HashMap<String, v8::Global<v8::Module>>,
) -> Option<v8::Local<'s, v8::Module>> {

    // Check if the module is already cached
    if let Some(cached_module) = module_cache.get(filename) {
        return Some(v8::Local::new(scope, cached_module));
    }

    // Load the JavaScript file
    let code = match load_file(filename) {
        Ok(code) => code,
        Err(_) => {
            eprintln!("Failed to load file: {}", filename);
            return None;
        }
    };

    if code.is_empty() {
        return None;
    }

    // Compile the module
    let source_code = v8::String::new(scope, &code).unwrap();
    let script_source = script_compiler::Source::new(source_code, None);
    let maybe_module = script_compiler::compile_module(scope, &script_source);

    if let Some(module) = maybe_module {
        // Instantiate the module and resolve imports using `resolve_module_callback`
        let success = module.instantiate_module(scope, resolve_module_callback);
        if success {
            // Cache the module after loading it
            let global_module = v8::Global::new(scope, module);
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

    let module_name = specifier.to_rust_string_lossy(context);
    let filename = format!("{}.js", module_name);

    // Load and instantiate the module
    let isolate = context.isolate();  // Access the isolate from the context
    let module_cache: &mut HashMap<String, v8::Global<v8::Module>> = isolate.get_slot_mut().unwrap();
    load_and_instantiate_module(context, &filename, module_cache)
}

// Helper function to load JavaScript code from a file
fn load_file(filename: &str) -> std::io::Result<String> {
    fs::read_to_string(filename)
}