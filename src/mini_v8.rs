use crate::*;
use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::string::String as StdString;
use std::sync::{Arc, Condvar, Mutex, Once};
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub struct MiniV8 {
    interface: Interface,
}

impl MiniV8 {
    pub fn new() -> MiniV8 {
        initialize_v8();
        let mut isolate = v8::Isolate::new(Default::default());
        initialize_slots(&mut isolate);
        MiniV8 { interface: Interface::new(isolate) }
    }

    /// Returns the global JavaScript object.
    pub fn global(&self) -> Object {
        self.scope(|scope| {
            let global = scope.get_current_context().global(scope);
            Object {
                mv8: self.clone(),
                handle: v8::Global::new(scope, global),
            }
        })
    }

    /// Executes a JavaScript script and returns its result.
    pub fn eval<S, R>(&self, script: S) -> Result<R>
    where
        S: Into<Script>,
        R: FromValue,
    {
        let script = script.into();
        let isolate_handle = self.interface.isolate_handle();
        match (self.interface.len() == 1, script.timeout) {
            (true, Some(timeout)) => {
                execute_with_timeout(
                    timeout,
                    || self.eval_inner(script),
                    move || { isolate_handle.terminate_execution(); },
                )?.into(self)
            },
            (false, Some(_)) => Err(Error::InvalidTimeout),
            (_, None) => self.eval_inner(script)?.into(self),
        }
    }

    fn eval_inner(&self, script: Script) -> Result<Value> {
        self.try_catch(|scope| {
            let source = create_string(scope, &script.source);
            let origin = script.origin.map(|o| {
                let name = create_string(scope, &o.name).into();
                let source_map_url = create_string(scope, "").into();
                v8::ScriptOrigin::new(
                    scope,
                    name,
                    o.line_offset,
                    o.column_offset,
                    false,
                    0,
                    source_map_url,
                    true,
                    false,
                    false,
                )
            });
            let script = v8::Script::compile(scope, source, origin.as_ref());
            self.exception(scope)?;
            let result = script.unwrap().run(scope);
            self.exception(scope)?;
            Ok(Value::from_v8_value(self, scope, result.unwrap()))
        })
    }

    /// Inserts any sort of keyed value of type `T` into the `MiniV8`, typically for later retrieval
    /// from within Rust functions called from within JavaScript. If a value already exists with the
    /// key, it is returned.
    pub fn set_user_data<K, T>(&self, key: K, data: T) -> Option<Box<dyn Any>>
    where
        K: ToString,
        T: Any,
    {
        self.interface.use_slot(|m: &AnyMap| m.0.borrow_mut().insert(key.to_string(), Box::new(data)))
    }

    /// Calls a function with a user data value by its key, or `None` if no value exists with the
    /// key. If a value exists but it is not of the type `T`, `None` is returned. This is typically
    /// used by a Rust function called from within JavaScript.
    pub fn use_user_data<F, T: Any, U>(&self, key: &str, func: F) -> U
    where
        F: FnOnce(Option<&T>) -> U + 'static,
    {
        self.interface.use_slot(|m: &AnyMap| {
            func(m.0.borrow().get(key).and_then(|d| d.downcast_ref::<T>()))
        })
    }

    /// Removes and returns a user data value by its key. Returns `None` if no value exists with the
    /// key.
    pub fn remove_user_data(&self, key: &str) -> Option<Box<dyn Any>> {
        self.interface.use_slot(|m: &AnyMap| m.0.borrow_mut().remove(key))
    }

    /// Creates and returns a string managed by V8.
    ///
    /// # Panics
    ///
    /// Panics if source value is longer than `(1 << 28) - 16` bytes.
    pub fn create_string(&self, value: &str) -> String {
        self.scope(|scope| {
            let string = create_string(scope, value);
            String {
                mv8: self.clone(),
                handle: v8::Global::new(scope, string),
            }
        })
    }

    /// Creates and returns an empty `Array` managed by V8.
    pub fn create_array(&self) -> Array {
        self.scope(|scope| {
            let array = v8::Array::new(scope, 0);
            Array {
                mv8: self.clone(),
                handle: v8::Global::new(scope, array),
            }
        })
    }

    /// Creates and returns an empty `Object` managed by V8.
    pub fn create_object(&self) -> Object {
        self.scope(|scope| {
            let object = v8::Object::new(scope);
            Object {
                mv8: self.clone(),
                handle: v8::Global::new(scope, object),
            }
        })
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
    pub fn create_function<F, R>(&self, func: F) -> Function
    where
        F: Fn(Invocation) -> Result<R> + 'static,
        R: ToValue,
    {
        let func = move |mv8: &MiniV8, this: Value, args: Values| {
            func(Invocation { mv8: mv8.clone(), this, args })?.to_value(mv8)
        };

        self.scope(|scope| {
            let callback = Box::new(func);
            let callback_info = CallbackInfo { mv8: self.clone(), callback };
            let ptr = Box::into_raw(Box::new(callback_info));
            let ext = v8::External::new(scope, ptr as _);

            let v8_func = |
                scope: &mut v8::HandleScope,
                fca: v8::FunctionCallbackArguments,
                mut rv: v8::ReturnValue,
            | {
                let data = fca.data();
                let ext = v8::Local::<v8::External>::try_from(data).unwrap();
                let callback_info_ptr = ext.value() as *mut CallbackInfo;
                let callback_info = unsafe { &mut *callback_info_ptr };
                let CallbackInfo { mv8, callback } = callback_info;
                let ptr = scope as *mut v8::HandleScope;
                // We can erase the lifetime of the `v8::HandleScope` safely because it only lives
                // on the interface stack during the current block:
                let ptr: *mut v8::HandleScope<'static> = unsafe { std::mem::transmute(ptr) };
                mv8.interface.push(ptr);
                let this = Value::from_v8_value(&mv8, scope, fca.this().into());
                let len = fca.length();
                let mut args = Vec::with_capacity(len as usize);
                for i in 0..len {
                    args.push(Value::from_v8_value(&mv8, scope, fca.get(i)));
                }
                match callback(&mv8, this, Values::from_vec(args)) {
                    Ok(v) => {
                        rv.set(v.to_v8_value(scope));
                    },
                    Err(e) => {
                        let exception = e.to_value(&mv8).to_v8_value(scope);
                        scope.throw_exception(exception);
                    },
                };
                mv8.interface.pop();
            };

            let value = v8::Function::builder(v8_func).data(ext.into()).build(scope).unwrap();
            // TODO: `v8::Isolate::adjust_amount_of_external_allocated_memory` should be called
            // appropriately with the following external resource size calculation. This cannot be
            // done as of now, since `v8::Weak::with_guaranteed_finalizer` does not provide a
            // `v8::Isolate` to the finalizer callback, and so the downward adjustment cannot be
            // made.
            //
            // let func_size = mem::size_of_val(&func); let ext_size = func_size +
            // mem::size_of::<CallbackInfo>;
            let drop_ext = Box::new(move || drop(unsafe { Box::from_raw(ptr) }));
            add_finalizer(scope, value, drop_ext);
            Function {
                mv8: self.clone(),
                handle: v8::Global::new(scope, value),
            }
        })
    }

    /// Wraps a mutable Rust closure, creating a callable JavaScript function handle to it.
    ///
    /// This is a version of `create_function` that accepts a FnMut argument. Refer to
    /// `create_function` for more information about the implementation.
    pub fn create_function_mut<F, R>(&self, func: F) -> Function
    where
        F: FnMut(Invocation) -> Result<R> + 'static,
        R: ToValue,
    {
        let func = RefCell::new(func);
        self.create_function(move |invocation| {
            (&mut *func.try_borrow_mut().map_err(|_| Error::RecursiveMutCallback)?)(invocation)
        })
    }

    // Opens a new handle scope in the global context. Nesting calls to this or `MiniV8::try_catch`
    // will cause a panic (unless a callback is entered, see `MiniV8::create_function`).
    pub(crate) fn scope<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut v8::ContextScope<v8::HandleScope>) -> T,
    {
        self.interface.scope(func)
    }

    // Opens a new try-catch scope in the global context. Nesting calls to this or `MiniV8::scope`
    // will cause a panic (unless a callback is entered, see `MiniV8::create_function`).
    pub(crate) fn try_catch<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut v8::TryCatch<v8::HandleScope>) -> T,
    {
        self.interface.try_catch(func)
    }

    pub(crate) fn exception(&self, scope: &mut v8::TryCatch<v8::HandleScope>) -> Result<()> {
        if scope.has_terminated() {
            Err(Error::Timeout)
        } else if let Some(exception) = scope.exception() {
            Err(Error::Value(Value::from_v8_value(self, scope, exception)))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
struct Interface(Rc<RefCell<Vec<Rc<RefCell<InterfaceEntry>>>>>);

impl Interface {
    fn len(&self) -> usize {
        self.0.borrow().len()
    }

    fn isolate_handle(&self) -> v8::IsolateHandle {
        self.top(|entry| entry.isolate_handle())
    }

    // Opens a new handle scope in the global context.
    fn scope<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut v8::ContextScope<v8::HandleScope>) -> T,
    {
        self.top(|entry| entry.scope(func))
    }

    // Opens a new try-catch scope in the global context.
    fn try_catch<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut v8::TryCatch<v8::HandleScope>) -> T,
    {
        self.scope(|scope| func(&mut v8::TryCatch::new(scope)))
    }

    fn new(isolate: v8::OwnedIsolate) -> Interface {
        Interface(Rc::new(RefCell::new(vec![Rc::new(RefCell::new(InterfaceEntry::Isolate(isolate)))])))
    }

    fn push(&self, handle_scope: *mut v8::HandleScope<'static>) {
        self.0.borrow_mut().push(Rc::new(RefCell::new(InterfaceEntry::HandleScope(handle_scope))));
    }

    fn pop(&self) {
        self.0.borrow_mut().pop();
    }

    fn use_slot<F, T: 'static, U>(&self, func: F) -> U
    where
        F: FnOnce(&T) -> U,
    {
        self.top(|entry| func(entry.get_slot()))
    }

    fn top<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut InterfaceEntry) -> T,
    {
        let top = self.0.borrow().last().unwrap().clone();
        let mut top_mut = top.borrow_mut();
        func(&mut top_mut)
    }
}

enum InterfaceEntry {
    Isolate(v8::OwnedIsolate),
    HandleScope(*mut v8::HandleScope<'static>),
}

impl InterfaceEntry {
    fn scope<F, T>(&mut self, func: F) -> T
    where
        F: FnOnce(&mut v8::ContextScope<v8::HandleScope>) -> T,
    {
        match self {
            InterfaceEntry::Isolate(isolate) => {
                let global_context = isolate.get_slot::<Global>().unwrap().context.clone();
                let scope = &mut v8::HandleScope::new(isolate);
                let context = v8::Local::new(scope, global_context);
                let scope = &mut v8::ContextScope::new(scope, context);
                func(scope)
            },
            InterfaceEntry::HandleScope(ref ptr) => {
                let scope: &mut v8::HandleScope = unsafe { &mut **ptr };
                let scope = &mut v8::ContextScope::new(scope, scope.get_current_context());
                func(scope)
            },
        }
    }

    fn get_slot<T: 'static>(&self) -> &T {
        match self {
            InterfaceEntry::Isolate(isolate) => isolate.get_slot::<T>().unwrap(),
            InterfaceEntry::HandleScope(ref ptr) => {
                let scope: &mut v8::HandleScope = unsafe { &mut **ptr };
                scope.get_slot::<T>().unwrap()
            },
        }
    }

    fn isolate_handle(&self) -> v8::IsolateHandle {
        match self {
            InterfaceEntry::Isolate(isolate) => isolate.thread_safe_handle(),
            InterfaceEntry::HandleScope(ref ptr) => {
                let scope: &mut v8::HandleScope = unsafe { &mut **ptr };
                scope.thread_safe_handle()
            },
        }
    }
}

struct Global {
    context: v8::Global<v8::Context>,
}

static INIT: Once = Once::new();

fn initialize_v8() {
    INIT.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

fn initialize_slots(isolate: &mut v8::Isolate) {
    let scope = &mut v8::HandleScope::new(isolate);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let global_context = v8::Global::new(scope, context);
    scope.set_slot(Global { context: global_context });
    scope.set_slot(AnyMap(Rc::new(RefCell::new(BTreeMap::new()))));
}

fn create_string<'s>(scope: &mut v8::HandleScope<'s>, value: &str) -> v8::Local<'s, v8::String> {
    v8::String::new(scope, value).expect("string exceeds maximum length")
}

fn add_finalizer<T: 'static>(
    isolate: &mut v8::Isolate,
    handle: impl v8::Handle<Data = T>,
    finalizer: impl FnOnce() + 'static,
) {
    let rc = Rc::new(RefCell::new(None));
    let weak = v8::Weak::with_guaranteed_finalizer(isolate, handle, Box::new({
        let rc = rc.clone();
        move || {
            let weak = rc.replace(None).unwrap();
            finalizer();
            drop(weak);
        }
    }));
    rc.replace(Some(weak));
}

type Callback = Box<dyn Fn(&MiniV8, Value, Values) -> Result<Value>>;

struct CallbackInfo {
    mv8: MiniV8,
    callback: Callback,
}

struct AnyMap(Rc<RefCell<BTreeMap<StdString, Box<dyn Any>>>>);

// A JavaScript script.
#[derive(Clone, Debug, Default)]
pub struct Script {
    /// The source of the script.
    pub source: StdString,
    /// The maximum runtime duration of the script's execution. This cannot be set within a nested
    /// evaluation, i.e. it cannot be set when calling `MiniV8::eval` from within a `Function`
    /// created with `MiniV8::create_function` or `MiniV8::create_function_mut`.
    ///
    /// V8 can only cancel script evaluation while running actual JavaScript code. If Rust code is
    /// being executed when the timeout is triggered, the execution will continue until the
    /// evaluation has returned to running JavaScript code.
    pub timeout: Option<Duration>,
    /// The script's origin.
    pub origin: Option<ScriptOrigin>,
}

/// The origin, within a file, of a JavaScript script.
#[derive(Clone, Debug, Default)]
pub struct ScriptOrigin {
    /// The name of the file this script belongs to.
    pub name: StdString,
    /// The line at which this script starts.
    pub line_offset: i32,
    /// The column at which this script starts.
    pub column_offset: i32,
}

impl From<StdString> for Script {
    fn from(source: StdString) -> Script {
        Script { source, ..Default::default() }
    }
}

impl<'a> From<&'a str> for Script {
    fn from(source: &'a str) -> Script {
        source.to_string().into()
    }
}

fn execute_with_timeout<T>(
    timeout: Duration,
    execute_fn: impl FnOnce() -> T,
    timed_out_fn: impl FnOnce() + Send + 'static,
) -> T {
    let wait = Arc::new((Mutex::new(true), Condvar::new()));
    let timer_wait = wait.clone();
    thread::spawn(move || {
        let (mutex, condvar) = &*timer_wait;
        let timer = condvar.wait_timeout_while(
            mutex.lock().unwrap(),
            timeout,
            |&mut is_executing| is_executing,
        ).unwrap();
        if timer.1.timed_out() {
            timed_out_fn();
        }
    });

    let result = execute_fn();
    let (mutex, condvar) = &*wait;
    *mutex.lock().unwrap() = false;
    condvar.notify_one();
    result
}
