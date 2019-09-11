use crate::array::Array;
use crate::error::Result;
use crate::ffi;
use crate::types::Ref;
use crate::value::{self, FromValue, ToValue, Value};
use std::marker::PhantomData;

/// Reference to a JavaScript object.
#[derive(Clone, Debug)]
pub struct Object<'mv8>(pub(crate) Ref<'mv8>);

impl<'mv8> Object<'mv8> {
    /// Get an object property value using the given key. Returns `Value::Undefined` if no property
    /// with the key exists.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
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
    /// value could not be cast to a property key string.
    pub fn set<K: ToValue<'mv8>, V: ToValue<'mv8>>(&self, key: K, value: V) -> Result<'mv8, ()> {
        let mv8 = self.0.mv8;
        let key = key.to_value(mv8)?;
        let ffi_key = value::to_ffi(mv8, &key);
        let value = value.to_value(mv8)?;
        let ffi_value = value::to_ffi(mv8, &value);
        let ffi_result = unsafe { ffi::object_set(mv8.context, self.0.value, ffi_key, ffi_value) };
        value::from_ffi_exception(mv8, ffi_result)
    }

    /// Removes the property associated with the given key from the object. This function does
    /// nothing if the property does not exist.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn remove<K: ToValue<'mv8>>(&self, key: K) -> Result<'mv8, ()> {
        let mv8 = self.0.mv8;
        let key = key.to_value(mv8)?;
        let ffi_key = value::to_ffi(mv8, &key);
        let ffi_result = unsafe { ffi::object_remove(mv8.context, self.0.value, ffi_key) };
        value::from_ffi_exception(mv8, ffi_result)
    }

    /// Returns `true` if the given key is a property of the object, `false` otherwise.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn contains_key<K: ToValue<'mv8>>(&self, key: K) -> Result<'mv8, bool> {
        let mv8 = self.0.mv8;
        let key = key.to_value(mv8)?;
        let ffi_key = value::to_ffi(mv8, &key);
        let ffi_result = unsafe { ffi::object_contains_key(mv8.context, self.0.value, ffi_key) };
        value::from_ffi_result(mv8, ffi_result).map(|value| mv8.coerce_boolean(value))
    }

    /// Returns an array containing all of this object's enumerable property keys. If
    /// `include_inherited` is `false`, then only the object's own enumerable properties will be
    /// collected (similar to `Object.getOwnPropertyNames` in Javascript). If `include_inherited` is
    /// `true`, then the object's own properties and the enumerable properties from its prototype
    /// chain will be collected.
    pub fn keys(&self, include_inherited: bool) -> Array<'mv8> {
        let mv8 = self.0.mv8;
        Array(Ref::from_persistent(mv8, unsafe {
            ffi::object_keys(mv8.context, self.0.value, if include_inherited { 1 } else { 0 })
        }))
    }

    /// Converts the object into an iterator over the object's keys and values, acting like a
    /// `for-in` loop.
    ///
    /// For information on the `include_inherited` argument, see `Object::keys`.
    pub fn properties<K, V>(self, include_inherited: bool) -> Properties<'mv8, K, V>
    where
        K: FromValue<'mv8>,
        V: FromValue<'mv8>,
    {
        let keys = self.keys(include_inherited);
        Properties { object: self, keys, index: 0, _phantom: PhantomData }
    }
}

/// An iterator over an object's keys and values, acting like a `for-in` loop.
pub struct Properties<'mv8, K, V> {
    object: Object<'mv8>,
    keys: Array<'mv8>,
    index: u32,
    _phantom: PhantomData<(K, V)>,
}

impl<'mv8, K, V> Iterator for Properties<'mv8, K, V>
where
    K: FromValue<'mv8>,
    V: FromValue<'mv8>,
{
    type Item = Result<'mv8, (K, V)>;

    /// This will return `Some(Err(...))` if the next property's key or value failed to be converted
    /// into `K` or `V` respectively (through `ToValue`).
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.keys.len() {
            return None;
        }

        let key = self.keys.get::<Value>(self.index);
        self.index += 1;

        let key = match key {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let value = match self.object.get::<_, V>(key.clone()) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        let key = match key.into(self.object.0.mv8) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        Some(Ok((key, value)))
    }
}
