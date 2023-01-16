use crate::*;
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
    /// Elapsed milliseconds since Unix epoch.
    Date(f64),
    /// An immutable JavaScript string, managed by V8.
    String(String),
    /// Reference to a JavaScript arrray.
    Array(Array),
    /// Reference to a JavaScript function.
    Function(Function),
    /// Reference to a JavaScript object. If a value is a function or an array in JavaScript, it
    /// will be converted to `Value::Array` or `Value::Function` instead of `Value::Object`.
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

    /// Returns `true` if this is a `Value::Date`, `false` otherwise.
    pub fn is_date(&self) -> bool {
        if let Value::Date(_) = *self { true } else { false }
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

    /// Returns `Some` if this is a `Value::Date`, `None` otherwise.
    pub fn as_date(&self) -> Option<f64> {
        if let Value::Date(value) = *self { Some(value) } else { None }
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
    pub fn into<T: FromValue>(self, mv8: &MiniV8) -> Result< T> {
        T::from_value(self, mv8)
    }

    /// Coerces a value to a boolean. Returns `true` if the value is "truthy", `false` otherwise.
    pub fn coerce_boolean(&self, mv8: &MiniV8) -> bool {
        match self {
            &Value::Boolean(b) => b,
            value => mv8.scope(|scope| value.to_v8_value(scope).boolean_value(scope)),
        }
    }

    /// Coerces a value to a number. Nearly all JavaScript values are coercible to numbers, but this
    /// may fail with a runtime error under extraordinary circumstances (e.g. if the ECMAScript
    /// `ToNumber` implementation throws an error).
    ///
    /// This will return `std::f64::NAN` if the value has no numerical equivalent.
    pub fn coerce_number(&self, mv8: &MiniV8) -> Result<f64> {
        match self {
            &Value::Number(n) => Ok(n),
            value => mv8.try_catch(|scope| {
                let maybe = value.to_v8_value(scope).to_number(scope);
                mv8.exception(scope).map(|_| maybe.unwrap().value())
            }),
        }
    }

    /// Coerces a value to a string. Nearly all JavaScript values are coercible to strings, but this
    /// may fail with a runtime error if `toString()` fails or under otherwise extraordinary
    /// circumstances (e.g. if the ECMAScript `ToString` implementation throws an error).
    pub fn coerce_string(&self, mv8: &MiniV8) -> Result<String> {
        match self {
            &Value::String(ref s) => Ok(s.clone()),
            value => mv8.try_catch(|scope| {
                let maybe = value.to_v8_value(scope).to_string(scope);
                mv8.exception(scope).map(|_| String {
                    mv8: mv8.clone(),
                    handle: v8::Global::new(scope, maybe.unwrap()),
                })
            }),
        }
    }

    pub(crate) fn type_name(&self) -> &'static str {
        match *self {
            Value::Undefined => "undefined",
            Value::Null => "null",
            Value::Boolean(_) => "boolean",
            Value::Number(_) => "number",
            Value::Date(_) => "date",
            Value::Function(_) => "function",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::String(_) => "string",
        }
    }

    pub(crate) fn from_v8_value(
        mv8: &MiniV8,
        scope: &mut v8::HandleScope,
        value: v8::Local<v8::Value>,
    ) -> Value {
        if value.is_undefined() {
            Value::Undefined
        } else if value.is_null() {
            Value::Null
        } else if value.is_boolean() {
            Value::Boolean(value.boolean_value(scope))
        } else if value.is_int32() {
            Value::Number(value.int32_value(scope).unwrap() as f64)
        } else if value.is_number() {
            Value::Number(value.number_value(scope).unwrap())
        } else if value.is_date() {
            let value: v8::Local<v8::Date> = value.try_into().unwrap();
            Value::Date(value.value_of())
        } else if value.is_string() {
            let value: v8::Local<v8::String> = value.try_into().unwrap();
            let handle = v8::Global::new(scope, value);
            Value::String(String { mv8: mv8.clone(), handle })
        } else if value.is_array() {
            let value: v8::Local<v8::Array> = value.try_into().unwrap();
            let handle = v8::Global::new(scope, value);
            Value::Array(Array { mv8: mv8.clone(), handle })
        } else if value.is_function() {
            let value: v8::Local<v8::Function> = value.try_into().unwrap();
            let handle = v8::Global::new(scope, value);
            Value::Function(Function { mv8: mv8.clone(), handle })
        } else if value.is_object() {
            let value: v8::Local<v8::Object> = value.try_into().unwrap();
            let handle = v8::Global::new(scope, value);
            Value::Object(Object { mv8: mv8.clone(), handle })
        } else {
            Value::Undefined
        }
    }

    pub(crate) fn to_v8_value<'s>(&self, scope: &mut v8::HandleScope<'s>)
        -> v8::Local<'s, v8::Value>
    {
        match self {
            Value::Undefined => v8::undefined(scope).into(),
            Value::Null => v8::null(scope).into(),
            Value::Boolean(v) => v8::Boolean::new(scope, *v).into(),
            Value::Number(v) => v8::Number::new(scope, *v).into(),
            Value::Date(v) => v8::Date::new(scope, *v).unwrap().into(),
            Value::Function(v) => v8::Local::new(scope, v.handle.clone()).into(),
            Value::Array(v) => v8::Local::new(scope, v.handle.clone()).into(),
            Value::Object(v) => v8::Local::new(scope, v.handle.clone()).into(),
            Value::String(v) => v8::Local::new(scope, v.handle.clone()).into(),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Undefined => write!(f, "undefined"),
            Value::Null => write!(f, "null"),
            Value::Boolean(b) => write!(f, "{:?}", b),
            Value::Number(n) => write!(f, "{}", n),
            Value::Date(d) => write!(f, "date:{}", d),
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
#[derive(Clone)]
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
#[derive(Clone)]
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
