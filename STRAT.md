```rust
struct MiniV8;
fn create_function(&self, ...) -> Function;
fn create_function_mut(&self, ...) -> Function;
fn set_user_data<K, T>(&mut self, key: K, data: T) -> Option<Box<Any + 'static>>;
fn get_user_data<T: Any + 'static>(&self, key: &str) -> Option<&T>;
fn remove_user_data(&mut self, key: &str) -> Option<Box<Any + 'static>>;

struct Function;
fn call<A: ToValues, R: FromValue>(&self, args: A) -> Result<R>;
fn call_method<T: ToValue, A: ToValues, R: FromValue>(&self, this: T, args: A) -> Result<R>;

struct Invocation;

struct Object;
fn call_prop<K: ToValue, A: ToValues, R: FromValue>(&self, key: K, args: A) -> Result<R>;
```

* Implement `Function` (and `Object::call_prop`)
* Finishing implementing `conversion.rs`
* Figure out `any_map` stuff
