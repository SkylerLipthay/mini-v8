use crate::error::Result;
use crate::ffi;
use crate::mini_v8::MiniV8;
use crate::value::{Value, Values};
use std::any::Any;
use std::collections::BTreeMap;

pub(crate) struct Ref<'mv8> {
    pub(crate) mv8: &'mv8 MiniV8,
    pub(crate) value: ffi::PersistentValue,
}

impl<'mv8> Ref<'mv8> {
    pub(crate) unsafe fn new(mv8: &MiniV8, value: ffi::Value) -> Ref {
        Ref { mv8, value: value.inner.value }
    }

    pub(crate) fn from_persistent(mv8: &MiniV8, value: ffi::PersistentValue) -> Ref {
        Ref { mv8, value }
    }
}

impl<'mv8> Clone for Ref<'mv8> {
    fn clone(&self) -> Ref<'mv8> {
        let value = unsafe { ffi::mv8_value_clone(self.mv8.context, self.value) };
        Ref { mv8: self.mv8, value }
    }
}

impl<'mv8> Drop for Ref<'mv8> {
    fn drop(&mut self) {
        unsafe { ffi::mv8_value_drop(self.value); }
    }
}

pub(crate) type Callback<'mv8, 'a> =
    Box<dyn Fn(&'mv8 MiniV8, Value<'mv8>, Values<'mv8>) -> Result<'mv8, Value<'mv8>> + 'a>;

pub(crate) type AnyMap = BTreeMap<String, Box<dyn Any + 'static>>;
