use rusty_v8 as v8;

//This struct has a lifetime tied to scope AND global
//If either scope or global go out of scope than this struct goes out of scope as well 
//However scope and global may have different lifetimes
pub struct NodeFS<'a, 'b, P> {
    pub name: String,
    scope: &'a mut v8::ContextScope<'b, P>,
    global: v8::Local<'b, v8::Object>,
}

impl<'a, 'b, P> NodeFS<'a, 'b, P> {
    pub fn new(scope: &'a mut v8::ContextScope<'b, P>, global: v8::Local<'b, v8::Object>) -> Self {
        Self {
            name: String::from("default_name"),
            scope,
            global,
        }
    }

    pub fn setup(&mut self, handle_scope: &mut v8::HandleScope<'b>) {
        let fs_object = v8::Object::new(handle_scope);

        let read_file_template = v8::FunctionTemplate::new(handle_scope, read_file_callback);
        let read_file_function = read_file_template.get_function(handle_scope).unwrap();

        let read_file_key = v8::String::new(handle_scope, "async_read_file").unwrap();
        fs_object.set(handle_scope, read_file_key.into(), read_file_function.into()).unwrap();

        let fs_key = v8::String::new(handle_scope, &self.name).unwrap();
        self.global.set(handle_scope, fs_key.into(), fs_object.into()).unwrap();
    }
}

//Function Definition 
//readFile(path/fd, encoding, flags, signal, (err, data))
fn read_file_callback(
    handle_scope: &mut v8::HandleScope<'_>, 
    args: v8::FunctionCallbackArguments<'_>, 
    _return_object: v8::ReturnValue<'_> 
){
    // let path = args
    //     .get(0) 
    //     .to_string(handle_scope)
    //     .unwrap()
    //     .to_rust_string_lossy(handle_scope);
    
    // let encoding = args.get(1);
    // let flags = args.get(2);
    // let signal = args.get(3);
    // let callback = args.get(4);


    // let content:Vec<u8> = fs::read(path).await; 
    // let result = match content {
    //     Ok(x) => x
    //     Err(_) => 1 
    // };
    println!("we read something");

}