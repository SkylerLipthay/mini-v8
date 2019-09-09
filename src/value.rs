use crate::array::Array;
use crate::ffi;
use crate::function::Function;
use crate::mini_v8::MiniV8;
use crate::object::Object;
use crate::string::String;
use crate::types::Ref;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum Value<'mv8> {
    Undefined,
    Null,
    Boolean(bool),
    Int(i32),
    Float(f64),
    /// Elapsed duration since Unix epoch.
    Date(Duration),
    Array(Array<'mv8>),
    Function(Function<'mv8>),
    Object(Object<'mv8>),
    String(String<'mv8>),
}

pub(crate) fn from_ffi(mv8: &MiniV8, value: ffi::Value) -> Value {
    use ffi::ValueTag as VT;

    match value.tag {
        VT::Null => Value::Null,
        VT::Undefined => Value::Undefined,
        VT::Boolean => Value::Boolean(unsafe { value.inner.boolean != 0 }),
        VT::Int => Value::Int(unsafe { value.inner.int }),
        VT::Float => Value::Float(unsafe { value.inner.float }),
        VT::Date => {
            let ts = unsafe { value.inner.float };
            let secs = ts / 1000.0;
            let nanos = ((secs - secs.floor()) * 1_000_000.0).round() as u32;
            Value::Date(Duration::new(secs as u64, nanos))
        },
        VT::Array => Value::Array(Array(unsafe { Ref::new(mv8, value) })),
        VT::Function => Value::Function(Function(unsafe { Ref::new(mv8, value) })),
        VT::Object => Value::Object(Object(unsafe { Ref::new(mv8, value) })),
        VT::String => Value::String(String(unsafe { Ref::new(mv8, value) })),
    }
}
