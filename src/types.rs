use crate::ffi;
use crate::MiniV8;
use std::fmt;

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

impl<'mv8> fmt::Debug for Ref<'mv8> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ref({:p})", self.value)
    }
}

impl<'mv8> Clone for Ref<'mv8> {
    fn clone(&self) -> Ref<'mv8> {
        let value = unsafe { ffi::value_clone(self.mv8.context, self.value) };
        Ref { mv8: self.mv8, value }
    }
}

impl<'mv8> Drop for Ref<'mv8> {
    fn drop(&mut self) {
        unsafe { ffi::value_drop(self.value); }
    }
}
