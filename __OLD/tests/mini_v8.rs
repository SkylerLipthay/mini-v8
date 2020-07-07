use crate::mini_v8::MiniV8;
use crate::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn wasm() {
    assert_eq!(7, 7);
}

#[test]
#[should_panic]
fn value_cross_contamination() {
    let mv8_1 = MiniV8::new();
    let str_1 = mv8_1.create_string("123");
    let mv8_2 = MiniV8::new();
    let _str_2 = mv8_2.create_string("456");
    let _ = mv8_2.coerce_number(Value::String(str_1));
}

#[test]
fn user_data_drop() {
    let mut mv8 = MiniV8::new();
    let (count, data) = make_test_user_data();
    mv8.set_user_data("data", data);
    drop(mv8);
    assert_eq!(*count.borrow(), 1000);
}

#[test]
fn user_data_get() {
    let mut mv8 = MiniV8::new();
    let (_, data) = make_test_user_data();
    mv8.set_user_data("data", data);
    assert!(mv8.get_user_data::<TestUserData>("no-exist").is_none());
    assert!(mv8.get_user_data::<usize>("data").is_none());

    {
        let data = mv8.get_user_data::<TestUserData>("data").unwrap();
        assert_eq!(data.get(), 0);
        data.increase();
        assert_eq!(data.get(), 1);
    }
}

#[test]
fn user_data_remove() {
    let mut mv8 = MiniV8::new();
    let (count, data) = make_test_user_data();
    mv8.set_user_data("data", data);
    assert_eq!(*count.borrow(), 0);
    let data = mv8.remove_user_data("data").unwrap();
    assert_eq!(*count.borrow(), 0);
    data.downcast_ref::<TestUserData>().unwrap().increase();
    assert_eq!(*count.borrow(), 1);
    drop(data);
    assert_eq!(*count.borrow(), 1000);
}

struct TestUserData {
    count: Rc<RefCell<usize>>,
}

impl TestUserData {
    fn increase(&self) {
        *self.count.borrow_mut() += 1;
    }

    fn get(&self) -> usize {
        *self.count.borrow()
    }
}

impl Drop for TestUserData {
    fn drop(&mut self) {
        *self.count.borrow_mut() = 1000;
    }
}

fn make_test_user_data() -> (Rc<RefCell<usize>>, TestUserData) {
    let count = Rc::new(RefCell::new(0));
    (count.clone(), TestUserData { count })
}
