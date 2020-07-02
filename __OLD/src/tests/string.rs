use crate::mini_v8::MiniV8;

#[test]
fn to_string() {
    let mv8 = MiniV8::new();
    assert_eq!(mv8.create_string("abc").to_string(), "abc".to_string());
}
