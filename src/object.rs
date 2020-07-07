use crate::*;

/// Reference to a JavaScript object.
#[derive(Clone)]
pub struct Object<'mv8>(pub(crate) Ref<'mv8>);

impl<'mv8> Object<'mv8> {
    /// Get an object property value using the given key. Returns `Value::Undefined` if no property
    /// with the key exists.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn get<K: ToValue<'mv8>, V: FromValue<'mv8>>(&self, key: K) -> Result<'mv8, V> {
        let mv8 = self.0.mv8;
        let key_desc = value_to_desc(mv8, &key.to_value(mv8)?);
        let result = unsafe { mv8_object_get(mv8.interface, self.0.value_ptr, key_desc) };
        desc_to_result(mv8, result)?.into(mv8)
    }

    /// Sets an object property using the given key and value.
    ///
    /// Returns an error if `ToValue::to_value` fails for either the key or the value or if the key
    /// value could not be cast to a property key string.
    pub fn set<K: ToValue<'mv8>, V: ToValue<'mv8>>(&self, key: K, value: V) -> Result<'mv8, ()> {
        let mv8 = self.0.mv8;
        let key_desc = value_to_desc(mv8, &key.to_value(mv8)?);
        let value_desc = value_to_desc(mv8, &value.to_value(mv8)?);
        desc_to_result_noval(mv8, unsafe {
            mv8_object_set(mv8.interface, self.0.value_ptr, key_desc, value_desc)
        })
    }

    /// Removes the property associated with the given key from the object. This function does
    /// nothing if the property does not exist.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn remove<K: ToValue<'mv8>>(&self, key: K) -> Result<'mv8, ()> {
        let mv8 = self.0.mv8;
        let key_desc = value_to_desc(mv8, &key.to_value(mv8)?);
        let result = unsafe { mv8_object_remove(mv8.interface, self.0.value_ptr, key_desc) };
        desc_to_result_noval(mv8, result)
    }

    /// Returns `true` if the given key is a property of the object, `false` otherwise.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn has<K: ToValue<'mv8>>(&self, key: K) -> Result<'mv8, bool> {
        let mv8 = self.0.mv8;
        let key_desc = value_to_desc(mv8, &key.to_value(mv8)?);
        let result = unsafe { mv8_object_has(mv8.interface, self.0.value_ptr, key_desc) };
        let has_desc = desc_to_result_val(mv8, result)?;
        Ok(unsafe { has_desc.payload.byte } == 1)
    }
}
