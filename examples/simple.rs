extern crate mini_v8;

use mini_v8::MiniV8;

fn main() {
    let mv8 = MiniV8::new();
    let result: String = mv8.eval("`Hello, World! 2 + 2 = ${2 + 2}`").unwrap();
    assert_eq!(result, "Hello, World! 2 + 2 = 4");
    println!("{}", result);
}
