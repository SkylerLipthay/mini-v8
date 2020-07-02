use crate::*;
use rusty_v8 as v8;
use std::fmt;
use std::marker::PhantomData;

/// Reference to a JavaScript array.
///
/// Attempts to interact with an instance after its parent `MiniV8` is dropped will result in a
/// panic.
pub struct Array {
    pub(crate) value: v8::Global<v8::Array>,
    mv8: MiniV8,
}

impl Array {
    /// Consumes the array and downgrades it to a JavaScript object. This is inexpensive, since an
    /// array *is* an object.
    pub fn into_object(self) -> Object {
        let object = self.mv8.scope(|scope| {
            let object: v8::Local<v8::Object> = v8::Local::new(scope, &self.value).into();
            v8::Global::<v8::Object>::new(scope, object)
        });

        Object::new(&self.mv8, object)
    }

    /// Get the value using the given array index. Returns `Value::Undefined` if no element at the
    /// index exists.
    ///
    /// Returns an error if `FromValue::from_value` fails for the element.
    pub fn get<V: FromValue>(&self, index: u32) -> Result<V> {
        self.mv8.try_catch_scope(|scope| {
            let array = self.value.get(scope);
            let maybe_value = array.get_index(scope, index);
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

    /// Sets an array element using the given index and value.
    ///
    /// Returns an error if `ToValue::to_value` fails for the value.
    pub fn set<V: ToValue>(&self, index: u32, value: V) -> Result<()> {
        let value = value.to_value(&self.mv8)?;
        self.mv8.try_catch_scope(|scope| {
            let native_value = value.to_native(scope);
            let array = self.value.get(scope);
            array.set_index(scope, index, native_value);
            if let Some(exception) = scope.exception() {
                Err(Error::Value(Value::from_native(&self.mv8, scope, exception)))
            } else {
                Ok(())
            }
        })
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> u32 {
        self.mv8.scope(|scope| self.value.get(scope).length())
    }

    /// Pushes an element to the end of the array. This is a shortcut for `set` using `len` as the
    /// index.
    pub fn push<V: ToValue>(&self, value: V) -> Result<()> {
        self.set(self.len(), value)
    }

    /// Returns an iterator over the array's indexable values.
    pub fn elements<V: FromValue>(self) -> Elements<V> {
        Elements { array: self, index: 0, len: None, _phantom: PhantomData }
    }

    pub(crate) fn new(mv8: &MiniV8, value: v8::Global<v8::Array>) -> Array {
        Array { value, mv8: mv8.weak() }
    }
}

impl Clone for Array {
    fn clone(&self) -> Array {
        Array { value: self.value.clone(), mv8: self.mv8.weak() }
    }
}

impl fmt::Debug for Array {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let len = self.len();
        write!(f, "[")?;
        for i in 0..len {
            match self.get::<Value>(i) {
                Ok(v) => write!(f, "{:?}", v)?,
                Err(_) => write!(f, "?")?,
            };
            if i + 1 < len {
                write!(f, ", ")?;
            }
        }
        write!(f, "]")
    }
}

pub struct Elements<V> {
    array: Array,
    index: u32,
    len: Option<u32>,
    _phantom: PhantomData<V>,
}

impl<V: FromValue> Iterator for Elements<V> {
    type Item = Result<V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len.is_none() {
            self.len = Some(self.array.len());
        }

        if self.index >= self.len.unwrap() {
            return None;
        }

        let result = self.array.get(self.index);
        self.index += 1;
        Some(result)
    }
}
