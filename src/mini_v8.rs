use crate::*;
use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::mem;
use std::string::String as StdString;

/// The entry point into the JavaScript execution environment.
pub struct MiniV8 {
    pub(crate) interface: Interface,
    // Internally, a `ctx` can live in multiple `MiniV8` instances (see
    // `function::create_callback`), so we need to make sure we only drop the V8 context in the
    // top-level "grandparent" `MiniV8`.
    pub(crate) is_top: bool,
}

impl MiniV8 {
    /// Creates a new JavaScript execution environment.
    pub fn new() -> MiniV8 {
        ffi_init();
        let interface = unsafe { mv8_interface_new() };
        let any_map = Box::into_raw(Box::new(AnyMap::new()));
        unsafe { mv8_interface_set_data(interface, DATA_KEY_ANY_MAP, any_map as _); }
        MiniV8 { interface, is_top: true }
    }

    /// Returns the global JavaScript object.
    pub fn global(&self) -> Object {
        Object(Ref::new(self, unsafe { ffi::mv8_interface_global(self.interface) }))
    }

    /// Executes a chunk of JavaScript code and returns its result.
    pub fn eval<'mv8, R: FromValue<'mv8>>(&'mv8 self, source: &str) -> Result<'mv8, R> {
        let len = source.len() as u32;
        let result = unsafe { mv8_interface_eval(self.interface, source.as_ptr(), len) };
        desc_to_result(self, result)?.into(self)
    }

    /// Inserts any sort of keyed value of type `T` into the `MiniV8`, typically for later retrieval
    /// from within Rust functions called from within JavaScript. If a value already exists with the
    /// key, it is returned.
    pub fn set_user_data<K, T>(&mut self, key: K, data: T) -> Option<Box<dyn Any>>
    where
        K: ToString,
        T: Any,
    {
        unsafe {
            let any_map = self.get_any_map();
            (*any_map).insert(key.to_string(), Box::new(data))
        }
    }

    /// Returns a user data value by its key, or `None` if no value exists with the key. If a value
    /// exists but it is not of the type `T`, `None` is returned. This is typically used by a Rust
    /// function called from within JavaScript.
    pub fn get_user_data<T: Any>(&self, key: &str) -> Option<&T> {
        unsafe {
            let any_map = self.get_any_map();
            match (*any_map).get(key) {
                Some(data) => data.downcast_ref::<T>(),
                None => None,
            }
        }
    }

    /// Removes and returns a user data value by its key. Returns `None` if no value exists with the
    /// key.
    pub fn remove_user_data(&mut self, key: &str) -> Option<Box<dyn Any>> {
        unsafe {
            let any_map = self.get_any_map();
            (*any_map).remove(key)
        }
    }

    /// Creates and returns a string managed by V8.
    pub fn create_string(&self, value: &str) -> String {
        let len = value.len() as u32;
        let value_ptr = unsafe { mv8_string_new(self.interface, value.as_ptr(), len) };
        String(Ref::new(self, value_ptr))
    }

    /// Creates and returns an empty `Array` managed by V8.
    pub fn create_array(&self) -> Array {
        Array(Ref::new(self, unsafe { mv8_array_new(self.interface) }))
    }

    /// Creates and returns an empty `Object` managed by V8.
    pub fn create_object(&self) -> Object {
        Object(Ref::new(self, unsafe { mv8_object_new(self.interface) }))
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

    /// Wraps a Rust function or closure, creating a callable JavaScript function handle to it.
    ///
    /// The function's return value is always a `Result`: If the function returns `Err`, the error
    /// is raised as a JavaScript exception, which can be caught within JavaScript or bubbled up
    /// back into Rust by not catching it. This allows using the `?` operator to propagate errors
    /// through intermediate JavaScript code.
    ///
    /// If the function returns `Ok`, the contained value will be converted to a JavaScript value.
    /// For details on Rust-to-JavaScript conversions, refer to the `ToValue` and `ToValues` traits.
    ///
    /// If the provided function panics, the executable will be aborted.
    pub fn create_function<'mv8, 'func, R, F>(&'mv8 self, func: F) -> Function<'mv8>
    where
        R: ToValue<'func>,
        F: Fn(Invocation<'func>) -> Result<'func, R> + 'static,
    {
        let func = move |mv8, this, args| func(Invocation { mv8, this, args })?.to_value(mv8);
        let func_size = mem::size_of_val(&func);
        Function::new(self, Box::new(func), func_size)
    }


    /// Wraps a mutable Rust closure, creating a callable JavaScript function handle to it.
    ///
    /// This is a version of `create_function` that accepts a FnMut argument. Refer to
    /// `create_function` for more information about the implementation.
    pub fn create_function_mut<'mv8, 'func, R, F>(&'mv8 self, func: F) -> Function<'mv8>
    where
        R: ToValue<'func>,
        F: FnMut(Invocation<'func>) -> Result<'func, R> + 'static,
    {
        let func = RefCell::new(func);
        self.create_function(move |invocation| {
            (&mut *func.try_borrow_mut().map_err(|_| Error::recursive_mut_callback())?)(invocation)
        })
    }

    /// Coerces a value to a boolean. Returns `true` if the value is "truthy", `false` otherwise.
    pub fn coerce_boolean<'mv8>(&'mv8 self, value: Value<'mv8>) -> bool {
        match value {
            Value::Boolean(b) => b,
            ref value => unsafe {
                mv8_coerce_boolean(self.interface, value_to_desc(self, &value)) != 0
            },
        }
    }

    /// Coerces a value to a number. Nearly all JavaScript values are coercible to numbers, but this
    /// may fail with a runtime error under extraordinary circumstances (e.g. if the ECMAScript
    /// `ToNumber` implementation throws an error).
    ///
    /// This will return `std::f64::NAN` if the value has no numerical equivalent.
    pub fn coerce_number<'mv8>(&'mv8 self, value: Value<'mv8>) -> Result<'mv8, f64> {
        match value {
            Value::Number(n) => Ok(n),
            value => unsafe {
                let result = mv8_coerce_number(self.interface, value_to_desc(self, &value));
                let number_desc = desc_to_result_val(self, result)?;
                Ok(number_desc.payload.number)
            },
        }
    }

    /// Coerces a value to a string. Nearly all JavaScript values are coercible to strings, but this
    /// may fail with a runtime error if `toString()` fails or under otherwise extraordinary
    /// circumstances (e.g. if the ECMAScript `ToString` implementation throws an error).
    pub fn coerce_string<'mv8>(&'mv8 self, value: Value<'mv8>) -> Result<'mv8, String> {
        match value {
            Value::String(ref s) => Ok(s.clone()),
            ref value => unsafe {
                let result = mv8_coerce_string(self.interface, value_to_desc(self, &value));
                let string_desc = desc_to_result_val(self, result)?;
                Ok(String(Ref::from_value_desc(self, string_desc)))
            },
        }
    }

    unsafe fn get_any_map(&self) -> *mut AnyMap {
        mv8_interface_get_data(self.interface, DATA_KEY_ANY_MAP) as _
    }
}

impl Drop for MiniV8 {
    fn drop(&mut self) {
        if !self.is_top {
            return;
        }

        unsafe {
            let any_map = self.get_any_map();
            mv8_interface_drop(self.interface);
            drop(Box::from_raw(any_map));
        }
    }
}

type AnyMap = BTreeMap<StdString, Box<dyn Any>>;
const DATA_KEY_ANY_MAP: u32 = 0;
