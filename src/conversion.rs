use crate::*;
use std::string::String as StdString;

impl ToValue for Value {
    fn to_value(self, _mv8: &MiniV8) -> Result<Value> {
        Ok(self)
    }
}

impl FromValue for Value {
    fn from_value(value: Value, _mv8: &MiniV8) -> Result<Self> {
        Ok(value)
    }
}

impl ToValue for () {
    fn to_value(self, _mv8: &MiniV8) -> Result<Value> {
        Ok(Value::Undefined)
    }
}

impl FromValue for () {
    fn from_value(_value: Value, _mv8: &MiniV8) -> Result<Self> {
        Ok(())
    }
}

impl<T: ToValue> ToValue for Option<T> {
    fn to_value(self, mv8: &MiniV8) -> Result<Value> {
        match self {
            Some(val) => val.to_value(mv8),
            None => Ok(Value::Null),
        }
    }
}

impl<T: FromValue> FromValue for Option<T> {
    fn from_value(value: Value, mv8: &MiniV8) -> Result<Self> {
        match value {
            Value::Null => Ok(None),
            value => Ok(Some(T::from_value(value, mv8)?)),
        }
    }
}

impl ToValue for String {
    fn to_value(self, _mv8: &MiniV8) -> Result<Value> {
        Ok(Value::String(self))
    }
}

impl FromValue for String {
    fn from_value(value: Value, mv8: &MiniV8) -> Result<String> {
        mv8.coerce_string(value)
    }
}

impl ToValue for Object {
    fn to_value(self, _mv8: &MiniV8) -> Result<Value> {
        Ok(Value::Object(self))
    }
}

impl FromValue for Object {
    fn from_value(value: Value, _mv8: &MiniV8) -> Result<Object> {
        match value {
            Value::Object(o) => Ok(o),
            value => Err(Error::from_js_conversion(value.type_name(), "Object")),
        }
    }
}

impl ToValue for StdString {
    fn to_value(self, mv8: &MiniV8) -> Result<Value> {
        Ok(Value::String(mv8.create_string(&self)))
    }
}

impl FromValue for StdString {
    fn from_value(value: Value, mv8: &MiniV8) -> Result<StdString> {
        Ok(mv8.coerce_string(value)?.to_string())
    }
}

impl<'a> ToValue for &'a str {
    fn to_value(self, mv8: &MiniV8) -> Result<Value> {
        Ok(Value::String(mv8.create_string(self)))
    }
}

macro_rules! convert_number {
    ($prim_ty: ty) => {
        impl ToValue for $prim_ty {
            fn to_value(self, _mv8: &MiniV8) -> Result<Value> {
                Ok(Value::Number(self as f64))
            }
        }

        impl FromValue for $prim_ty {
            fn from_value(value: Value, mv8: &MiniV8) -> Result<Self> {
                Ok(mv8.coerce_number(value)? as $prim_ty)
            }
        }
    }
}

convert_number!(i8);
convert_number!(u8);
convert_number!(i16);
convert_number!(u16);
convert_number!(i32);
convert_number!(u32);
convert_number!(i64);
convert_number!(u64);
convert_number!(isize);
convert_number!(usize);
convert_number!(f32);
convert_number!(f64);

impl ToValues for Values {
    fn to_values(self, _mv8: &MiniV8) -> Result<Values> {
        Ok(self)
    }
}

impl FromValues for Values {
    fn from_values(values: Values, _mv8: &MiniV8) -> Result<Self> {
        Ok(values)
    }
}

impl<T: ToValue> ToValues for Variadic<T> {
    fn to_values(self, mv8: &MiniV8) -> Result<Values> {
        self.0.into_iter().map(|value| value.to_value(mv8)).collect()
    }
}

impl<T: FromValue> FromValues for Variadic<T> {
    fn from_values(values: Values, mv8: &MiniV8) -> Result<Self> {
        values.into_iter()
            .map(|value| T::from_value(value, mv8))
            .collect::<Result<Vec<T>>>()
            .map(Variadic)
    }
}

impl ToValues for () {
    fn to_values(self, _mv8: &MiniV8) -> Result<Values> {
        Ok(Values::new())
    }
}

impl FromValues for () {
    fn from_values(_values: Values, _mv8: &MiniV8) -> Result<Self> {
        Ok(())
    }
}

macro_rules! impl_tuple {
    ($($name:ident),*) => (
        impl<$($name),*> ToValues for ($($name,)*)
        where
            $($name: ToValue,)*
        {
            #[allow(non_snake_case)]
            fn to_values(self, mv8: &MiniV8) -> Result<Values> {
                let ($($name,)*) = self;
                let reservation = $({ &$name; 1 } +)* 0;
                let mut results = Vec::with_capacity(reservation);
                $(results.push($name.to_value(mv8)?);)*
                Ok(Values::from_vec(results))
            }
        }

        impl<$($name),*> FromValues for ($($name,)*)
        where
            $($name: FromValue,)*
        {
            #[allow(non_snake_case, unused_mut, unused_variables)]
            fn from_values(values: Values, mv8: &MiniV8) -> Result<Self> {
                let mut iter = values.into_vec().into_iter();
                Ok(($({
                    let $name = ();
                    FromValue::from_value(iter.next().unwrap_or(Value::Undefined), mv8)?
                },)*))
            }
        }

        impl<$($name,)* VAR> ToValues for ($($name,)* Variadic<VAR>)
        where
            $($name: ToValue,)*
            VAR: ToValue,
        {
            #[allow(non_snake_case)]
            fn to_values(self, mv8: &MiniV8) -> Result<Values> {
                let ($($name,)* variadic) = self;
                let reservation = $({ &$name; 1 } +)* 1;
                let mut results = Vec::with_capacity(reservation);
                $(results.push($name.to_value(mv8)?);)*
                if results.is_empty() {
                    Ok(variadic.to_values(mv8)?)
                } else {
                    results.append(&mut variadic.to_values(mv8)?.into_vec());
                    Ok(Values::from_vec(results))
                }
            }
        }

        impl<$($name,)* VAR> FromValues for ($($name,)* Variadic<VAR>)
        where
            $($name: FromValue,)*
            VAR: FromValue,
        {
            #[allow(non_snake_case, unused_mut, unused_variables)]
            fn from_values(values: Values, mv8: &MiniV8) -> Result<Self> {
                let mut values = values.into_vec();
                let len = values.len();
                let split = $({ let $name = (); 1 } +)* 0;

                if len < split {
                    values.reserve(split - len);
                    for _ in len..split {
                        values.push(Value::Undefined);
                    }
                }

                let last_values = Values::from_vec(values.split_off(split));
                let variadic = FromValues::from_values(last_values, mv8)?;

                let mut iter = values.into_iter();
                let ($($name,)*) = ($({ let $name = (); iter.next().unwrap() },)*);

                Ok(($(FromValue::from_value($name, mv8)?,)* variadic))
            }
        }
    )
}

impl_tuple!(A);
impl_tuple!(A, B);
impl_tuple!(A, B, C);
impl_tuple!(A, B, C, D);
impl_tuple!(A, B, C, D, E);
impl_tuple!(A, B, C, D, E, F);
impl_tuple!(A, B, C, D, E, F, G);
impl_tuple!(A, B, C, D, E, F, G, H);
impl_tuple!(A, B, C, D, E, F, G, H, I);
impl_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
