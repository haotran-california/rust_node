use tokio::time::{sleep, Duration};
use rusty_v8 as v8;
use tokio; 
use tokio::sync::mpsc::UnboundedSender;

use crate::interface::Operations;
use crate::interface::TimerOperation;
use crate::helper::retrieve_tx; 

pub struct Timer {
    tx: UnboundedSender<Operations>,
}

impl Timer {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<Operations>) -> Self {
        Timer { tx }
    }

    pub fn set_timeout(&self, callback: v8::Global<v8::Function>, delay_ms: u64) {
        let tx = self.tx.clone();
        tokio::task::spawn_local(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            if tx.send(Operations::Timer(TimerOperation::Timeout { callback })).is_err() {
                eprintln!("Failed to send timeout operation.");
            }
        });
    }

    pub fn set_interval(&self, callback: v8::Global<v8::Function>, delay_ms: u64) {
        let tx = self.tx.clone();
        tokio::task::spawn_local(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                if tx.send(Operations::Timer(TimerOperation::Interval { callback: callback.clone() })).is_err() {
                    eprintln!("Failed to send interval operation.");
                    break;
                }
            }
        });
    }
}

pub fn set_timeout_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {
    let timer = get_timer_instance(scope); 

    // Extract callback and delay
    let callback = args.get(0);
    let delay = args.get(1);

    let callback_function = v8::Local::<v8::Function>::try_from(callback).unwrap();
    let persistent_callback = v8::Global::new(scope, callback_function);

    // Parse delay
    let delay_ms = delay.number_value(scope).unwrap_or(0.0) as u64;

    // Delegate to Timer to handle async scheduling
    timer.set_timeout(persistent_callback, delay_ms);
}

pub fn set_interval_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _return_value: v8::ReturnValue,
) {
    let timer = get_timer_instance(scope); // Assuming `get_timer_instance` provides access to the `Timer`

    // Extract callback and delay
    let callback = args.get(0);
    let delay = args.get(1);

    let callback_function = v8::Local::<v8::Function>::try_from(callback).unwrap();
    let persistent_callback = v8::Global::new(scope, callback_function);

    // Parse delay
    let delay_ms = delay.number_value(scope).unwrap_or(0.0) as u64;

    // Delegate to Timer to handle async scheduling
    timer.set_interval(persistent_callback, delay_ms);
}

// Helper function to retrieve the Timer instance
fn get_timer_instance(scope: &mut v8::HandleScope) -> Timer {
    let raw_ptr = retrieve_tx(scope, "channel").unwrap();
    let tx = unsafe { &*raw_ptr };
    Timer::new(tx.clone())
}