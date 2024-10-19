use std::any::type_name;
use std::fs::File; 
use std::io::prelude::*;

use rusty_v8 as v8;
use tokio; 

use crate::interface::Operations;
use crate::net::Request; 
use crate::net::Response;

//Rust Notes: 
//std::io::Result<> is the same as Result<, std::io::Error>
//? either unwraps OK or SOME, or returns error to function 
pub fn read_file(filepath: &str) -> std::io::Result<String> {
    let mut file = File::open(filepath)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn print_type_of<T>(_: &T) {
    println!("Type: {}", type_name::<T>());
}

//Needs to be abstracted with an enum return type
//Perhaps need to be in a class method
pub fn retrieve_tx(
    scope: &mut v8::HandleScope, 
    channel_name: &str
) ->
    Option<*const tokio::sync::mpsc::UnboundedSender<Operations>>
{
    // Retrieve transmitter from external store
    let context = scope.get_current_context();
    let global = context.global(scope);

    let key = v8::String::new(scope, channel_name).unwrap();
    let object = global.get(scope, key.into()).unwrap();

    let object = v8::Local::<v8::Object>::try_from(object).unwrap();
    let internal_field = object.get_internal_field(scope, 0);

    let external = match internal_field {
        Some(field) => v8::Local::<v8::External>::try_from(field).unwrap(),
        None => {
            eprintln!("Error: No internal field set on the object");
            return None;
        }
    };

    let raw_ptr = external.value() as *const tokio::sync::mpsc::UnboundedSender<Operations>;
    return Some(raw_ptr);
}

// pub fn retrieve_tx_fs(
//     scope: &mut v8::HandleScope,
//     ) ->
//     Option<*const tokio::sync::mpsc::UnboundedSender<FsOperation>>
//     {
//         // Retrieve transmitter from external store
//         let context = scope.get_current_context();
//         let global = context.global(scope);
    
//         let key = v8::String::new(scope, "fs").unwrap();
//         let object = global.get(scope, key.into()).unwrap();
    
//         let object = v8::Local::<v8::Object>::try_from(object).unwrap();
//         let internal_field = object.get_internal_field(scope, 0);
    
//         let external = match internal_field {
//             Some(field) => v8::Local::<v8::External>::try_from(field).unwrap(),
//             None => {
//                 eprintln!("Error: No internal field set on the object");
//                 return None;
//             }
//         };
    
//         let raw_ptr = external.value() as *const tokio::sync::mpsc::UnboundedSender<FsOperation>;
//         return Some(raw_ptr);
// }