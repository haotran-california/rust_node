use std::env;
use rusty_v8 as v8;

pub fn home_dir_callback(
    handle_scope: &mut v8::HandleScope, 
    args: v8::FunctionCallbackArguments, 
    _return_object: v8::ReturnValue 
){

    let homedir = match env::var("HOME"){
        Ok(value) => value,
        Err(_) => String::new()
    };

    println!("{}", &homedir);
}