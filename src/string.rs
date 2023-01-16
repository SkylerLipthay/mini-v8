use crate::*;
use std::fmt;
use std::string::String as StdString;

#[derive(Clone)]
pub struct String {
    pub(crate) mv8: MiniV8,
    pub(crate) handle: v8::Global<v8::String>,
}

impl String {
    /// Returns a Rust string converted from the V8 string.
    pub fn to_string(&self) -> StdString {
        self.mv8.scope(|scope| {
            v8::Local::new(scope, self.handle.clone()).to_rust_string_lossy(scope)
        })
    }
}

impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}
