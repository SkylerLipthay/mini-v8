use crate::*;
use rusty_v8 as v8;
use std::fmt;
use std::string::String as StdString;

/// Reference to an immutable JavaScript string.
///
/// Attempts to interact with an instance after its parent `MiniV8` is dropped will result in a
/// panic.
pub struct String {
    pub(crate) value: v8::Global<v8::String>,
    mv8: MiniV8,
}

impl String {
    /// Returns a Rust string converted from the V8 string.
    pub fn to_string(&self) -> StdString {
        self.mv8.scope(|scope| self.value.get(scope).to_rust_string_lossy(scope))
    }

    pub(crate) fn new(mv8: &MiniV8, value: v8::Global<v8::String>) -> String {
        String { value, mv8: mv8.weak() }
    }
}

impl Clone for String {
    fn clone(&self) -> String {
        String { value: self.value.clone(), mv8: self.mv8.weak() }
    }
}

impl fmt::Debug for String {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}
