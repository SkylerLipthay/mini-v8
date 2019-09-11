use crate::array::Array;
use crate::error::{Error, Result};
use crate::function::Function;
use crate::mini_v8::MiniV8;
use crate::object::Object;
use crate::string::String;
use crate::value::{FromValue, ToValue, Value};
use std::collections::{BTreeMap, HashMap, BTreeSet, HashSet};
use std::hash::{BuildHasher, Hash};
// use std::string::String as StdString;

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


impl<'mv8, K, V, S> ToValue<'mv8> for HashMap<K, V, S>
where
    K: Eq + Hash + ToValue<'mv8>,
    V: ToValue<'mv8>,
    S: BuildHasher,
{
    fn to_value(self, mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        let object = mv8.create_object();
        for (k, v) in self.into_iter() {
            object.set(k, v)?;
        }
        Ok(Value::Object(object))
    }
}

impl<'mv8, K, V, S> FromValue<'mv8> for HashMap<K, V, S>
where
    K: Eq + Hash + FromValue<'mv8>,
    V: FromValue<'mv8>,
    S: BuildHasher + Default,
{
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        match value {
            Value::Object(o) => o.properties(false).collect(),
            value => Err(Error::from_js_conversion(value.type_name(), "HashMap")),
        }
    }
}

impl<'mv8, K, V> ToValue<'mv8> for BTreeMap<K, V>
where
    K: Ord + ToValue<'mv8>,
    V: ToValue<'mv8>,
{
    fn to_value(self, mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        let object = mv8.create_object();
        for (k, v) in self.into_iter() {
            object.set(k, v)?;
        }
        Ok(Value::Object(object))
    }
}

impl<'mv8, K, V> FromValue<'mv8> for BTreeMap<K, V>
where
    K: Ord + FromValue<'mv8>,
    V: FromValue<'mv8>,
{
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        match value {
            Value::Object(o) => o.properties(false).collect(),
            value => Err(Error::from_js_conversion(value.type_name(), "BTreeMap")),
        }
    }
}

impl<'mv8, V: ToValue<'mv8>> ToValue<'mv8> for BTreeSet<V> {
    fn to_value(self, mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        let array = mv8.create_array();
        for v in self.into_iter() {
            array.push(v)?;
        }
        Ok(Value::Array(array))
    }
}

impl<'mv8, V: FromValue<'mv8> + Ord> FromValue<'mv8> for BTreeSet<V> {
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        match value {
            Value::Array(a) => a.elements().collect(),
            value => Err(Error::from_js_conversion(value.type_name(), "BTreeSet")),
        }
    }
}

impl<'mv8, V: ToValue<'mv8>> ToValue<'mv8> for HashSet<V> {
    fn to_value(self, mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        let array = mv8.create_array();
        for v in self.into_iter() {
            array.push(v)?;
        }
        Ok(Value::Array(array))
    }
}

impl<'mv8, V: FromValue<'mv8> + Hash + Eq> FromValue<'mv8> for HashSet<V> {
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        match value {
            Value::Array(a) => a.elements().collect(),
            value => Err(Error::from_js_conversion(value.type_name(), "HashSet")),
        }
    }
}

impl<'mv8, V: ToValue<'mv8>> ToValue<'mv8> for Vec<V> {
    fn to_value(self, mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        let array = mv8.create_array();
        for v in self.into_iter() {
            array.push(v)?;
        }
        Ok(Value::Array(array))
    }
}

impl<'mv8, V: FromValue<'mv8>> FromValue<'mv8> for Vec<V> {
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        match value {
            Value::Array(a) => a.elements().collect(),
            value => Err(Error::from_js_conversion(value.type_name(), "Vec")),
        }
    }
}

// TODO: Date conversion...
// let ts = unsafe { value.inner.float };
// let secs = ts / 1000.0;
// let nanos = ((secs - secs.floor()) * 1_000_000.0).round() as u32;
