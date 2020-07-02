use crate::*;


#[test]
fn option() {
    let mv8 = MiniV8::new();

    let none_val = None::<usize>.to_value(&mv8).unwrap();
    assert!(none_val.is_null());
    let num_val = Some(123).to_value(&mv8).unwrap();
    assert!(num_val.is_number());

    let none: Option<usize> = FromValue::from_value(none_val.clone(), &mv8).unwrap();
    assert_eq!(none, None::<usize>);
    let some_num: Option<usize> = FromValue::from_value(num_val.clone(), &mv8).unwrap();
    assert_eq!(some_num, Some(123));
    let num: usize = FromValue::from_value(num_val.clone(), &mv8).unwrap();
    assert_eq!(num, 123);
    let num_zero: usize = FromValue::from_value(none_val.clone(), &mv8).unwrap();
    assert_eq!(num_zero, 0);
}

#[test]
fn variadic() {
    let mv8 = MiniV8::new();
    let values = (1, 0, 1).to_values(&mv8).unwrap();

    let var: Variadic<u8> = FromValues::from_values(values.clone(), &mv8).unwrap();
    assert_eq!(*var, vec![1, 0, 1]);

    let values = (1, Variadic::from_vec(vec![0, 1])).to_values(&mv8).unwrap();
    let var: Variadic<u8> = FromValues::from_values(values.clone(), &mv8).unwrap();
    assert_eq!(*var, vec![1, 0, 1]);
}

#[test]
fn tuple() {
    let mv8 = MiniV8::new();
    let values = (1, 0, 1).to_values(&mv8).unwrap();

    let out: (u8, u8, u8) = FromValues::from_values(values.clone(), &mv8).unwrap();
    assert_eq!((1, 0, 1), out);

    let out: (u8, u8) = FromValues::from_values(values.clone(), &mv8).unwrap();
    assert_eq!((1, 0), out);

    type Overflow = (u8, u8, u8, Value, Value);
    let (a, b, c, d, e): Overflow = FromValues::from_values(values.clone(), &mv8).unwrap();
    assert_eq!((1, 0, 1), (a, b, c));
    assert!(d.is_undefined());
    assert!(e.is_undefined());

    type VariadicTuple = (u8, Variadic<u8>);
    let (a, var): VariadicTuple = FromValues::from_values(values.clone(), &mv8).unwrap();
    assert_eq!(1, a);
    assert_eq!(*var, vec![0, 1]);

    type VariadicOver = (u8, u8, u8, u8, Variadic<u8>);
    let (a, b, c, d, var): VariadicOver = FromValues::from_values(values.clone(), &mv8).unwrap();
    // `d` is `0` because `undefined` is coerced into `0`:
    assert_eq!((1, 0, 1, 0), (a, b, c, d));
    assert_eq!(*var, vec![]);
}
