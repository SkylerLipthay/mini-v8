use crate::*;

#[test]
fn coerce_boolean() {
    let mv8 = MiniV8::new();
    assert!(!Value::Undefined.coerce_boolean(&mv8));
    assert!(!Value::Null.coerce_boolean(&mv8));
    assert!(!Value::Number(0.0).coerce_boolean(&mv8));
    assert!(Value::Number(1.0).coerce_boolean(&mv8));
    assert!(!Value::String(mv8.create_string("")).coerce_boolean(&mv8));
    assert!(Value::String(mv8.create_string("a")).coerce_boolean(&mv8));
    assert!(Value::Object(mv8.create_object()).coerce_boolean(&mv8));
}

#[test]
fn coerce_number() {
    let mv8 = MiniV8::new();
    assert!(Value::Undefined.coerce_number(&mv8).unwrap().is_nan());
    assert_eq!(0.0, Value::Null.coerce_number(&mv8).unwrap());
    assert_eq!(0.0, Value::Number(0.0).coerce_number(&mv8).unwrap());
    assert_eq!(1.0, Value::Number(1.0).coerce_number(&mv8).unwrap());
    assert_eq!(0.0, Value::String(mv8.create_string("")).coerce_number(&mv8).unwrap());
    assert!(Value::String(mv8.create_string("a")).coerce_number(&mv8).unwrap().is_nan());
    assert!(Value::Object(mv8.create_object()).coerce_number(&mv8).unwrap().is_nan());
}

#[test]
fn coerce_string() {
    fn assert_string_eq(mv8: &MiniV8, value: Value, expected: &str) {
        assert_eq!(expected, value.coerce_string(mv8).unwrap().to_string());
    }

    let mv8 = MiniV8::new();
    assert_string_eq(&mv8, Value::Undefined, "undefined");
    assert_string_eq(&mv8, Value::Null, "null");
    assert_string_eq(&mv8, Value::Number(123.0), "123");
    assert_string_eq(&mv8, Value::String(mv8.create_string("abc")), "abc");
    assert_string_eq(&mv8, Value::Object(mv8.create_object()), "[object Object]");
}
