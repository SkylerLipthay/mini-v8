#![allow(dead_code)]

extern crate mini_v8;

use mini_v8::{MiniV8, Value, Invocation};

fn main() {
    // sample_1();
    // sample_2();
    // sample_3();
    // sample_4();
    // sample_5();
    // sample_6();
    sample_7();
}

fn sample_7() {
    fn add(inv: Invocation) -> mini_v8::Result<usize> {
        let (a, b): (usize, usize) = inv.args.into(inv.mv8)?;
        return Ok(a + b);
    }

    let context = MiniV8::new();
    {
        // TODO: To avoid memory management of rust functions, maybe we remove support for
        // closures... Look at what v8-rs does and perhaps what rlua does. Finalizers suck.
        let func = context.create_function(add);
        println!("{:?}", func);
        println!("{:?}", func.call::<_, Value>((1, 2)));
    }
    drop(context);
    println!("YEH");
}

fn sample_6() {
    let context = MiniV8::new();
    let round = context.eval("Math.round").unwrap();
    let slice = context.eval("Array.prototype.slice").unwrap();
    let array = context.eval("['ひらがな', 1, 1.2]").unwrap();
    println!("{:?}", round.as_function().unwrap().call::<_, Value>((2.6,)));
    println!("{:?}", slice.as_function().unwrap().call_method::<_, _, Vec<String>>(array, (0, 2)).unwrap());
}

fn sample_5() {
    let context = MiniV8::new();
    let x = context.eval("x = {a:1}").unwrap();
    let y = context.eval("y = Object.create(x)").unwrap();
    println!("{}", x.as_object().unwrap().keys(false).len());
    println!("{}", y.as_object().unwrap().keys(false).len());
    println!("{}", x.as_object().unwrap().keys(true).len());
    println!("{}", y.as_object().unwrap().keys(true).len());
    println!("{}", y.as_object().unwrap().keys(true)
        .get::<Value>(0).unwrap().as_string().unwrap().to_string());
}

fn sample_4() {
    let context = MiniV8::new();
    let object = context.create_object();
    println!("get {:?}", object.get::<Value, Value>(Value::Number(1.0)));
    println!("contains_key {:?}", object.contains_key(Value::Number(1.0)));
    println!("set {:?}", object.set(Value::Number(1.0), Value::Number(2.0)));
    println!("get {:?}", object.get::<Value, Value>(Value::Number(1.0)));
    println!("contains_key {:?}", object.contains_key(Value::Number(1.0)));
    println!("remove {:?}", object.remove(Value::Number(1.0)));
    println!("get {:?}", object.get::<Value, Value>(Value::Number(1.0)));
    println!("contains_key {:?}", object.contains_key(Value::Number(1.0)));
}

fn sample_3() {
    let context = MiniV8::new();
    println!("{:?}", context.coerce_number(context.eval("123").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("'123'").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("NaN").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("Infinity").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("-Infinity").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("[]").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("(function() {})").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("({})").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("undefined").unwrap()));
    println!("{:?}", context.coerce_number(context.eval("null").unwrap()));

    println!("{}", context.coerce_string(context.eval("null").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("undefined").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("(()=>{})").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("({})").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("([])").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("123.456").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("true").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("NaN").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("Infinity").unwrap()).unwrap().to_string());
    println!("{}", context.coerce_string(context.eval("-Infinity").unwrap()).unwrap().to_string());
    println!("{:?}", context.coerce_string(context.eval("({toString:()=>{throw 1;}})").unwrap()));
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
    assert!(context.coerce_boolean(Value::Object(context.create_object())));
    assert!(context.coerce_boolean(Value::Array(context.create_array())));
    assert!(!context.coerce_boolean(Value::String(context.create_string(""))));
    assert!(context.coerce_boolean(Value::String(context.create_string("abc"))));
    assert!(!context.coerce_boolean(Value::Boolean(false)));
    assert!(context.coerce_boolean(Value::Boolean(true)));
    assert!(!context.coerce_boolean(Value::Number(0.0)));
    assert!(context.coerce_boolean(Value::Number(1.0)));
    assert!(!context.coerce_boolean(Value::Number(std::f64::NAN)));
    assert!(context.coerce_boolean(Value::Date(0.0)));
    assert!(!context.coerce_boolean(Value::Undefined));
    assert!(!context.coerce_boolean(Value::Null));
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

    // let garbage = context.eval("x = { toString: () => { throw 100 } }; x").unwrap();
    // let object = context.eval("x = { a: 123, '1': 456 }; x").unwrap();
    // object.as_object().unwrap().set(garbage.clone(), Value::Null).unwrap(); // Exception 100
    // println!("{:?}", object.as_object().unwrap().get::<Value, Value>(garbage)); // Exception 100
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

fn bench<F: FnOnce()>(f: F) {
    use std::time::SystemTime;
    let before = SystemTime::now();
    f();
    let after = SystemTime::now();
    println!("{:?}", after.duration_since(before).unwrap());
}
