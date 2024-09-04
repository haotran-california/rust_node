use rusty_v8 as v8;
use std::rc::Rc;
use std::ffi::c_void; 
use std::cell::RefCell;


//Declare internal modules 
mod helper; 
mod console; 
mod os; 
mod fs; 

struct FS {
    pub filename: String, 
    pub filepath: String, 
    pub mode: String
}

impl FS {
    pub fn new(arg1: &str, arg2: &str, arg3: &str) -> Self{
        Self {
            filename: arg1.to_string(),
            filepath: arg2.to_string(), 
            mode: arg2.to_string(), 
        }
    }
    
    pub fn display_info(&self) {
        println!("Filename: {}, Filepath: {}, Mode: {}", self.filename, self.filepath, self.mode);
    }
}

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
    let filepath: &str = "src/examples/04.txt"; 

    let file_contents = match helper::read_file(filepath){
        Ok(contents) => contents, 
        Err (e) => {
            eprintln!("ERROR: {}", e);
            return; 
        }
    };

    let base_file_object = FS::new("blob.txt", "./src/example/blob", "read");

    //Rc<RefCell<>>
    let fs_object = Rc::new(RefCell::new(base_file_object));
    let fs_object_c_pointer = Rc::as_ptr(&fs_object) as *mut c_void;
    let external_fs = v8::External::new(scope, fs_object_c_pointer);

    // Register the function in V8
    let fn_template = v8::FunctionTemplate::new(scope, get_fs_info_callback);
    let function = fn_template.get_function(scope).unwrap();
    fn_template.prototype_template(scope).set_internal_field_count(1);
    function.set_internal_field(0, external_fs.into());

    // Attach the function to the global object
    let global = context.global(scope);
    let key = v8::String::new(scope, "getFsInfo").unwrap();
    global.set(scope, key.into(), function.into());

    // Register the function in V8
    let js_code = v8::String::new(scope, "getFsInfo();").unwrap();
    let script = v8::Script::compile(scope, js_code, None).unwrap();
    script.run(scope).unwrap();

}

pub fn get_fs_info_callback(
    handle_scope: &mut v8::HandleScope, 
    args: v8::FunctionCallbackArguments, 
    _return_object: v8::ReturnValue 
) {
    // Get the `this` object (the function object) and access the internal field
    let this = args.this();
    let external = this.get_internal_field(handle_scope, 0).unwrap();
    let external = v8::Local::<v8::External>::try_from(external).unwrap();

    let raw_ptr = external.value() as *const RefCell<FS>;

    if raw_ptr.is_null() {
        eprintln!("Error: raw_ptr is null");
        return;
    }

    let fs_object = unsafe { &*raw_ptr };
    let fs = fs_object.borrow();

    fs.display_info();
}


