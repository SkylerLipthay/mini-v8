use crate::mini_v8::MiniV8;
use crate::error::Result;
use crate::function::Invocation;
use crate::object::Object;
use crate::value::Value;

#[test]
fn contains_key() {
    let mv8 = MiniV8::new();
    let globals = mv8.global();
    assert!(globals.contains_key("Array").unwrap());
    assert!(!globals.contains_key("~NOT-EXIST~").unwrap());
}

#[test]
fn set_get() {
    let mv8 = MiniV8::new();

    let object = mv8.create_object();
    object.set("a", 123).unwrap();
    object.set(123, "a").unwrap();
    let parent = mv8.create_object();
    parent.set("obj", object).unwrap();
    let object: Object = parent.get("obj").unwrap();
    assert_eq!(object.get::<_, i8>("a").unwrap(), 123);
    assert_eq!(object.get::<_, String>("a").unwrap(), "123");
    assert_eq!(object.get::<_, String>("123").unwrap(), "a");
    assert_eq!(object.get::<_, String>(123).unwrap(), "a");
}

#[test]
fn remove() {
    let mv8 = MiniV8::new();
    let globals = mv8.global();
    assert!(globals.contains_key("Object").unwrap());
    globals.remove("Object").unwrap();
    assert!(!globals.contains_key("Object").unwrap());
    // Removing keys that don't exist does nothing:
    globals.remove("Object").unwrap();
    assert!(!globals.contains_key("Object").unwrap());
}

#[test]
fn call_prop() {
    fn add(inv: Invocation) -> Result<usize> {
        let this: Object = inv.this.into(inv.mv8)?;
        let (acc,): (usize,) = inv.args.into(inv.mv8)?;
        return Ok(this.get::<_, usize>("base").unwrap() + acc);
    }

    let mv8 = MiniV8::new();
    let object = mv8.create_object();
    object.set("base", 123).unwrap();
    object.set("add", mv8.create_function(add)).unwrap();
    let number: f64 = object.call_prop("add", (456,)).unwrap();
    assert_eq!(number, 579.0f64);
}

#[test]
fn keys() {
    let mv8 = MiniV8::new();
    let object = mv8.create_object();
    object.set("c", 3).unwrap();
    object.set("b", 2).unwrap();
    object.set("a", 1).unwrap();
    let keys: Result<Vec<String>> = object.keys(true).elements().collect();
    assert_eq!(keys.unwrap(), vec!["c".to_string(), "b".to_string(), "a".to_string()])
}

#[test]
fn properties() {
    let mv8 = MiniV8::new();

    let object = mv8.create_object();
    object.set("a", 123).unwrap();
    object.set(4, Value::Undefined).unwrap();
    object.set(123, "456").unwrap();

    let list = object.properties(false).map(|property| {
        let result: (String, usize) = property.unwrap();
        result
    }).collect::<Vec<_>>();

    assert_eq!(list, vec![("4".to_string(), 0), ("123".to_string(), 456), ("a".to_string(), 123)]);
}
