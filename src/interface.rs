use rusty_v8 as v8; 
use tokio;

use crate::http::IncomingMessage;
use std::sync::Arc;
use tokio::sync::oneshot; 
use std::sync::Mutex;
use crate::request::Request;

pub enum Operations {
    Timer(TimerOperation),
    Fs(FsOperation),
    Http(HttpOperation),
    Response(ResponseEvent)
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
    ReadFileSuccess {
        callback: v8::Global<v8::Function>,
        contents: String,
    },
    ReadFileError {
        callback: v8::Global<v8::Function>,
        error_message: String,
    },
    WriteFileSuccess {
        callback: v8::Global<v8::Function>,
    },
    WriteFileError {
        callback: v8::Global<v8::Function>,
        error_message: String,
    },

}

pub enum HttpOperation {
    Get(Arc<Mutex<IncomingMessage>>, v8::Global<v8::Function>, oneshot::Sender<bool>),
    Request(tokio::net::TcpStream, v8::Global<v8::Function>),
    Listen(Request, tokio::net::TcpStream, v8::Global<v8::Function>)
}

pub enum ResponseEvent {
    Data {
        res: Arc<Mutex<IncomingMessage>>, 
        chunk: Vec<u8>,
    },
    End {
        res: Arc<Mutex<IncomingMessage>>, 
    },
    Error {
        res: Arc<Mutex<IncomingMessage>>, 
        error_message: String,
    },
}