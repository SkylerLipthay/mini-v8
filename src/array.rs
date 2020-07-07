use crate::*;

/// Reference to a JavaScript array.
#[derive(Clone)]
pub struct Array<'mv8>(pub(crate) Ref<'mv8>);

impl<'mv8> Array<'mv8> {
    /// Consumes the array and downgrades it to a JavaScript object. This is inexpensive, since an
    /// array *is* an object.
    pub fn into_object(self) -> Object<'mv8> {
        Object(self.0)
    }

    /// Returns the number of elements in the array.
    pub fn len(&self) -> u32 {
        unsafe { mv8_array_len(self.0.mv8.interface, self.0.value_ptr) }
    }
}
