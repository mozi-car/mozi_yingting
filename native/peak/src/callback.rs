use napi::bindgen_prelude::Function;
use napi::threadsafe_function::{ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi::Env;
use std::sync::{Arc, Mutex};

pub struct CallbackState {
    pub callback: Arc<Mutex<Option<ThreadsafeFunction<()>>>>,
}
impl CallbackState {
    pub fn new(function: Function<'static>) -> napi::Result<Self> {
        let tsfn = function
            .build_threadsafe_function()
            .callee_handled()
            .build_callback(|_| Ok(()))?;
        Ok(Self {
            callback: Arc::new(Mutex::new(Some(tsfn))),
        })
    }
    pub fn emit(&self) {
        if let Ok(value) = self.callback.lock() {
            if let Some(tsfn) = value.as_ref() {
                let _ = tsfn.call(Ok(()), ThreadsafeFunctionCallMode::NonBlocking);
            }
        }
    }
}
