//setTimeout(function, miliseconds);
//setInternal(function, miliseconds);

use std::sync::Arc;
use tokio::sync::Mutex; // Use async mutex in case of async tasks
use tokio::time::{sleep, Duration};
use rusty_v8 as v8;

pub fn set_timeout_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_object: v8::ReturnValue
) {
    let raw_ptr = retrieve_timer_tx(scope).unwrap();
    let tx = unsafe{ &* raw_ptr};

    // Extract arguments and validate them (this would be the JavaScript callback and delay)
    let callback = args.get(0);
    let delay = args.get(1);

    if !callback.is_function() {
        let exception = v8::String::new(scope, "First argument must be a function").unwrap();
        scope.throw_exception(exception.into());
        return;
    }

    let callback_function = v8::Local::<v8::Function>::try_from(callback).unwrap();
    let persistent_callback = v8::Global::new(scope, callback_function);

    let delay_ms = if delay.is_number() {
        delay.number_value(scope).unwrap_or(0.0)
    } else {
        0.0
    };

    // Schedule the callback using Tokio
    tokio::task::spawn_local(async move {
        sleep(Duration::from_millis(delay_ms as u64)).await; //non-blocking
        tx.send(persistent_callback).unwrap(); // Send the callback to the queue to be executed later
    });
}



fn retrieve_timer_tx(
scope: &mut v8::HandleScope,
) ->
Option<*const tokio::sync::mpsc::UnboundedSender<v8::Global<v8::Function>>>
{
    // Retrieve transmitter from external store
    let context = scope.get_current_context();
    let global = context.global(scope);

    let key = v8::String::new(scope, "timer").unwrap();
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

    let raw_ptr = external.value() as *const tokio::sync::mpsc::UnboundedSender<v8::Global<v8::Function>>;
    return Some(raw_ptr);
}

