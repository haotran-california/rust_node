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
    let persistent_context = v8::Global::new(handle_scope, context);
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
    //Rc is a reference counted pointer and is single threaded
    //Arc async reference counted and used with async code
    //Both Rc and Arc are smart pointers and go out of scope after the last reference, they are read only by default 
    //RefCell is a 'container' which allows us to mutate the smart pointer
    let fs_object = Rc::new(RefCell::new(base_file_object));
    let fs_object_c_pointer = Rc::as_ptr(&fs_object) as *mut c_void;
    let external_fs = v8::External::new(scope, fs_object_c_pointer);

    // Create an ObjectTemplate with internal fields
    let object_template = v8::ObjectTemplate::new(scope);
    object_template.set_internal_field_count(1);

    // Create a new Object from the template
    let object = object_template.new_instance(scope).unwrap();
    
    // Set the internal field
    object.set_internal_field(0, external_fs.into());

    // Create persistent handle for object
    let persistent_object = v8::Global::new(scope, object);

    // Add the object to the global scope
    let key = v8::String::new(scope, "FS").unwrap();
    global.set(scope, key.into(), object.into());

    // Register the function in V8
    let fn_template = v8::FunctionTemplate::new(scope, get_fs_info_callback);
    let function = fn_template.get_function(scope).unwrap();
    let key = v8::String::new(scope, "getFsInfo").unwrap();
    global.set(scope, key.into(), function.into());

    // Execute the JavaScript code to call getFsInfo
    let js_code = v8::String::new(scope, "getFsInfo();").unwrap();
    let script = v8::Script::compile(scope, js_code, None).unwrap();
    script.run(scope).unwrap();
}

pub fn get_fs_info_callback(
    handle_scope: &mut v8::HandleScope, 
    args: v8::FunctionCallbackArguments, 
    _return_object: v8::ReturnValue 
) {
    // Get the current context from the handle scope
    let context = handle_scope.get_current_context();

    // Access the global object of the current context
    let global = context.global(handle_scope);

    // Retrieve the object with the internal field from the global object
    let key = v8::String::new(handle_scope, "FS").unwrap();
    let object = global.get(handle_scope, key.into()).unwrap();

    // Ensure the retrieved value is the expected object with an internal field
    if !object.is_object() {
        eprintln!("Error: fs_internal is not an object");
        return;
    }

    let object = v8::Local::<v8::Object>::try_from(object).unwrap();

    // Access the internal field
    let internal_field = object.get_internal_field(handle_scope, 0);

    // Handle the case where the internal field might be None
    let external = match internal_field {
        Some(field) => v8::Local::<v8::External>::try_from(field).unwrap(),
        None => {
            eprintln!("Error: No internal field set on the object");
            return;
        }
    };

    let raw_ptr = external.value() as *const RefCell<FS>;

    if raw_ptr.is_null() {
        eprintln!("Error: raw_ptr is null");
        return;
    }

    let fs_object = unsafe { &*raw_ptr };
    let fs = fs_object.borrow();

    fs.display_info();
}


