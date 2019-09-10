#![allow(dead_code)]

extern crate mini_v8;

use mini_v8::{MiniV8, Value};

fn main() {
    sample_2();
}

fn cross_contaminate() {
    let context_a = MiniV8::new();
    let context_b = MiniV8::new();
    let array_a = context_a.create_array();
    let array_b = context_b.create_array();
    array_a.push(Value::Array(array_b.clone())).unwrap();
    println!("{:?}", array_a);
    drop(array_b);
    drop(context_b);
    println!("{:?}", array_a.get::<Value>(0));
}

fn sample_2() {
    let context = MiniV8::new();
    println!("{:?}", context.create_object());
    println!("{:?}", context.create_array());
    let mut s = format!("{}+1", 123);
    let sv8 = context.create_string(&s);
    s.clear();
    s.push_str("YIKES");
    println!("{:?}", sv8.to_string());
    println!("{:?}", s);
    assert!(context.coerce_boolean(&Value::Object(context.create_object())));
    assert!(context.coerce_boolean(&Value::Array(context.create_array())));
    assert!(!context.coerce_boolean(&Value::String(context.create_string(""))));
    assert!(context.coerce_boolean(&Value::String(context.create_string("abc"))));
    assert!(!context.coerce_boolean(&Value::Boolean(false)));
    assert!(context.coerce_boolean(&Value::Boolean(true)));
    assert!(!context.coerce_boolean(&Value::Float(0.0)));
    assert!(context.coerce_boolean(&Value::Float(1.0)));
    assert!(!context.coerce_boolean(&Value::Float(std::f64::NAN)));
    assert!(!context.coerce_boolean(&Value::Int32(0)));
    assert!(context.coerce_boolean(&Value::Int32(0x7FFFFFFF)));
    assert!(context.coerce_boolean(&Value::Date(0.0)));
    assert!(!context.coerce_boolean(&Value::Undefined));
    assert!(!context.coerce_boolean(&Value::Null));
}

fn sample_1() {
    let context = MiniV8::new();
    let abc = context.eval("'abc'").unwrap();
    let def = context.eval("'def'").unwrap();
    println!("{:?}, {:?}", abc, def);
    println!("{}, {}", abc.as_string().unwrap().to_string(), def.as_string().unwrap().to_string());

    let array1 = context.eval("['ひらがな', 1, 1.2]").unwrap();
    let array2 = context.eval("['ひらがな', 1, 1.2]").unwrap();
    println!("{:?}, {:?}",
        array1.as_array().unwrap().get::<Value>(0).unwrap(),
        array2.as_array().unwrap().get::<Value>(0).unwrap());

    println!("-------------");

    let array = context.eval("['ひらがな', 1, 1.2]").unwrap();
    if let Value::Array(array) = array {
        let s = context.eval("'abcdef'");
        array.set(3, s.unwrap()).unwrap();
        println!("{:?}", array.get::<Value>(0));
        println!("{:?}", array.get::<Value>(3));
        println!("{}", array.get::<Value>(0).unwrap().as_string().unwrap().to_string());
        println!("{}", array.get::<Value>(3).unwrap().as_string().unwrap().to_string());
        println!("-------------");
        println!("{:?}, {:?}", array.get::<Value>(0).unwrap(), array.get::<Value>(3).unwrap());
        println!("{}, {}",
            array.get::<Value>(0).unwrap().as_string().unwrap().to_string(),
            array.get::<Value>(3).unwrap().as_string().unwrap().to_string());
    }

    println!("-------------");

    bench(|| {
        let _ = context.eval("for (var i = 0; i < 1000000; i++) x = 'abc'").unwrap();
    });

    println!("-------------");

    println!("{}", context.eval("[1, 2, 3, '4', []]").unwrap().as_array().unwrap().len());

    let garbage = context.eval("x = { toString: () => { throw 100 } }; x").unwrap();
    let object = context.eval("x = { a: 123, '1': 456 }; x").unwrap();
    object.as_object().unwrap().set(garbage.clone(), Value::Null).unwrap(); // Exception 100
    println!("{:?}", object.as_object().unwrap().get::<Value, Value>(garbage)); // Exception 100
}

fn bench<F: FnOnce()>(f: F) {
    use std::time::SystemTime;
    let before = SystemTime::now();
    f();
    let after = SystemTime::now();
    println!("{:?}", after.duration_since(before).unwrap());
}
