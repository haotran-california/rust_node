use rusty_v8 as v8; 
use tokio;

pub enum Operations {
    Timer(TimerOperation),
    Fs(FsOperation),
    Http(HttpOperation)
}

pub enum TimerOperation {
    Timeout {
        callback: v8::Global<v8::Function>
    },
    Interval {
        callback: v8::Global<v8::Function>
    }
}

pub enum FsOperation {
    ReadFile {
        callback: v8::Global<v8::Function>,
        filename: v8::Global<v8::String>,
    },
    WriteFile {
        callback: v8::Global<v8::Function>,
        filename: v8::Global<v8::String>,
        contents: v8::Global<v8::String>,
    },
}

pub enum HttpOperation {
    Get(tokio::net::TcpStream, v8::Global<v8::Function>),
    //Request(),
    Listen(tokio::net::TcpStream, v8::Global<v8::Function>)
    // Request(tokio::net::TcpStream, v8::Global<v8::Function>)
}