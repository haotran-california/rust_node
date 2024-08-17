use rusty_v8 as v8;

//How to make a callback function in V8? 
//Arguements are automatically passed in callback functions, like React
pub fn console_log_callback(
handle_scope: &mut v8::HandleScope, 
args: v8::FunctionCallbackArguments, 
_return_object: v8::ReturnValue 
){

    //convert from V8 string local handle to Rust String
    let input_str = args
        .get(0) 
        .to_string(handle_scope)
        .unwrap()
        .to_rust_string_lossy(handle_scope);

    println!("{}", input_str);
}
