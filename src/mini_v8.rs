use crate::error::{Error, Result};
use crate::ffi;
use crate::value::{self, Value};

/// The entry point into the JavaScript execution environment.
pub struct MiniV8 {
    pub(crate) context: ffi::Context,
}

impl MiniV8 {
    /// Creates a new JavaScript execution environment.
    pub fn new() -> MiniV8 {
        let context = unsafe { ffi::context_new() };
        MiniV8 { context }
    }

    /// Executes a chunk of JavaScript code and returns its result.
    pub fn eval<'mv8>(&'mv8 self, source: &str) -> Result<'mv8, Value> {
        let result = unsafe { ffi::context_eval(self.context, source.as_ptr(), source.len()) };
        let is_exception = result.exception != 0;
        let value = value::from_ffi(self, result.value);
        if !is_exception { Ok(value) } else { Err(Error::RuntimeError(value)) }
    }
}

impl Drop for MiniV8 {
    fn drop(&mut self) {
        unsafe { ffi::context_drop(self.context); }
    }
}
