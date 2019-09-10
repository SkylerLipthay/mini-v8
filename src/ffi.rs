use std::ffi::c_void;

pub(crate) type Context = *const c_void;
pub(crate) type PersistentValue = *const c_void;

#[allow(dead_code)]
#[repr(u32)]
#[derive(Copy, Clone)]
pub(crate) enum ValueTag {
    Null = 0,
    Undefined = 1,
    Int32 = 2,
    Float = 3,
    Boolean = 4,
    Array = 5,
    Function = 6,
    Date = 7,
    Object = 8,
    String = 9,
}

#[repr(C)]
pub(crate) union ValueInner {
    pub(crate) empty: u8,
    pub(crate) int32: i32,
    pub(crate) float: f64,
    pub(crate) boolean: u8,
    pub(crate) value: PersistentValue,
}

#[repr(C)]
pub(crate) struct Value {
    pub(crate) tag: ValueTag,
    pub(crate) inner: ValueInner,
}

impl Value {
    pub(crate) fn new(tag: ValueTag, inner: ValueInner) -> Value {
        Value { tag, inner }
    }
}

#[repr(C)]
pub(crate) struct EvalResult {
    pub(crate) exception: u8,
    pub(crate) value: Value,
}

#[repr(C)]
pub(crate) struct Utf8Value {
    pub(crate) data: *const u8,
    pub(crate) length: i32,
    src: *mut c_void,
}

extern "C" {
    pub(crate) fn context_new() -> Context;
    pub(crate) fn context_eval(ctx: Context, data: *const u8, length: usize) -> EvalResult;
    pub(crate) fn context_drop(ctx: Context);
    pub(crate) fn value_clone(ctx: Context, value: PersistentValue) -> PersistentValue;
    pub(crate) fn value_drop(value: PersistentValue);
    pub(crate) fn string_to_utf8_value(ctx: Context, value: PersistentValue) -> Utf8Value;
    pub(crate) fn utf8_value_drop(utf8_value: Utf8Value);
    pub(crate) fn array_length(ctx: Context, object: PersistentValue) -> u32;
    pub(crate) fn object_get_index(ctx: Context, object: PersistentValue, index: u32) -> Value;
    pub(crate) fn object_set_index(ctx: Context, object: PersistentValue, index: u32, value: Value);
}
