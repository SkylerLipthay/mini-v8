use crate::*;

#[test]
fn js_function() {
    let mv8 = MiniV8::new();
    let func: Value = mv8.eval("(function(x, y) { return (isNaN(this) ? 0 : this) + x + y; })")
        .unwrap();
    assert!(func.is_function());
    let func = if let Value::Function(f) = func { f } else { unreachable!(); };
    let value: f64 = func.call((1, 2)).unwrap();
    assert_eq!(3.0f64, value);
    let value: f64 = func.call_method(3, (1, 2)).unwrap();
    assert_eq!(6.0f64, value);
}

#[test]
fn rust_function() {
    fn add(inv: Invocation) -> Result<usize> {
        let (a, b): (usize, usize) = inv.args.into(inv.mv8)?;
        return Ok(a + b);
    }

    let mv8 = MiniV8::new();
    let func = mv8.create_function(add);
    let value: f64 = func.call((1, 2)).unwrap();
    assert_eq!(3.0f64, value);

    mv8.global().set("add", func).unwrap();
    let value: f64 = mv8.eval("add(4, 5)").unwrap();
    assert_eq!(9.0f64, value);
}
