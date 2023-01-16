extern crate mini_v8;

use mini_v8::{MiniV8, Array, Object, Function};

fn main() {
    // A `MiniV8` is a V8 context that can execute JavaScript.
    let mv8 = MiniV8::new();

    // JavaScript can be evaluated and transformed into various Rust types.
    let value: usize = mv8.eval("2 + 2").unwrap();
    assert_eq!(value, 4);
    let value: String = mv8.eval("`Two plus two is ${2 + 2}`").unwrap();
    assert_eq!(value, "Two plus two is 4".to_string());

    // JavaScript objects can be directly manipulated, without eagerly converting into Rust.
    let array: Array = mv8.eval("[123, 'abc']").unwrap();
    let element: String = array.get(1).unwrap();
    assert_eq!(element, "abc".to_string());
    array.set(0, 456).unwrap();

    // JavaScript values can be created directly, without using `mv8.eval` as above.
    let object: Object = mv8.create_object();
    let js_string = mv8.create_string("This string is owned by JavaScript!");
    object.set("someString", js_string).unwrap();

    // Rust functions can be passed into JavaScript.
    let rust_add = mv8.create_function(|inv| {
        let (a, b): (f64, f64) = inv.args.into(&inv.mv8)?;
        Ok(a + b)
    });
    // Like any other value, these functions can be bound as properties of an object.
    //
    // (Notice: Cloning values just creates a new reference to the value, similar to JavaScript's
    // own object referencing semantics.)
    object.set("add", rust_add.clone()).unwrap();

    // JavaScript functions can be passed into Rust.
    let js_add: Function = mv8.eval("(a, b) => a + b").unwrap();
    // Functions can be called from within Rust.
    let value: f64 = rust_add.call((1, 2)).unwrap();
    assert_eq!(value, 3.0);
    let value: f64 = js_add.call((1, 2)).unwrap();
    assert_eq!(value, 3.0);
}
