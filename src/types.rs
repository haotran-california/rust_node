use rusty_v8 as v8; 

pub enum Operations {
    Timer(TimerOperation),
    Fs(FsOperation),
}

pub enum TimerOperation {
    Timeout {
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