use crate::*;
use rusty_v8 as v8;
use std::fmt;

/// Reference to a JavaScript array.
///
/// Attempts to interact with an instance after its parent `MiniV8` is dropped will result in a
/// panic.
pub struct Function {
    pub(crate) value: v8::Global<v8::Function>,
    mv8: MiniV8,
}

impl Function {
    /// Consumes the function and downgrades it to a JavaScript object. This is inexpensive, since a
    /// function *is* an object.
    pub fn into_object(self) -> Object {
        let object = self.mv8.scope(|scope| {
            let object: v8::Local<v8::Object> = v8::Local::new(scope, &self.value).into();
            v8::Global::<v8::Object>::new(scope, object)
        });

        Object::new(&self.mv8, object)
    }
}

impl Clone for Function {
    fn clone(&self) -> Function {
        Function { value: self.value.clone(), mv8: self.mv8.weak() }
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<function>")
    }
}
