use crate::error::Result;
use crate::ffi;
use crate::object::Object;
use crate::types::Ref;
use crate::value::{self, FromValue, ToValue, ToValues, Value};
use std::{cmp, i32};

/// Reference to a JavaScript function.
#[derive(Clone, Debug)]
pub struct Function<'mv8>(pub(crate) Ref<'mv8>);

impl<'mv8> Function<'mv8> {
    /// Consumes the function and downgrades it to a JavaScript object. This is inexpensive, since
    /// an array *is* an object.
    pub fn into_object(self) -> Object<'mv8> {
        Object(self.0)
    }

    /// Calls the function with the given arguments, with `this` set to `undefined`.
    pub fn call<A, R>(&self, args: A) -> Result<'mv8, R>
    where
        A: ToValues<'mv8>,
        R: FromValue<'mv8>,
    {
        self.call_method(Value::Undefined, args)
    }

    /// Calls the function with the given `this` and arguments.
    pub fn call_method<T, A, R>(&self, this: T, args: A) -> Result<'mv8, R>
    where
        T: ToValue<'mv8>,
        A: ToValues<'mv8>,
        R: FromValue<'mv8>,
    {
        let mv8 = self.0.mv8;
        let this = this.to_value(mv8)?;
        let args = args.to_values(mv8)?;

        let ffi_this = value::to_ffi(mv8, &this);
        let ffi_args: Vec<_> = args.iter().map(|arg| value::to_ffi(mv8, arg)).collect();
        let ffi_result = unsafe {
            ffi::function_call(
                mv8.context,
                self.0.value,
                ffi_this,
                ffi_args.as_ptr(),
                cmp::min(ffi_args.len(), i32::MAX as usize) as i32,
            )
        };

        value::from_ffi_result(mv8, ffi_result).and_then(|v| v.into(mv8))
    }
}
