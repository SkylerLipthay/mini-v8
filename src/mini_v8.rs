use crate::array::Array;
use crate::error::Result;
use crate::ffi;
use crate::object::Object;
use crate::string::String;
use crate::types::Ref;
use crate::value::{self, Value, ToValue};

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
        value::from_ffi_result(self, result)
    }

    /// Creates and returns a string managed by V8.
    pub fn create_string(&self, value: &str) -> String {
        String(Ref::from_persistent(self, unsafe {
            ffi::string_create(self.context, value.as_ptr(), value.len())
        }))
    }

    /// Creates and returns an empty `Object` managed by V8.
    pub fn create_object(&self) -> Object {
        Object(Ref::from_persistent(self, unsafe { ffi::object_create(self.context) }))
    }

    /// Creates and returns an empty `Array` managed by V8.
    pub fn create_array(&self) -> Array {
        Array(Ref::from_persistent(self, unsafe { ffi::array_create(self.context) }))
    }

    /// Creates and returns an `Object` managed by V8 filled with the keys and values from an
    /// iterator. Keys are coerced to object properties.
    ///
    /// This is a thin wrapper around `MiniV8::create_object` and `Object::set`. See `Object::set`
    /// for how this method might return an error.
    pub fn create_object_from<'mv8, K, V, I>(&'mv8 self, iter: I) -> Result<'mv8, Object<'mv8>>
    where
        K: ToValue<'mv8>,
        V: ToValue<'mv8>,
        I: IntoIterator<Item = (K, V)>,
    {
        let object = self.create_object();
        for (k, v) in iter {
            object.set(k, v)?;
        }
        Ok(object)
    }

    /// Coerces a value to a string. Nearly all JavaScript values are coercible to strings, but this
    /// may fail with a runtime error if `toString()` fails or under otherwise extraordinary
    /// circumstances (e.g. if the ECMAScript `ToString` implementation throws an error).
    pub fn coerce_string<'mv8>(&'mv8 self, value: &Value<'mv8>) -> Result<'mv8, String<'mv8>> {
        match value {
            Value::String(ref s) => Ok(s.clone()),
            ref value => {
                let ffi_result = unsafe {
                    ffi::coerce_string(self.context, value::to_ffi(self, value))
                };
                match value::from_ffi_result(self, ffi_result) {
                    Ok(Value::String(s)) => Ok(s),
                    Err(err) => Err(err),
                    _ => unreachable!(),
                }
            },
        }
    }

    /// Coerces a value to a number. Nearly all JavaScript values are coercible to numbers, but this
    /// may fail with a runtime error under extraordinary circumstances (e.g. if the ECMAScript
    /// `ToNumber` implementation throws an error).
    ///
    /// This will return `std::f64::NAN` if the value has no numerical equivalent.
    pub fn coerce_number<'mv8>(&'mv8 self, value: &Value) -> Result<'mv8, f64> {
        match *value {
            Value::Number(n) => Ok(n),
            ref value => {
                let ffi_result = unsafe {
                    ffi::coerce_number(self.context, value::to_ffi(self, value))
                };
                value::from_ffi_result(self, ffi_result).map(|value| value.as_number().unwrap())
            },
        }
    }

    /// Coerces a value to a boolean (returns `true` if the value is "truthy", `false` otherwise).
    pub fn coerce_boolean(&self, value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            ref value => unsafe {
                ffi::coerce_boolean(self.context, value::to_ffi(self, value)) != 0
            },
        }
    }
}

impl Drop for MiniV8 {
    fn drop(&mut self) {
        unsafe { ffi::context_drop(self.context); }
    }
}
