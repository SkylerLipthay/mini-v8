use crate::error::Result;
use crate::ffi;
use crate::object::Object;
use crate::types::Ref;
use crate::value::{self, FromValue, ToValue};
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct Array<'mv8>(pub(crate) Ref<'mv8>);

impl<'mv8> Array<'mv8> {
    /// Consumes the array and downgrades it to a JavaScript object. This is inexpensive, since an
    /// array *is* an object.
    pub fn into_object(self) -> Object<'mv8> {
        Object(self.0)
    }

    /// Get the value using the given array index. Returns `Value::Undefined` if no element at the
    /// index exists.
    ///
    /// Returns an error if `FromValue::from_value` fails for the element.
    pub fn get<V: FromValue<'mv8>>(&self, index: u32) -> Result<'mv8, V> {
        let mv8 = self.0.mv8;
        let ffi_value = unsafe { ffi::object_get_index(mv8.context, self.0.value, index) };
        let value = value::from_ffi(mv8, ffi_value);
        V::from_value(value, mv8)
    }

    /// Sets an array element using the given index and value.
    ///
    /// Returns an error if `ToValue::to_value` fails for the value.
    pub fn set<V: ToValue<'mv8>>(&self, index: u32, value: V) -> Result<'mv8, ()> {
        let mv8 = self.0.mv8;
        let value = value.to_value(mv8)?;
        let ffi_value = value::to_ffi(mv8, &value);
        unsafe { ffi::object_set_index(mv8.context, self.0.value, index, ffi_value); }
        Ok(())
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> u32 {
        let mv8 = self.0.mv8;
        unsafe { ffi::array_length(mv8.context, self.0.value) }
    }

    /// Pushes an element to the end of the array. This is a shortcut for `set` using `len` as the
    /// index.
    pub fn push<V: ToValue<'mv8>>(&self, value: V) -> Result<'mv8, ()> {
        self.set(self.len(), value)
    }

    /// Returns an iterator over the array's indexable values.
    pub fn elements<V: FromValue<'mv8>>(self) -> Elements<'mv8, V> {
        Elements {
            array: self,
            index: 0,
            len: None,
            _phantom: PhantomData,
        }
    }
}

pub struct Elements<'mv8, V> {
    array: Array<'mv8>,
    index: u32,
    len: Option<u32>,
    _phantom: PhantomData<V>,
}

impl<'mv8, V: FromValue<'mv8>> Iterator for Elements<'mv8, V> {
    type Item = Result<'mv8, V>;

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