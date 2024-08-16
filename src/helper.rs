use std::any::type_name;
use std::fs::File; 
use std::io::prelude::*;

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
