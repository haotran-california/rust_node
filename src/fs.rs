use rusty_v8 as v8;
use tokio;
use tokio::sync::mpsc::UnboundedSender;

use std::path::Path;
use std::path::PathBuf;
use std::ffi::c_void;

use crate::interface::Operations;
use crate::interface::FsOperation;
use crate::helper::retrieve_tx; 

pub struct File {
    path: PathBuf,
    tx: UnboundedSender<Operations>
}

impl File {
    pub fn new(path: PathBuf, tx: UnboundedSender<Operations>) -> Self {
        Self {
            path,
            tx,
        }
    }

    pub fn set_path(&mut self, path: &str) {
        self.path = PathBuf::from(path);
    }

    pub fn path_exists(&self) -> bool {
        self.path.exists()
    }

    // Reads the file asynchronously and triggers the callback if set.
    pub fn read(&self, callback: v8::Global<v8::Function>) {
        let path_clone = self.path.clone();
        let tx_clone = self.tx.clone();

        if self.path_exists() == false {
            println!("Error: The path {} does not exist", path_clone.display());
            return;
        }

        tokio::task::spawn_local(async move {
            match tokio::fs::read_to_string(&path_clone).await {
                Ok(contents) => {
                    // let contents = v8::String::new(scope, std::str::from_utf8(&contents).unwrap()).unwrap();
                    // let null_value = v8::null(scope).into(); 
                    // let args = &[null_value, contents_str.into()];
                    let op = Operations::Fs(FsOperation::ReadFileSuccess{ callback, contents }); 
                    tx_clone.send(op).unwrap();
                },
                
                Err(error_message) => {
                    // let error_message = v8::String::new(scope, &e.to_string()).unwrap();
                    // let args = &[error_message.into(), v8::undefined(scope).into()];
                    let error_message = error_message.to_string();
                    let op = Operations::Fs(FsOperation::ReadFileError{ callback, error_message }); 
                    tx_clone.send(op).unwrap();
                } 
            }
        });
    }

    pub fn write(&self, data: String, callback: v8::Global<v8::Function>) {
        let path_clone = self.path.clone();
        let tx_clone = self.tx.clone();

        if !self.path_exists(){
            println!("Error: The path {} does not exist", path_clone.display());
            return;
        }

        tokio::task::spawn_local(async move {
            match tokio::fs::write(&path_clone, data).await {
                Ok(_) => {
                    // let null_value = v8::null(scope).into();
                    // let args = &[null_value, undefined_value]
                    let op = Operations::Fs(FsOperation::WriteFileSuccess{ callback }); 
                    tx_clone.send(op).unwrap();
                }, 

                Err(error_message) => {
                    // let error_message = v8::String::new(scope, &e.to_string()).unwrap();
                    // let args = &[error_message.into(), v8::undefined(scope).into()];
                    let error_message = error_message.to_string();
                    let op = Operations::Fs(FsOperation::WriteFileError{ callback, error_message }); 
                    tx_clone.send(op).unwrap();
                } 
            }
        });
    }


}

pub fn fs_read_file_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_fs_obj = args.this();

    // Get the internal field (the Rust File struct)
    let internal_field = js_fs_obj.get_internal_field(scope, 0).unwrap();
    let external_fs = v8::Local::<v8::External>::try_from(internal_field).unwrap();
    let file_ptr = unsafe { &mut *(external_fs.value() as *mut File) };
    
    // Extract the file path from the arguments
    let path = args.get(0).to_rust_string_lossy(scope);
    let callback = args.get(1);

    let callback_function = v8::Local::<v8::Function>::try_from(callback).unwrap();
    let persistent_callback = v8::Global::new(scope, callback_function);

    file_ptr.set_path(&path);
    file_ptr.read(persistent_callback)

}

pub fn fs_write_file_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {
    // Retrieve the JS object (the "this" object in JavaScript)
    let js_fs_obj = args.this();

    // Get the internal field (the Rust Request struct)
    let internal_field = js_fs_obj.get_internal_field(scope, 0).unwrap();
    let external_fs = v8::Local::<v8::External>::try_from(internal_field).unwrap();
    let file_ptr = unsafe { &mut *(external_fs.value() as *mut File) };

    // Extract the file path from the arguments
    let path = args.get(0).to_rust_string_lossy(scope);
    let contents = args.get(1).to_rust_string_lossy(scope);
    let callback = args.get(2);

    let callback_function = v8::Local::<v8::Function>::try_from(callback).unwrap();
    let persistent_callback = v8::Global::new(scope, callback_function);

    file_ptr.set_path(&path);
    file_ptr.write(contents, persistent_callback);
}

pub fn initialize_fs(
    scope: &mut v8::ContextScope<'_, v8::HandleScope<'_>>,
    tx: UnboundedSender<Operations>
){
    let fs_template = v8::ObjectTemplate::new(scope);
    fs_template.set_internal_field_count(2); // Store the Rust Response struct and socket internally
    let fs_obj = fs_template.new_instance(scope).unwrap();

    let read_file_fn_template = v8::FunctionTemplate::new(scope, fs_read_file_callback);
    let write_file_fn_template = v8::FunctionTemplate::new(scope, fs_write_file_callback);

    let read_file_fn = read_file_fn_template.get_function(scope).unwrap();
    let write_file_fn = write_file_fn_template.get_function(scope).unwrap();

    let read_file_key = v8::String::new(scope, "readFile").unwrap();
    let write_file_key = v8::String::new(scope, "writeFile").unwrap();

    fs_obj.set(scope, read_file_key.into(), read_file_fn.into());
    fs_obj.set(scope, write_file_key.into(), write_file_fn.into());

    let empty_path = PathBuf::new();
    let file = File::new(empty_path, tx.clone());

    let context = scope.get_current_context();
    let global = context.global(scope);
    let global_key = v8::String::new(scope, "fs").unwrap();

    // Create a Rust File object and wrap it in External
    let boxed_file = Box::new(file);
    let external_fs = v8::External::new(scope, Box::into_raw(boxed_file) as *const _ as *mut c_void);

    // Set the Rust Response object as an internal field of the JS object
    fs_obj.set_internal_field(0, external_fs.into());
    global.set(scope, global_key.into(), fs_obj.into());
}