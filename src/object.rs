use crate::*;
use rusty_v8 as v8;
use std::fmt;
use std::marker::PhantomData;

/// Reference to a JavaScript object.
///
/// Attempts to interact with an instance after its parent `MiniV8` is dropped will result in a
/// panic.
pub struct Object {
    pub(crate) value: v8::Global<v8::Object>,
    mv8: MiniV8,
}

impl Object {
    /// Get an object property value using the given key. Returns `Value::Undefined` if no property
    /// with the key exists.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn get<K: ToValue, V: FromValue>(&self, key: K) -> Result<V> {
        let key = key.to_value(&self.mv8)?;
        self.mv8.try_catch_scope(|scope| {
            let native_key = key.to_native(scope);
            let object = self.value.get(scope);
            let maybe_value = object.get(scope, native_key);
            if let Some(exception) = scope.exception() {
                Err(Error::Value(Value::from_native(&self.mv8, scope, exception)))
            } else {
                Ok(match maybe_value {
                    Some(value) => Value::from_native(&self.mv8, scope, value),
                    None => Value::Undefined,
                })
            }
        }).and_then(|v| V::from_value(v, &self.mv8))
    }

    /// Sets an object property using the given key and value.
    ///
    /// Returns an error if `ToValue::to_value` fails for either the key or the value or if the key
    /// value could not be cast to a property key string.
    pub fn set<K: ToValue, V: ToValue>(&self, key: K, value: V) -> Result<()> {
        let key = key.to_value(&self.mv8)?;
        let value = value.to_value(&self.mv8)?;
        self.mv8.try_catch_scope(|scope| {
            let native_key = key.to_native(scope);
            let native_value = value.to_native(scope);
            let object = self.value.get(scope);
            object.set(scope, native_key, native_value);
            if let Some(exception) = scope.exception() {
                Err(Error::Value(Value::from_native(&self.mv8, scope, exception)))
            } else {
                Ok(())
            }
        })
    }

    /// Removes the property associated with the given key from the object. This function does
    /// nothing if the property does not exist.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn remove<K: ToValue>(&self, key: K) -> Result<()> {
        let key = key.to_value(&self.mv8)?;
        self.mv8.try_catch_scope(|scope| {
            let native_key = key.to_native(scope);
            let object = self.value.get(scope);
            object.delete(scope, native_key);
            if let Some(exception) = scope.exception() {
                Err(Error::Value(Value::from_native(&self.mv8, scope, exception)))
            } else {
                Ok(())
            }
        })
    }

    /// Returns `true` if the given key is a property of the object, `false` otherwise.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn has<K: ToValue>(&self, key: K) -> Result<bool> {
        let key = key.to_value(&self.mv8)?;
        self.mv8.try_catch_scope(|scope| {
            let native_key = key.to_native(scope);
            let object = self.value.get(scope);
            let maybe_bool = object.has(scope, native_key);
            if let Some(exception) = scope.exception() {
                Err(Error::Value(Value::from_native(&self.mv8, scope, exception)))
            } else {
                Ok(maybe_bool.unwrap_or(false))
            }
        })
    }

    /// Returns an array containing all of this object's enumerable property keys. If
    /// `include_inherited` is `false`, then only the object's own enumerable properties will be
    /// collected (similar to `Object.getOwnPropertyNames`in Javascript). If `include_inherited` is
    /// `true`, then the object's own properties and the enumerable properties from its prototype
    /// chain will be collected.
    pub fn keys(&self, include_inherited: bool) -> Result<Array> {
        self.mv8.try_catch_scope(|scope| {
            let object = self.value.get(scope);
            let maybe_array = match include_inherited {
                true => object.get_property_names(scope),
                false => object.get_own_property_names(scope),
            };
            if let Some(exception) = scope.exception() {
                Err(Error::Value(Value::from_native(&self.mv8, scope, exception)))
            } else {
                let array = maybe_array.unwrap_or_else(|| v8::Array::new(scope, 0));
                Ok(v8::Global::<v8::Array>::new(scope, array))
            }
        }).map(|a| Array::new(&self.mv8, a))
    }

    /// Converts the object into an iterator over the object's keys and values, acting like a
    /// `for-in` loop.
    ///
    /// For information on the `include_inherited` argument, see `Object::keys`.
    pub fn properties<K, V>(self, include_inherited: bool) -> Result<Properties<K, V>>
    where
        K: FromValue,
        V: FromValue,
    {
        let keys = self.keys(include_inherited)?;
        Ok(Properties { object: self, keys, index: 0, _phantom: PhantomData })
    }

    pub(crate) fn new(mv8: &MiniV8, value: v8::Global<v8::Object>) -> Object {
        Object { value, mv8: mv8.weak() }
    }
}

impl Clone for Object {
    fn clone(&self) -> Object {
        Object { value: self.value.clone(), mv8: self.mv8.weak() }
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let keys = match self.keys(false) {
            Ok(keys) => keys,
            Err(_) => return write!(f, "{{}}"),
        };

        let len = keys.len();
        if len == 0 {
            return write!(f, "{{}}");
        }

        write!(f, "{{ ")?;
        for i in 0..len {
            if let Ok(k) = keys.get::<Value>(i).and_then(|k| self.mv8.coerce_string(k)) {
                write!(f, "{:?}: ", k)?;
                match self.get::<_, Value>(k) {
                    Ok(v) => write!(f, "{:?}", v)?,
                    Err(_) => write!(f, "?")?,
                };
            } else {
                write!(f, "?")?;
            }
            if i + 1 < len {
                write!(f, ", ")?;
            }
        }
        write!(f, " }}")
    }
}

/// An iterator over an object's keys and values, acting like a `for-in` loop.
pub struct Properties<K, V> {
    object: Object,
    keys: Array,
    index: u32,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> Iterator for Properties<K, V>
where
    K: FromValue,
    V: FromValue,
{
    type Item = Result<(K, V)>;

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

        let key = match key.into(&self.object.mv8) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };

        Some(Ok((key, value)))
    }
}
