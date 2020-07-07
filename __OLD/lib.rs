//! MiniV8 is a minimal embedded V8 JavaScript engine wrapper for Rust.

mod array;
mod conversion;
mod error;
mod ffi;
mod function;
mod mini_v8;
mod object;
mod string;
mod types;
mod value;

#[cfg(test)] mod tests;

pub use crate::array::Array;
pub use crate::error::{Error, Result};
pub use crate::function::{Function, Invocation};
pub use crate::mini_v8::MiniV8;
pub use crate::object::Object;
pub use crate::string::String;
pub use crate::value::{FromValue, FromValues, ToValue, ToValues, Value, Values, Variadic};
