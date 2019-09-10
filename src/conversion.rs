use crate::error::Result;
use crate::mini_v8::MiniV8;
use crate::value::{FromValue, ToValue, Value};

impl<'mv8> ToValue<'mv8> for Value<'mv8> {
    fn to_value(self, _mv8: &'mv8 MiniV8) -> Result<'mv8, Value<'mv8>> {
        Ok(self)
    }
}

impl<'mv8> FromValue<'mv8> for Value<'mv8> {
    fn from_value(value: Value<'mv8>, _mv8: &'mv8 MiniV8) -> Result<'mv8, Self> {
        Ok(value)
    }
}

// TODO: Date conversion...
// let ts = unsafe { value.inner.float };
// let secs = ts / 1000.0;
// let nanos = ((secs - secs.floor()) * 1_000_000.0).round() as u32;
