use crate::*;
use rusty_v8 as v8;
use std::fmt;

/// Reference to a JavaScript array.
///
/// Attempts to interact with an instance after its parent `MiniV8` is dropped will result in a
/// panic.
pub struct Function {
    pub(crate) value: v8::Global<v8::Function>,
    mv8: MiniV8,
}

impl Function {
    /// Consumes the function and downgrades it to a JavaScript object. This is inexpensive, since a
    /// function *is* an object.
    pub fn into_object(self) -> Object {
        let object = self.mv8.scope(|scope| {
            let object: v8::Local<v8::Object> = v8::Local::new(scope, &self.value).into();
            v8::Global::<v8::Object>::new(scope, object)
        });

        Object::new(&self.mv8, object)
    }

    /// Calls the function with the given arguments, with `this` set to `undefined`.
    pub fn call<A: ToValues, R: FromValue>(&self, args: A) -> Result<R> {
        self.call_method(Value::Undefined, args)
    }

    /// Calls the function with the given `this` and arguments.
    pub fn call_method<T, A, R>(&self, this: T, args: A) -> Result<R>
    where
        T: ToValue,
        A: ToValues,
        R: FromValue,
    {
        let this = this.to_value(&self.mv8)?;
        let args = args.to_values(&self.mv8)?;

        self.mv8.try_catch_scope(|scope| {
            let native_this = this.to_native(scope);
            let native_args: Vec<_> = args.into_vec().into_iter().map(|arg| arg.to_native(scope))
                .collect();
            let function = self.value.get(scope);
            let maybe_value = function.call(scope, native_this, &native_args);
            if let Some(exception) = scope.exception() {
                Err(Error::Value(Value::from_native(&self.mv8, scope, exception)))
            } else {
                Ok(match maybe_value {
                    Some(value) => Value::from_native(&self.mv8, scope, value),
                    None => Value::Undefined,
                })
            }
        }).and_then(|v| R::from_value(v, &self.mv8))
    }

    pub(crate) fn new(mv8: &MiniV8, value: v8::Global<v8::Function>) -> Function {
        Function { value, mv8: mv8.weak() }
    }
}

impl Clone for Function {
    fn clone(&self) -> Function {
        Function { value: self.value.clone(), mv8: self.mv8.weak() }
    }
}

impl fmt::Debug for Function {
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
    pub this: Value,
    /// The list of arguments with which the function was called.
    pub args: Values,
}
