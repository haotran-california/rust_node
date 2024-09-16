//setTimeout(function, miliseconds);
//setInternal(function, miliseconds);

use tokio::time::{sleep, Duration};
use rusty_v8 as v8;

pub fn set_timeout_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    tx: tokio::sync::mpsc::UnboundedSender<v8::Global<v8::Function>>,
) {
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
        sleep(Duration::from_millis(delay_ms as u64)).await;
        tx.send(persistent_callback).unwrap(); // Send the callback to the queue to be executed later
    });
}