use crate::*;
use std::string::String as StdString;

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
    assert_eq!(object.get::<_, StdString>("a").unwrap(), "123");
    assert_eq!(object.get::<_, StdString>("123").unwrap(), "a");
    assert_eq!(object.get::<_, StdString>(123).unwrap(), "a");
}


#[test]
fn remove() {
    let mv8 = MiniV8::new();
    let globals = mv8.global();
    assert!(globals.has("Object").unwrap());
    globals.remove("Object").unwrap();
    assert!(!globals.has("Object").unwrap());
    // Removing keys that don't exist does nothing:
    globals.remove("Object").unwrap();
    assert!(!globals.has("Object").unwrap());
}

#[test]
fn has() {
    let mv8 = MiniV8::new();
    let globals = mv8.global();
    assert!(globals.has("Array").unwrap());
    assert!(!globals.has("~NOT-EXIST~").unwrap());
}
