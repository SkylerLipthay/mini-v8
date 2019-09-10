extern crate mini_v8;

use mini_v8::{MiniV8, Value};

fn main() {
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
}

fn bench<F: FnOnce()>(f: F) {
    use std::time::SystemTime;
    let before = SystemTime::now();
    f();
    let after = SystemTime::now();
    println!("{:?}", after.duration_since(before).unwrap());
}
