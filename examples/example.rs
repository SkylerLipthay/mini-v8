extern crate mini_v8;

fn main() {
    let context = mini_v8::MiniV8::new();
    let value = context.eval("'ひらがな'");
    println!("{:?}", value);
    println!("{:?}", value.clone());
    match value {
        Ok(mini_v8::Value::String(string)) | Err(mini_v8::Value::String(string)) => {
            println!("It's a string! Here it is:");
            println!("{}", string.to_string());
        }
        _ => (),
    }
}
