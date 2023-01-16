use crate::*;
use std::fmt;
use std::marker::PhantomData;

#[derive(Clone)]
pub struct Object {
    pub(crate) mv8: MiniV8,
    pub(crate) handle: v8::Global<v8::Object>,
}

impl Object {
    /// Get an object property value using the given key. Returns `Value::Undefined` if no property
    /// with the key exists.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn get<K: ToValue, V: FromValue>(&self, key: K) -> Result<V> {
        let key = key.to_value(&self.mv8)?;
        self.mv8.try_catch(|scope| {
            let object = v8::Local::new(scope, self.handle.clone());
            let key = key.to_v8_value(scope);
            let result = object.get(scope, key);
            self.mv8.exception(scope)?;
            Ok(Value::from_v8_value(&self.mv8, scope, result.unwrap()))
        }).and_then(|v| v.into(&self.mv8))
    }

    /// Sets an object property using the given key and value.
    ///
    /// Returns an error if `ToValue::to_value` fails for either the key or the value or if the key
    /// value could not be cast to a property key string.
    pub fn set<K: ToValue, V: ToValue>(&self, key: K, value: V) -> Result<()> {
        let key = key.to_value(&self.mv8)?;
        let value = value.to_value(&self.mv8)?;
        self.mv8.try_catch(|scope| {
            let object = v8::Local::new(scope, self.handle.clone());
            let key = key.to_v8_value(scope);
            let value = value.to_v8_value(scope);
            object.set(scope, key, value);
            self.mv8.exception(scope)
        })
    }

    /// Removes the property associated with the given key from the object. This function does
    /// nothing if the property does not exist.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn remove<K: ToValue>(&self, key: K) -> Result<()> {
        let key = key.to_value(&self.mv8)?;
        self.mv8.try_catch(|scope| {
            let object = v8::Local::new(scope, self.handle.clone());
            let key = key.to_v8_value(scope);
            object.delete(scope, key);
            self.mv8.exception(scope)
        })
    }

    /// Returns `true` if the given key is a property of the object, `false` otherwise.
    ///
    /// Returns an error if `ToValue::to_value` fails for the key or if the key value could not be
    /// cast to a property key string.
    pub fn has<K: ToValue>(&self, key: K) -> Result<bool> {
        let key = key.to_value(&self.mv8)?;
        self.mv8.try_catch(|scope| {
            let object = v8::Local::new(scope, self.handle.clone());
            let key = key.to_v8_value(scope);
            let has = object.has(scope, key);
            self.mv8.exception(scope)?;
            Ok(has.unwrap())
        })
    }

    /// Calls the function at the key with the given arguments, with `this` set to the object.
    /// Returns an error if the value at the key is not a function.
    pub fn call_prop<K, A, R>(&self, key: K, args: A) -> Result<R>
    where
        K: ToValue,
        A: ToValues,
        R: FromValue,
    {
        let func: Function = self.get(key)?;
        func.call_method(self.clone(), args)
    }

    /// Returns an array containing all of this object's enumerable property keys. If
    /// `include_inherited` is `false`, then only the object's own enumerable properties will be
    /// collected (similar to `Object.getOwnPropertyNames` in Javascript). If `include_inherited` is
    /// `true`, then the object's own properties and the enumerable properties from its prototype
    /// chain will be collected.
    pub fn keys(&self, include_inherited: bool) -> Result<Array> {
        self.mv8.try_catch(|scope| {
            let object = v8::Local::new(scope, self.handle.clone());
            let keys = if include_inherited {
                object.get_property_names(scope, Default::default())
            } else {
                object.get_own_property_names(scope, Default::default())
            };
            self.mv8.exception(scope)?;
            Ok(Array {
                mv8: self.mv8.clone(),
                handle: v8::Global::new(scope, keys.unwrap()),
            })
        })
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
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let keys = match self.keys(false) {
            Ok(keys) => keys,
            Err(_) => return write!(f, "<object with keys exception>"),
        };

        let len = keys.len();
        if len == 0 {
            return write!(f, "{{}}");
        }

        write!(f, "{{ ")?;
        for i in 0..len {
            if let Ok(k) = keys.get::<Value>(i).and_then(|k| k.coerce_string(&self.mv8)) {
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
