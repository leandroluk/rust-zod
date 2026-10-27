// @spec REQ-PRIM-001
use crate::error::ZodIssueCode;
use crate::schema::{Schema, SchemaDef};
use crate::value::Value;
use alloc::string::String;
use alloc::vec::Vec;

/// Schema matching only `null`.
pub fn null() -> Schema<()> {
    Schema::new(SchemaDef::Null, |val, ctx| match val {
        Value::Null => Some(Value::Null),
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "null".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

/// Schema accepting any value (type-safe unknown).
pub fn unknown() -> Schema<Value> {
    Schema::new(SchemaDef::Any, |val, _ctx| Some(val.clone()))
}

/// Schema for dates (represented as Unix millisecond timestamp `Value::Date(i64)`).
pub fn date() -> Schema<i64> {
    Schema::new(SchemaDef::Date, |val, ctx| match val {
        Value::Date(d) => Some(Value::Date(*d)),
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "date".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

/// Schema for raw bytes (`Value::Bytes`).
pub fn bytes() -> Schema<Vec<u8>> {
    Schema::new(SchemaDef::Bytes, |val, ctx| match val {
        Value::Bytes(b) => Some(Value::Bytes(b.clone())),
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "bytes".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

/// JSON schema accepting any valid JSON-compatible structure.
pub fn json() -> Schema<Value> {
    Schema::new(SchemaDef::Any, |val, ctx| match val {
        Value::Date(_) | Value::Bytes(_) => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "json".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
        _ => Some(val.clone()),
    })
}

/// Trait to support single or multiple literal values for `z::literal(...)`.
pub trait IntoLiterals {
    fn into_literals(self) -> Vec<Value>;
}

impl IntoLiterals for Value {
    fn into_literals(self) -> Vec<Value> {
        alloc::vec![self]
    }
}

impl IntoLiterals for &str {
    fn into_literals(self) -> Vec<Value> {
        alloc::vec![Value::from(self)]
    }
}

impl IntoLiterals for String {
    fn into_literals(self) -> Vec<Value> {
        alloc::vec![Value::from(self)]
    }
}

impl IntoLiterals for bool {
    fn into_literals(self) -> Vec<Value> {
        alloc::vec![Value::from(self)]
    }
}

impl IntoLiterals for i64 {
    fn into_literals(self) -> Vec<Value> {
        alloc::vec![Value::from(self)]
    }
}

impl IntoLiterals for i32 {
    fn into_literals(self) -> Vec<Value> {
        alloc::vec![Value::from(self as i64)]
    }
}

impl IntoLiterals for f64 {
    fn into_literals(self) -> Vec<Value> {
        alloc::vec![Value::from(self)]
    }
}

impl<const N: usize> IntoLiterals for [&str; N] {
    fn into_literals(self) -> Vec<Value> {
        self.into_iter().map(Value::from).collect()
    }
}

impl IntoLiterals for &[&str] {
    fn into_literals(self) -> Vec<Value> {
        self.iter().map(|&s| Value::from(s)).collect()
    }
}

impl<const N: usize> IntoLiterals for [Value; N] {
    fn into_literals(self) -> Vec<Value> {
        self.into_iter().collect()
    }
}

impl IntoLiterals for &[Value] {
    fn into_literals(self) -> Vec<Value> {
        self.to_vec()
    }
}

impl IntoLiterals for Vec<Value> {
    fn into_literals(self) -> Vec<Value> {
        self
    }
}

/// Literal schema accepting one or more exact values (Zod v4 multi-literal support).
pub fn literal<L: IntoLiterals>(lits: L) -> Schema<Value> {
    let expected_values = lits.into_literals();
    let expected_strings: Vec<String> = expected_values.iter().map(|v| v.to_string()).collect();
    let allowed = expected_values.clone();

    Schema::new(
        SchemaDef::Literal(expected_values.clone()),
        move |val, ctx| {
            if allowed.contains(val) {
                Some(val.clone())
            } else {
                ctx.add_issue(
                    ZodIssueCode::InvalidValue {
                        values: expected_strings.clone(),
                    },
                    None,
                );
                None
            }
        },
    )
}
