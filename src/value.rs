// @spec REQ-CORE-001
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use compact_str::CompactString;
use core::fmt;

/// Runtime Value model for `rust-zod`, representing all valid data variants
/// inspectable and transformable by schemas without compile-time macro bloat.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Uint(u64),
    Float(f64),
    /// Strings of up to 24 bytes live inline in the enum (no heap allocation).
    String(CompactString),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
    Date(i64), // Unix timestamp in milliseconds UTC
    Bytes(Vec<u8>),
}

impl Value {
    /// Return the human-readable type name of the value matching Zod type strings.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Int(_) | Value::Uint(_) | Value::Float(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Date(_) => "date",
            Value::Bytes(_) => "bytes",
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Uint(u) => i64::try_from(*u).ok(),
            Value::Float(f) if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 => {
                Some(*f as i64)
            }
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Uint(u) => Some(*u),
            Value::Int(i) if *i >= 0 => Some(*i as u64),
            Value::Float(f) if f.fract() == 0.0 && *f >= 0.0 && *f <= u64::MAX as f64 => {
                Some(*f as u64)
            }
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            Value::Uint(u) => Some(*u as f64),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut BTreeMap<String, Value>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_date(&self) -> Option<i64> {
        match self {
            Value::Date(d) => Some(*d),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b.as_slice()),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Uint(u) => write!(f, "{}", u),
            Value::Float(v) => write!(f, "{}", v),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Array(arr) => {
                write!(f, "[")?;
                for (idx, item) in arr.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Object(obj) => {
                write!(f, "{{")?;
                for (idx, (k, v)) in obj.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Date(d) => write!(f, "Date({})", d),
            Value::Bytes(b) => write!(f, "Bytes({} bytes)", b.len()),
        }
    }
}

// Conversions from Rust primitive and standard types
impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i8> for Value {
    fn from(n: i8) -> Self {
        Value::Int(n as i64)
    }
}

impl From<i16> for Value {
    fn from(n: i16) -> Self {
        Value::Int(n as i64)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Int(n as i64)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Int(n)
    }
}

impl From<isize> for Value {
    fn from(n: isize) -> Self {
        Value::Int(n as i64)
    }
}

impl From<u8> for Value {
    fn from(n: u8) -> Self {
        Value::Uint(n as u64)
    }
}

impl From<u16> for Value {
    fn from(n: u16) -> Self {
        Value::Uint(n as u64)
    }
}

impl From<u32> for Value {
    fn from(n: u32) -> Self {
        Value::Uint(n as u64)
    }
}

impl From<u64> for Value {
    fn from(n: u64) -> Self {
        Value::Uint(n)
    }
}

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Value::Uint(n as u64)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Float(f as f64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(CompactString::new(s))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(CompactString::from(s))
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(vec: Vec<T>) -> Self {
        Value::Array(vec.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value> + Clone> From<&[T]> for Value {
    fn from(slice: &[T]) -> Self {
        Value::Array(slice.iter().cloned().map(Into::into).collect())
    }
}

impl<K: Into<String>, V: Into<Value>> From<BTreeMap<K, V>> for Value {
    fn from(map: BTreeMap<K, V>) -> Self {
        let mut obj = BTreeMap::new();
        for (k, v) in map {
            obj.insert(k.into(), v.into());
        }
        Value::Object(obj)
    }
}

#[cfg(feature = "serde_json")]
impl From<serde_json::Value> for Value {
    fn from(json: serde_json::Value) -> Self {
        match json {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Bool(b),
            serde_json::Value::Number(num) => {
                if let Some(i) = num.as_i64() {
                    Value::Int(i)
                } else if let Some(u) = num.as_u64() {
                    Value::Uint(u)
                } else if let Some(f) = num.as_f64() {
                    Value::Float(f)
                } else {
                    Value::Float(0.0)
                }
            }
            serde_json::Value::String(s) => Value::String((s).into()),
            serde_json::Value::Array(arr) => {
                Value::Array(arr.into_iter().map(Value::from).collect())
            }
            serde_json::Value::Object(map) => {
                let mut obj = BTreeMap::new();
                for (k, v) in map {
                    obj.insert(k, Value::from(v));
                }
                Value::Object(obj)
            }
        }
    }
}

#[cfg(feature = "serde_json")]
impl From<Value> for serde_json::Value {
    fn from(val: Value) -> Self {
        match val {
            Value::Null => serde_json::Value::Null,
            Value::Bool(b) => serde_json::Value::Bool(b),
            Value::Int(i) => serde_json::Value::Number(serde_json::Number::from(i)),
            Value::Uint(u) => serde_json::Value::Number(serde_json::Number::from(u)),
            Value::Float(f) => serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Value::String(s) => serde_json::Value::String(s.into()),
            Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(serde_json::Value::from).collect())
            }
            Value::Object(obj) => {
                let mut map = serde_json::Map::new();
                for (k, v) in obj {
                    map.insert(k, serde_json::Value::from(v));
                }
                serde_json::Value::Object(map)
            }
            Value::Date(d) => serde_json::Value::Number(serde_json::Number::from(d)),
            Value::Bytes(b) => {
                serde_json::Value::Array(b.into_iter().map(serde_json::Value::from).collect())
            }
        }
    }
}

#[cfg(feature = "serde_json")]
impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let json_val = serde_json::Value::deserialize(deserializer)?;
        Ok(Value::from(json_val))
    }
}

#[cfg(feature = "serde_json")]
impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let json_val = serde_json::Value::from(self.clone());
        json_val.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_value_conversions() {
        assert_eq!(Value::from(42i32), Value::Int(42));
        assert_eq!(Value::from("hello"), Value::String("hello".into()));
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(()), Value::Null);
        assert_eq!(Value::from(vec![1, 2, 3]), Value::Array(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
    }

    #[cfg(feature = "serde_json")]
    #[test]
    fn test_serde_json_interop() {
        let json_str = r#"{"name":"Alice","age":30,"tags":["rust","zod"]}"#;
        let json_val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let val: Value = json_val.into();

        assert_eq!(val.type_name(), "object");
        let obj = val.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap().as_str(), Some("Alice"));
        assert_eq!(obj.get("age").unwrap().as_i64(), Some(30));
        assert_eq!(obj.get("tags").unwrap().as_array().unwrap().len(), 2);
    }
}
