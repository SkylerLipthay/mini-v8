use crate::*;
use std::fmt;

#[derive(Clone)]
pub struct Function {
    pub(crate) mv8: MiniV8,
    pub(crate) handle: v8::Global<v8::Function>,
}

impl Function {
    /// Consumes the function and downgrades it to a JavaScript object.
    pub fn into_object(self) -> Object {
        self.mv8.clone().scope(|scope| {
            let object: v8::Local<v8::Object> = v8::Local::new(scope, self.handle.clone()).into();
            Object {
                mv8: self.mv8,
                handle: v8::Global::new(scope, object),
            }
        })
    }

    /// Calls the function with the given arguments, with `this` set to `undefined`.
    pub fn call<A, R>(&self, args: A) -> Result<R>
    where
        A: ToValues,
        R: FromValue,
    {
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
        self.mv8.try_catch(|scope| {
            let function = v8::Local::new(scope, self.handle.clone());
            let this = this.to_v8_value(scope);
            let args = args.into_vec();
            let args_v8: Vec<_> = args.into_iter().map(|v| v.to_v8_value(scope)).collect();
            let result = function.call(scope, this, &args_v8);
            self.mv8.exception(scope)?;
            Ok(Value::from_v8_value(&self.mv8, scope, result.unwrap()))
        }).and_then(|v| v.into(&self.mv8))
    }

    /// Calls the function as a constructor function with the given arguments.
    pub fn call_new<A, R>(&self, args: A) -> Result<R>
    where
        A: ToValues,
        R: FromValue,
    {
        let args = args.to_values(&self.mv8)?;
        self.mv8.try_catch(|scope| {
            let function = v8::Local::new(scope, self.handle.clone());
            let args = args.into_vec();
            let args_v8: Vec<_> = args.into_iter().map(|v| v.to_v8_value(scope)).collect();
            let result = function.new_instance(scope, &args_v8);
            self.mv8.exception(scope)?;
            Ok(Value::from_v8_value(&self.mv8, scope, result.unwrap().into()))
        }).and_then(|v| v.into(&self.mv8))
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<function>")
    }
}

/// A bundle of information about an invocation of a function that has been embedded from Rust into
/// JavaScript.
pub struct Invocation {
    /// The `MiniV8` within which the function was called.
    pub mv8: MiniV8,
    /// The value of the function invocation's `this` binding.
    pub this: Value,
    /// The list of arguments with which the function was called.
    pub args: Values,
}
