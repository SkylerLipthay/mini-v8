use crate::*;
use rusty_v8 as v8;
use std::convert::TryInto;
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::{fmt, slice, vec};

/// A JavaScript value.
///
/// `Value`s can either hold direct values (undefined, null, booleans, numbers, dates) or references
/// (strings, arrays, functions, other objects). Cloning values (via Rust's `Clone`) of the direct
/// types defers to Rust's `Copy`, while cloning values of the referential types results in a simple
/// reference clone similar to JavaScript's own "by-reference" semantics.
#[derive(Clone)]
pub enum Value {
    /// The JavaScript value `undefined`.
    Undefined,
    /// The JavaScript value `null`.
    Null,
    /// The JavaScript value `true` or `false`.
    Boolean(bool),
    /// A JavaScript floating point number.
    Number(f64),
    /// An immutable JavaScript string, managed by V8. Contains an internal reference to its parent
    /// `MiniV8`.
    String(String),
    /// Reference to a JavaScript arrray. Contains an internal reference to its parent `MiniV8`.
    Array(Array),
    /// Reference to a JavaScript function. Contains an internal reference to its parent `MiniV8`.
    Function(Function),
    /// Reference to a JavaScript object. Contains an internal reference to its parent `MiniV8`. If
    /// a value is a function or an array in JavaScript, it will be converted to `Value::Array` or
    /// `Value::Function` instead of `Value::Object`.
    Object(Object),
}

impl Value {
    /// Returns `true` if this is a `Value::Undefined`, `false` otherwise.
    pub fn is_undefined(&self) -> bool {
        if let Value::Undefined = *self { true } else { false }
    }

    /// Returns `true` if this is a `Value::Null`, `false` otherwise.
    pub fn is_null(&self) -> bool {
        if let Value::Null = *self { true } else { false }
    }

    /// Returns `true` if this is a `Value::Boolean`, `false` otherwise.
    pub fn is_boolean(&self) -> bool {
        if let Value::Boolean(_) = *self { true } else { false }
    }

    /// Returns `true` if this is a `Value::Number`, `false` otherwise.
    pub fn is_number(&self) -> bool {
        if let Value::Number(_) = *self { true } else { false }
    }

    /// Returns `true` if this is a `Value::String`, `false` otherwise.
    pub fn is_string(&self) -> bool {
        if let Value::String(_) = *self { true } else { false }
    }

    /// Returns `true` if this is a `Value::Array`, `false` otherwise.
    pub fn is_array(&self) -> bool {
        if let Value::Array(_) = *self { true } else { false }
    }

    /// Returns `true` if this is a `Value::Function`, `false` otherwise.
    pub fn is_function(&self) -> bool {
        if let Value::Function(_) = *self { true } else { false }
    }

    /// Returns `true` if this is a `Value::Object`, `false` otherwise.
    pub fn is_object(&self) -> bool {
        if let Value::Object(_) = *self { true } else { false }
    }

    /// Returns `Some(())` if this is a `Value::Undefined`, `None` otherwise.
    pub fn as_undefined(&self) -> Option<()> {
        if let Value::Undefined = *self { Some(()) } else { None }
    }

    /// Returns `Some(())` if this is a `Value::Null`, `None` otherwise.
    pub fn as_null(&self) -> Option<()> {
        if let Value::Undefined = *self { Some(()) } else { None }
    }

    /// Returns `Some` if this is a `Value::Boolean`, `None` otherwise.
    pub fn as_boolean(&self) -> Option<bool> {
        if let Value::Boolean(value) = *self { Some(value) } else { None }
    }

    /// Returns `Some` if this is a `Value::Number`, `None` otherwise.
    pub fn as_number(&self) -> Option<f64> {
        if let Value::Number(value) = *self { Some(value) } else { None }
    }

    /// Returns `Some` if this is a `Value::String`, `None` otherwise.
    pub fn as_string(&self) -> Option<&String> {
        if let Value::String(ref value) = *self { Some(value) } else { None }
    }

    /// Returns `Some` if this is a `Value::Array`, `None` otherwise.
    pub fn as_array(&self) -> Option<&Array> {
        if let Value::Array(ref value) = *self { Some(value) } else { None }
    }

    /// Returns `Some` if this is a `Value::Function`, `None` otherwise.
    pub fn as_function(&self) -> Option<&Function> {
        if let Value::Function(ref value) = *self { Some(value) } else { None }
    }

    /// Returns `Some` if this is a `Value::Object`, `None` otherwise.
    pub fn as_object(&self) -> Option<&Object> {
        if let Value::Object(ref value) = *self { Some(value) } else { None }
    }

    /// A wrapper around `FromValue::from_value`.
    pub fn into<T: FromValue>(self, mv8: &MiniV8) -> Result<T> {
        T::from_value(self, mv8)
    }

    pub(crate) fn type_name(&self) -> &'static str {
        match *self {
            Value::Undefined => "undefined",
            Value::Null => "null",
            Value::Boolean(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Function(_) => "function",
            Value::Object(_) => "object",
        }
    }

    pub(crate) fn from_native(
        mv8: &MiniV8,
        scope: &mut v8::HandleScope<v8::Context>,
        value: v8::Local<v8::Value>,
    ) -> Value {
        if value.is_undefined() {
            return Value::Undefined;
        } else if value.is_null() {
            return Value::Null;
        } else if value.is_true() {
            return Value::Boolean(true);
        } else if value.is_false() {
            return Value::Boolean(false);
        }

        if value.is_number() {
            return Value::Number(value.number_value(scope).unwrap());
        } else if value.is_string() {
            let string = value.to_string(scope).unwrap();
            let handle = v8::Global::<v8::String>::new(scope, string);
            return Value::String(String::new(mv8, handle));
        } else if value.is_array() {
            let array: v8::Local<v8::Array> = value.try_into().unwrap();
            let handle = v8::Global::<v8::Array>::new(scope, array);
            return Value::Array(Array::new(mv8, handle));
        } else if value.is_function() {
            let function: v8::Local<v8::Function> = value.try_into().unwrap();
            let handle = v8::Global::<v8::Function>::new(scope, function);
            return Value::Function(Function::new(mv8, handle));
        } else if value.is_object() {
            let object = value.to_object(scope).unwrap();
            let handle = v8::Global::<v8::Object>::new(scope, object);
            return Value::Object(Object::new(mv8, handle));
        }

        Value::Undefined
    }

    pub(crate) fn to_native<'s>(&self, scope: &mut v8::HandleScope<'s, ()>)
        -> v8::Local<'s, v8::Value>
    {
        let scope = &mut v8::EscapableHandleScope::new(scope);

        // Perhaps we should find a way to do cross-isolate prevention here, but `rusty_v8` does it
        // for us.

        let value = match self {
            &Value::Undefined => v8::undefined(scope).into(),
            &Value::Null => v8::null(scope).into(),
            &Value::Boolean(v) => v8::Boolean::new(scope, v).into(),
            &Value::Number(v) => v8::Number::new(scope, v).into(),
            &Value::String(ref v) => v8::Local::new(scope, &v.value).into(),
            &Value::Array(ref v) => v8::Local::new(scope, &v.value).into(),
            &Value::Function(ref v) => v8::Local::new(scope, &v.value).into(),
            &Value::Object(ref v) => v8::Local::new(scope, &v.value).into(),
        };

        scope.escape(value)
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Undefined => write!(f, "undefined"),
            Value::Null => write!(f, "null"),
            Value::Boolean(b) => write!(f, "{:?}", b),
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{:?}", s),
            Value::Array(a) => write!(f, "{:?}", a),
            Value::Function(u) => write!(f, "{:?}", u),
            Value::Object(o) => write!(f, "{:?}", o),
        }
    }
}

/// Trait for types convertible to `Value`.
pub trait ToValue {
    /// Performs the conversion.
    fn to_value(self, mv8: &MiniV8) -> Result<Value>;
}

/// Trait for types convertible from `Value`.
pub trait FromValue: Sized {
    /// Performs the conversion.
    fn from_value(value: Value, mv8: &MiniV8) -> Result<Self>;
}

/// A collection of multiple JavaScript values used for interacting with function arguments.
#[derive(Clone, Debug)]
pub struct Values(Vec<Value>);

impl Values {
    /// Creates an empty `Values`.
    pub fn new() -> Values {
        Values(Vec::new())
    }

    pub fn from_vec(vec: Vec<Value>) -> Values {
        Values(vec)
    }

    pub fn into_vec(self) -> Vec<Value> {
        self.0
    }

    pub fn get(&self, index: usize) -> Value {
        self.0.get(index).map(Clone::clone).unwrap_or(Value::Undefined)
    }

    pub fn from<T: FromValue>(&self, mv8: &MiniV8, index: usize) -> Result<T> {
        T::from_value(self.0.get(index).map(Clone::clone).unwrap_or(Value::Undefined), mv8)
    }

    pub fn into<T: FromValues>(self, mv8: &MiniV8) -> Result<T> {
        T::from_values(self, mv8)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Value> {
        self.0.iter()
    }
}

impl FromIterator<Value> for Values {
    fn from_iter<I: IntoIterator<Item = Value>>(iter: I) -> Self {
        Values::from_vec(Vec::from_iter(iter))
    }
}

impl IntoIterator for Values {
    type Item = Value;
    type IntoIter = vec::IntoIter<Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Values {
    type Item = &'a Value;
    type IntoIter = slice::Iter<'a, Value>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

/// Trait for types convertible to any number of JavaScript values.
///
/// This is a generalization of `ToValue`, allowing any number of resulting JavaScript values
/// instead of just one. Any type that implements `ToValue` will automatically implement this trait.
pub trait ToValues {
    /// Performs the conversion.
    fn to_values(self, mv8: &MiniV8) -> Result<Values>;
}

/// Trait for types that can be created from an arbitrary number of JavaScript values.
///
/// This is a generalization of `FromValue`, allowing an arbitrary number of JavaScript values to
/// participate in the conversion. Any type that implements `FromValue` will automatically implement
/// this trait.
pub trait FromValues: Sized {
    /// Performs the conversion.
    ///
    /// In case `values` contains more values than needed to perform the conversion, the excess
    /// values should be ignored. Similarly, if not enough values are given, conversions should
    /// assume that any missing values are undefined.
    fn from_values(values: Values, mv8: &MiniV8) -> Result<Self>;
}

/// Wraps a variable number of `T`s.
///
/// Can be used to work with variadic functions more easily. Using this type as the last argument of
/// a Rust callback will accept any number of arguments from JavaScript and convert them to the type
/// `T` using [`FromValue`]. `Variadic<T>` can also be returned from a callback, returning a
/// variable number of values to JavaScript.
#[derive(Clone, Debug)]
pub struct Variadic<T>(pub(crate) Vec<T>);

impl<T> Variadic<T> {
    /// Creates an empty `Variadic` wrapper containing no values.
    pub fn new() -> Variadic<T> {
        Variadic(Vec::new())
    }

    pub fn from_vec(vec: Vec<T>) -> Variadic<T> {
        Variadic(vec)
    }

    pub fn into_vec(self) -> Vec<T> {
        self.0
    }
}

impl<T> FromIterator<T> for Variadic<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Variadic(Vec::from_iter(iter))
    }
}

impl<T> IntoIterator for Variadic<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> Deref for Variadic<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Variadic<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
