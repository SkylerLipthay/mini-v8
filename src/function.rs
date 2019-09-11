use crate::object::Object;
use crate::types::Ref;

/// Reference to a JavaScript function.
#[derive(Clone, Debug)]
pub struct Function<'mv8>(pub(crate) Ref<'mv8>);

impl<'mv8> Function<'mv8> {
    /// Consumes the function and downgrades it to a JavaScript object. This is inexpensive, since
    /// an array *is* an object.
    pub fn into_object(self) -> Object<'mv8> {
        Object(self.0)
    }
}
