fn create_module_loader<'s>(
    scope: &mut v8::ContextScope<'s, v8::HandleScope<'s>>,
    module_cache: &'s mut HashMap<String, v8::Global<'s, v8::Object>>,
) {
    let module_loader = v8::FunctionTemplate::new(scope, move |args: &v8::FunctionCallbackInfo| {
        let isolate = args.get_isolate();
        let handle_scope = &mut v8::HandleScope::new(isolate);

        if args.length() < 1 {
            let msg = v8::String::new(handle_scope, "Module name required").unwrap();
            isolate.throw_exception(msg.into());
            return;
        }

        let js_module_name = args.get(0).to_string(handle_scope).unwrap();
        let module_name = js_module_name.to_rust_string_lossy(handle_scope);

        // Check cache first
        if let Some(module) = module_cache.get(&module_name) {
            let local_module = v8::Local::new(handle_scope, module);
            args.set_return_value(local_module.into());
            return;
        }

        // Load and execute module
        let module_code = load_module_from_file(&module_name).unwrap_or_else(|_| {
            let msg = v8::String::new(handle_scope, "Failed to load module").unwrap();
            isolate.throw_exception(msg.into());
            return;
        });

        let code = v8::String::new(handle_scope, &module_code).unwrap();
        let script = v8::Script::compile(handle_scope, code, None).unwrap();
        let result = script.run(handle_scope).unwrap();
        let module_object = result.to_object(handle_scope).unwrap();

        // Cache the module
        module_cache.insert(module_name.clone(), v8::Global::new(handle_scope, module_object));
        args.set_return_value(module_object.into());
    });

    let loader_function = module_loader.get_function(scope).unwrap();
    let global = scope.global(scope);
    global.set(
        scope,
        v8::String::new(scope, "require").unwrap().into(),
        loader_function.into(),
    ).unwrap();
}

fn load_module_from_file(module_name: &str) -> Result<String, std::io::Error> {
    let path = format!("{}.js", module_name);
    std::fs::read_to_string(path)
}