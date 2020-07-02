use crate::*;
use rusty_v8 as v8;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Once;

/// The entry point into the JavaScript execution environment.
pub struct MiniV8 {
    instance: RefInstance,
}

impl MiniV8 {
    /// Creates a new JavaScript execution environment.
    pub fn new() -> MiniV8 {
        MiniV8 { instance: RefInstance::Strong(Rc::new(Instance::new())) }
    }

    /// Executes a chunk of JavaScript code and returns its result.
    pub fn eval<R: FromValue>(&self, source: &str) -> Result<R> {
        self.try_catch_scope(|scope| {
            let source = v8::String::new(scope, source).expect("script too long");
            if let Some(script) = v8::Script::compile(scope, source, None) {
                if let Some(value) = script.run(scope) {
                    return Ok(Value::from_native(self, scope, value));
                }
            }
            let exception = scope.exception().unwrap_or_else(|| v8::undefined(scope).into());
            Err(Error::Value(Value::from_native(self, scope, exception)))
        }).and_then(|v| R::from_value(v, self))
    }

    /// Returns the global JavaScript object.
    pub fn global(&self) -> Object {
        Object::new(self, {
            let instance = self.instance();
            let mut isolate = instance.isolate.borrow_mut();
            let scope = &mut v8::HandleScope::new(&mut *isolate);
            let global = instance.context.get(scope).global(scope);
            v8::Global::<v8::Object>::new(scope, global)
        })
    }

    /// Creates and returns a string managed by V8.
    pub fn create_string(&self, value: &str) -> String {
        String::new(self, self.scope(|scope| {
            let string = v8::String::new(scope, value).expect("string too long");
            v8::Global::<v8::String>::new(scope, string)
        }))
    }

    /// Creates and returns an empty `Object` managed by V8.
    pub fn create_object(&self) -> Object {
        Object::new(self, self.context_scope(|scope| {
            let object = v8::Object::new(scope);
            v8::Global::<v8::Object>::new(scope, object)
        }))
    }

    /// Creates and returns an empty `Array` managed by V8.
    pub fn create_array(&self) -> Array {
        Array::new(self, self.context_scope(|scope| {
            let array = v8::Array::new(scope, 0);
            v8::Global::<v8::Array>::new(scope, array)
        }))
    }

    /// Creates and returns an `Object` managed by V8 filled with the keys and values from an
    /// iterator. Keys are coerced to object properties.
    ///
    /// This is a thin wrapper around `MiniV8::create_object` and `Object::set`. See `Object::set`
    /// for how this method might return an error.
    pub fn create_object_from<K, V, I>(&self, iter: I) -> Result<Object>
    where
        K: ToValue,
        V: ToValue,
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
    pub fn coerce_string(&self, value: Value) -> Result<String> {
        if let Value::String(ref s) = value {
            return Ok(s.clone());
        }

        self.try_catch_scope(|scope| {
            let maybe_string = value.to_native(scope).to_string(scope);
            match scope.exception() {
                Some(exception) => Err(Error::Value(Value::from_native(self, scope, exception))),
                None => {
                    let string = maybe_string.unwrap_or_else(|| v8::String::empty(scope));
                    Ok(String::new(self, v8::Global::<v8::String>::new(scope, string)))
                },
            }
        })
    }

    /// Coerces a value to a number. Nearly all JavaScript values are coercible to numbers, but this
    /// may fail with a runtime error under extraordinary circumstances (e.g. if the ECMAScript
    /// `ToNumber` implementation throws an error).
    ///
    /// This will return `std::f64::NAN` if the value has no numerical equivalent.
    pub fn coerce_number(&self, value: Value) -> Result<f64> {
        if let Value::Number(n) = value {
            return Ok(n);
        }

        self.try_catch_scope(|scope| {
            let maybe_number = value.to_native(scope).number_value(scope);
            match scope.exception() {
                Some(exception) => Err(Error::Value(Value::from_native(self, scope, exception))),
                None => Ok(maybe_number.unwrap_or(f64::NAN)),
            }
        })
    }

    /// Coerces a value to a boolean (returns `true` if the value is "truthy", `false` otherwise).
    pub fn coerce_boolean(&self, value: Value) -> bool {
        match value {
            Value::Boolean(b) => b,
            ref value => self.context_scope(|scope| value.to_native(scope).boolean_value(scope)),
        }
    }

    // The following `scope` functions are used to enter the isolate. Great care must be taken to
    // not recursively call these functions, otherwise a panic will occur for having mutably
    // borrowed the isolate more than once. Every time you call one of these functions, ask
    // yourself: "Is there anything inside of this passed closure that will call a scoping function
    // again?" If so, refactor!

    pub(crate) fn scope<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut v8::HandleScope<()>) -> T,
    {
        let instance = self.instance();
        let mut isolate = instance.isolate.borrow_mut();
        let scope = &mut v8::HandleScope::new(&mut *isolate);
        f(scope)
    }

    pub(crate) fn context_scope<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut v8::ContextScope<v8::HandleScope>) -> T,
    {
        let instance = self.instance();
        let mut isolate = instance.isolate.borrow_mut();
        let scope = &mut v8::HandleScope::new(&mut *isolate);
        let local_context = v8::Local::new(scope, &instance.context);
        let context_scope = &mut v8::ContextScope::new(scope, local_context);
        f(context_scope)
    }

    pub(crate) fn try_catch_scope<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut v8::TryCatch<v8::HandleScope>) -> T,
    {
        let instance = self.instance();
        let mut isolate = instance.isolate.borrow_mut();
        let scope = &mut v8::HandleScope::new(&mut *isolate);
        let local_context = v8::Local::new(scope, &instance.context);
        let context_scope = &mut v8::ContextScope::new(scope, local_context);
        let tc_scope = &mut v8::TryCatch::new(context_scope);
        f(tc_scope)
    }

    pub(crate) fn weak(&self) -> MiniV8 {
        let weak = match self.instance {
            RefInstance::Strong(ref rc) => Rc::downgrade(rc),
            RefInstance::Weak(ref weak) => weak.clone(),
        };

        MiniV8 { instance: RefInstance::Weak(weak) }
    }

    fn instance(&self) -> Rc<Instance> {
        match self.instance {
            RefInstance::Strong(ref rc) => rc.clone(),
            RefInstance::Weak(ref weak) => {
                weak.upgrade().expect("referred to `MiniV8` after disposal")
            },
        }
    }
}

struct Instance {
    isolate: RefCell<v8::OwnedIsolate>,
    context: v8::Global<v8::Context>,
}

impl Instance {
    fn new() -> Instance {
        ensure_v8_is_initialized();
        let mut isolate = v8::Isolate::new(Default::default());
        let context = create_context(&mut isolate);
        Instance { isolate: RefCell::new(isolate), context }
    }
}

fn ensure_v8_is_initialized() {
    static START: Once = Once::new();
    START.call_once(|| {
      v8::V8::initialize_platform(v8::new_default_platform().unwrap());
      v8::V8::initialize();
    });
}

fn create_context(isolate: &mut v8::OwnedIsolate) -> v8::Global<v8::Context> {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    v8::Global::<v8::Context>::new(scope, context)
}

enum RefInstance {
    Strong(Rc<Instance>),
    Weak(Weak<Instance>),
}
