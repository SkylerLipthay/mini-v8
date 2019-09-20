use crate::error::Result;
use crate::ffi;
use crate::mini_v8::MiniV8;
use crate::object::Object;
use crate::types::{Callback, Ref};
use crate::value::{self, FromValue, ToValue, ToValues, Value, Values};
use std::{cmp, fmt, i32, process, slice};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Reference to a JavaScript function.
#[derive(Clone)]
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

        let ffi_this = value::to_ffi(mv8, &this, false);
        let ffi_args: Vec<_> = args.iter().map(|arg| value::to_ffi(mv8, arg, false)).collect();
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

impl<'mv8> fmt::Debug for Function<'mv8> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<function>")
    }
}

/// A bundle of information about an invocation of a function that has been embedded from Rust into
/// JavaScript.
pub struct Invocation<'mv8> {
    /// The `MiniV8` within which the function was called.
    pub mv8: &'mv8 MiniV8,
    /// The value of the function invocation's `this` binding.
    pub this: Value<'mv8>,
    /// The list of arguments with which the function was called.
    pub args: Values<'mv8>,
}

pub(crate) fn create_callback<'mv8, 'callback>(
    mv8: &'mv8 MiniV8,
    func: Callback<'callback, 'static>,
) -> Function<'mv8> {
    let ffi_func = unsafe { ffi::function_create(mv8.context, Box::into_raw(Box::new(func)) as _) };
    Function(Ref::from_persistent(mv8, ffi_func))
}

pub(crate) unsafe extern "C" fn callback_wrapper(
    context: ffi::Context,
    callback_ptr: *const std::ffi::c_void,
    ffi_this: ffi::Value,
    ffi_args: *const ffi::Value,
    num_args: i32,
) -> ffi::EvalResult {
    let inner = || {
        let mv8 = MiniV8 { context, is_top: false };
        let this = value::from_ffi(&mv8, ffi_this);
        let ffi_args_arr = slice::from_raw_parts(ffi_args, num_args as usize);
        let args: Vec<Value> = ffi_args_arr.iter().map(|v| value::from_ffi(&mv8, *v)).collect();
        let args = Values::from_vec(args);

        let callback = callback_ptr as *mut Callback;
        let result = (*callback)(&mv8, this, args);
        let (exception, value) = match result {
            Ok(value) => (0, value),
            Err(value) => (1, value.to_value(&mv8)),
        };
        let value = value::to_ffi(&mv8, &value, true);
        ffi::EvalResult { exception, value }
    };

    match catch_unwind(AssertUnwindSafe(inner)) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("panic during rust function embedded in v8: {:?}", err);
            // Unfortunately I don't think there's a clean way to unwind normally, so we'll have to
            // abort the entire process without destructing its threads.
            process::abort();
        },
    }
}

pub(crate) unsafe extern "C" fn callback_drop(callback: *mut Callback) {
    Box::from_raw(callback);
}
