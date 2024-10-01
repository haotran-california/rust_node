use rusty_v8 as v8;
use tokio::fs::read;
use std::path::Path;
use crate::helper::retrieve_tx_fs; 
use crate::helper::print_type_of;


pub fn fs_read_file_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {

    let raw_ptr = retrieve_tx_fs(scope).unwrap(); // Retrieve your channel sender for async task communication
    let tx = unsafe { &*raw_ptr };
    
    print_type_of(tx);
    // Extract the file path from the arguments
    let file_path = args.get(0);
    let callback = args.get(1);

    // Convert file path to V8 string and persist it for future use
    let file_path_v8_str = v8::Local::<v8::String>::try_from(file_path).unwrap();
    let persistent_file_path = v8::Global::new(scope, file_path_v8_str);

    // Check if the file path exists (synchronously in Rust)
    let file_path_str = file_path_v8_str.to_rust_string_lossy(scope);
    let path = Path::new(&file_path_str);

    // Check if the file path exists
    if !path.exists() {
        let exception = v8::String::new(scope, "File does not exist").unwrap();
        scope.throw_exception(exception.into());
        return; 
    }

    let callback_function = v8::Local::<v8::Function>::try_from(callback).unwrap();
    let persistent_callback = v8::Global::new(scope, callback_function);

    tokio::task::spawn_local(async move {
        tx.send((persistent_callback, persistent_file_path)).unwrap();     
    });

}