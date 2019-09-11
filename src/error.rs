use crate::value::Value;
use std::result::Result as StdResult;

pub type Result<'mv8, T> = StdResult<T, Error<'mv8>>;

#[derive(Debug)]
pub enum Error<'mv8> {
    /// A Rust value could not be converted to a JavaScript value.
    ToJsConversionError {
        /// Name of the Rust type that could not be converted.
        from: &'static str,
        /// Name of the JavaScript type that could not be created.
        to: &'static str,
    },
    /// A JavaScript value could not be converted to the expected Rust type.
    FromJsConversionError {
        /// Name of the JavaScript type that could not be converted.
        from: &'static str,
        /// Name of the Rust type that could not be created.
        to: &'static str,
    },
    /// An error that occurred within the JavaScript environment.
    RuntimeError(Value<'mv8>),
}

impl<'mv8> Error<'mv8> {
    pub fn from_js_conversion(from: &'static str, to: &'static str) -> Error<'mv8> {
        Error::FromJsConversionError { from, to }
    }

    pub fn to_js_conversion(from: &'static str, to: &'static str) -> Error<'mv8> {
        Error::ToJsConversionError { from, to }
    }
}
