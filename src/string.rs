use crate::ffi;
use crate::types::Ref;
use std::slice;
use std::string::String as StdString;

/// An immutable JavaScript string managed by V8.
#[derive(Clone, Debug)]
pub struct String<'mv8>(pub(crate) Ref<'mv8>);

impl<'mv8> String<'mv8> {
    /// Returns a Rust string converted from the V8 string.
    pub fn to_string(&self) -> StdString {
        unsafe {
            let utf8 = ffi::string_to_utf8_value(self.0.mv8.context, self.0.value);
            assert!(!utf8.data.is_null());
            let data = slice::from_raw_parts(utf8.data, utf8.length as usize).to_vec();
            let string = StdString::from_utf8_unchecked(data);
            ffi::utf8_value_drop(utf8);
            string
        }
    }
}
