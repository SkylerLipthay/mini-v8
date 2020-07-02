use crate::*;

#[test]
#[should_panic]
fn prevent_use_after_drop() {
    let mv8 = MiniV8::new();
    let string = mv8.create_string("test");
    drop(mv8);
    string.to_string();
}

#[test]
#[should_panic]
fn prevent_value_cross_contamination() {
    let mv8_1 = MiniV8::new();
    let str_1 = mv8_1.create_string("123");
    let mv8_2 = MiniV8::new();
    let _str_2 = mv8_2.create_string("456");
    let _ = mv8_2.coerce_number(Value::String(str_1));
}
