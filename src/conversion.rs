use crate::*;
use std::string::String as StdString;

impl<'mv8> ToValue<'mv8> for Value<'mv8> {
    fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(self)
    }
}

impl<'mv8> FromValue<'mv8> for Value<'mv8> {
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        Ok(value)
    }
}

impl<'mv8> ToValue<'mv8> for () {
    fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(Value::Undefined)
    }
}

impl<'mv8> FromValue<'mv8> for () {
    fn from_value(_value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        Ok(())
    }
}

impl<'mv8, T: ToValue<'mv8>> ToValue<'mv8> for Option<T> {
    fn to_value(self, mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        match self {
            Some(val) => val.to_value(mv8),
            None => Ok(Value::Null),
        }
    }
}

impl<'mv8, T: FromValue<'mv8>> FromValue<'mv8> for Option<T> {
    fn from_value(value: Value<'mv8>, mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        match value {
            Value::Null => Ok(None),
            value => Ok(Some(T::from_value(value, mv8)?)),
        }
    }
}

impl<'mv8> ToValue<'mv8> for String<'mv8> {
    fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(Value::String(self))
    }
}

impl<'mv8> FromValue<'mv8> for String<'mv8> {
    fn from_value(value: Value<'mv8>, mv8: &'mv8 MiniV8) -> Result<'mv8, String<'mv8>> {
        mv8.coerce_string(value)
    }
}

impl<'mv8> ToValue<'mv8> for Array<'mv8> {
    fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(Value::Array(self))
    }
}

impl<'mv8> FromValue<'mv8> for Array<'mv8> {
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Array<'mv8>> {
        match value {
            Value::Array(a) => Ok(a),
            value => Err(Error::from_js_conversion(value.type_name(), "Array")),
        }
    }
}

impl<'mv8> ToValue<'mv8> for Function<'mv8> {
    fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(Value::Function(self))
    }
}

impl<'mv8> FromValue<'mv8> for Function<'mv8> {
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Function<'mv8>> {
        match value {
            Value::Function(f) => Ok(f),
            value => Err(Error::from_js_conversion(value.type_name(), "Function")),
        }
    }
}

impl<'mv8> ToValue<'mv8> for Object<'mv8> {
    fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(Value::Object(self))
    }
}

impl<'mv8> FromValue<'mv8> for Object<'mv8> {
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Object<'mv8>> {
        match value {
            Value::Object(o) => Ok(o),
            value => Err(Error::from_js_conversion(value.type_name(), "Object")),
        }
    }
}

impl<'mv8> ToValue<'mv8> for bool {
    fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(Value::Boolean(self))
    }
}

impl<'mv8> FromValue<'mv8> for bool {
    fn from_value(value: Value, mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        Ok(mv8.coerce_boolean(value))
    }
}

impl<'mv8> ToValue<'mv8> for StdString {
    fn to_value(self, mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(Value::String(mv8.create_string(&self)))
    }
}

impl<'mv8> FromValue<'mv8> for StdString {
    fn from_value(value: Value<'mv8>, mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        Ok(mv8.coerce_string(value)?.to_string())
    }
}

impl<'mv8, 'a> ToValue<'mv8> for &'a str {
    fn to_value(self, mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(Value::String(mv8.create_string(self)))
    }
}

macro_rules! convert_number {
    ($prim_ty: ty) => {
        impl<'mv8> ToValue<'mv8> for $prim_ty {
            fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
                Ok(Value::Number(self as f64))
            }
        }

        impl<'mv8> FromValue<'mv8> for $prim_ty {
            fn from_value(value: Value<'mv8>, mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
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
