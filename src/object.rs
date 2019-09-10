use crate::error::Result;
use crate::ffi;
use crate::types::Ref;
use crate::value::{self, FromValue, ToValue};

#[derive(Clone, Debug)]
pub struct Object<'mv8>(pub(crate) Ref<'mv8>);

impl<'mv8> Object<'mv8> {
    /// Get an object property value using the given key. Returns `Value::Undefined` if no property
    /// with the key exists.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key could not be cast to
    /// a string.
    pub fn get<K: ToValue<'mv8>, V: FromValue<'mv8>>(&self, key: K) -> Result<'mv8, V> {
        let mv8 = self.0.mv8;
        let key = key.to_value(mv8)?;
        let ffi_key = value::to_ffi(mv8, &key);
        let ffi_result = unsafe { ffi::object_get(mv8.context, self.0.value, ffi_key) };
        value::from_ffi_result(mv8, ffi_result).and_then(|value| V::from_value(value, mv8))
    }

    /// Sets an object property using the given key and value.
    ///
    /// Returns an error if `ToValue::to_value` fails for either the key or the value or if the key
    /// could not be cast to a string.
    pub fn set<K: ToValue<'mv8>, V: ToValue<'mv8>>(&self, key: K, value: V) -> Result<'mv8, ()> {
        let mv8 = self.0.mv8;
        let key = key.to_value(mv8)?;
        let ffi_key = value::to_ffi(mv8, &key);
        let value = value.to_value(mv8)?;
        let ffi_value = value::to_ffi(mv8, &value);
        let ffi_result = unsafe { ffi::object_set(mv8.context, self.0.value, ffi_key, ffi_value) };
        value::from_ffi_exception(mv8, ffi_result)
    }
}
