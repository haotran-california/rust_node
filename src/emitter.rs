use rusty_v8 as v8;
use std::collections::HashMap;
use std::ffi::c_void;

pub struct EventEmitter {
    listeners: HashMap<String, Vec<v8::Global<v8::Function>>>,
}

impl EventEmitter {
    pub fn new() -> Self {
        EventEmitter {
            listeners: HashMap::new(),
        }
    }

    pub fn on(
        &mut self,
        event: String,
        callback: v8::Global<v8::Function>,
    ) {
        self.listeners.entry(event).or_insert_with(Vec::new).push(callback);
    }

    pub fn emit(
        &mut self,
        scope: &mut v8::HandleScope,
        event: String,
        args: &[v8::Local<v8::Value>],
    ) {

        if let Some(callbacks) = self.listeners.get_mut(&event) {
            for callback in callbacks.drain(..) {
                let local_cb = v8::Local::new(scope, callback);
                let undefined = v8::undefined(scope).into();
                local_cb.call(scope, undefined, args);
            }
        }
    }
}
