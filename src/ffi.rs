use crate::function::{callback_drop, callback_wrapper};
use std::ffi::c_void;
use std::sync::Once;

pub(crate) type Context = *const c_void;
pub(crate) type PersistentValue = *const c_void;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Copy, Clone)]
pub(crate) enum ValueTag {
    Null = 0,
    Undefined = 1,
    Number = 2,
    Boolean = 3,
    Array = 4,
    Function = 5,
    Date = 6,
    Object = 7,
    String = 8,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub(crate) union ValueInner {
    pub(crate) empty: u8,
    pub(crate) number: f64,
    pub(crate) boolean: u8,
    pub(crate) value: PersistentValue,
}

#[derive(Copy, Clone)]
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
    src: *const c_void,
}

pub(crate) fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        unsafe { mv8_init_set_callback_lifecycle_funcs(callback_wrapper as _, callback_drop as _) }
    });
}

extern "C" {
    pub(crate) fn mv8_init_set_callback_lifecycle_funcs(
        wrapper_func: *const c_void,
        drop_func: *const c_void,
    );
    pub(crate) fn mv8_context_new() -> Context;
    pub(crate) fn mv8_context_eval(ctx: Context, data: *const u8, length: usize) -> EvalResult;
    pub(crate) fn mv8_context_drop(ctx: Context);
    pub(crate) fn mv8_context_global(ctx: Context) -> PersistentValue;
    pub(crate) fn mv8_context_set_data(ctx: Context, slot: u32, data: *mut c_void);
    pub(crate) fn mv8_context_get_data(ctx: Context, slot: u32) -> *mut c_void;
    pub(crate) fn mv8_value_clone(ctx: Context, value: PersistentValue) -> PersistentValue;
    pub(crate) fn mv8_value_drop(value: PersistentValue);
    pub(crate) fn mv8_string_create(ctx: Context, data: *const u8, length: usize)
        -> PersistentValue;
    pub(crate) fn mv8_string_to_utf8_value(ctx: Context, value: PersistentValue) -> Utf8Value;
    pub(crate) fn mv8_utf8_value_drop(utf8_value: Utf8Value);
    pub(crate) fn mv8_array_length(ctx: Context, object: PersistentValue) -> u32;
    pub(crate) fn mv8_array_create(ctx: Context) -> PersistentValue;
    pub(crate) fn mv8_object_create(ctx: Context) -> PersistentValue;
    pub(crate) fn mv8_object_get(ctx: Context, object: PersistentValue, key: Value) -> EvalResult;
    pub(crate) fn mv8_object_set(ctx: Context, object: PersistentValue, key: Value, value: Value)
        -> EvalResult;
    pub(crate) fn mv8_object_remove(ctx: Context, object: PersistentValue, key: Value)
        -> EvalResult;
    pub(crate) fn mv8_object_contains_key(ctx: Context, object: PersistentValue, key: Value)
        -> EvalResult;
    pub(crate) fn mv8_object_keys(ctx: Context, object: PersistentValue, include_inherited: u8)
        -> PersistentValue;
    pub(crate) fn mv8_object_get_index(ctx: Context, object: PersistentValue, index: u32) -> Value;
    pub(crate) fn mv8_object_set_index(
        ctx: Context,
        object: PersistentValue,
        index: u32,
        value: Value,
    );
    pub(crate) fn mv8_coerce_boolean(ctx: Context, value: Value) -> u8;
    pub(crate) fn mv8_coerce_number(ctx: Context, value: Value) -> EvalResult;
    pub(crate) fn mv8_coerce_string(ctx: Context, value: Value) -> EvalResult;
    pub(crate) fn mv8_function_call(
        ctx: Context,
        function: PersistentValue,
        this: Value,
        args: *const Value,
        num_args: i32,
    ) -> EvalResult;
    pub(crate) fn mv8_function_create(ctx: Context, callback: *mut c_void) -> PersistentValue;
}
