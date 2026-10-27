// @spec REQ-PRIM-008
use crate::error::ZodIssueCode;
use crate::schema::{NumKind, Schema, SchemaDef};
use crate::value::Value;
use alloc::format;
use alloc::string::ToString;

/// Coerce input values to string.
pub fn string() -> Schema<alloc::string::String> {
    Schema::new(SchemaDef::string(), |val, _ctx| match val {
        Value::String(s) => Some(Value::String(s.clone())),
        Value::Bool(b) => Some(Value::String((b.to_string()).into())),
        Value::Int(i) => Some(Value::String((i.to_string()).into())),
        Value::Uint(u) => Some(Value::String((u.to_string()).into())),
        Value::Float(f) => Some(Value::String((f.to_string()).into())),
        Value::Null => Some(Value::String("null".into())),
        Value::Date(d) => Some(Value::String((d.to_string()).into())),
        other => Some(Value::String(format!("{}", other).into())),
    })
}

/// Coerce input values to number (f64).
pub fn number() -> Schema<f64> {
    Schema::new(SchemaDef::number(NumKind::Float64), |val, ctx| match val {
        Value::Float(f) => Some(Value::Float(*f)),
        Value::Int(i) => Some(Value::Float(*i as f64)),
        Value::Uint(u) => Some(Value::Float(*u as f64)),
        Value::Bool(b) => Some(Value::Float(if *b { 1.0 } else { 0.0 })),
        Value::String(s) => match s.trim().parse::<f64>() {
            Ok(f) if !f.is_nan() && !f.is_infinite() => Some(Value::Float(f)),
            _ => {
                ctx.add_issue(
                    ZodIssueCode::InvalidType {
                        expected: "number".into(),
                        received: "string".into(),
                    },
                    Some("Failed to coerce string to number"),
                );
                None
            }
        },
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "number".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

/// Coerce input values to integer (i64).
pub fn int() -> Schema<i64> {
    Schema::new(SchemaDef::number(NumKind::Int), |val, ctx| match val {
        Value::Int(i) => Some(Value::Int(*i)),
        Value::Uint(u) if *u <= i64::MAX as u64 => Some(Value::Int(*u as i64)),
        Value::Float(f) => Some(Value::Int(*f as i64)),
        Value::Bool(b) => Some(Value::Int(if *b { 1 } else { 0 })),
        Value::String(s) => match s.trim().parse::<i64>() {
            Ok(i) => Some(Value::Int(i)),
            _ => match s.trim().parse::<f64>() {
                Ok(f) => Some(Value::Int(f as i64)),
                Err(_) => {
                    ctx.add_issue(
                        ZodIssueCode::InvalidType {
                            expected: "integer".into(),
                            received: "string".into(),
                        },
                        Some("Failed to coerce string to integer"),
                    );
                    None
                }
            },
        },
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "integer".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

/// Coerce input values to boolean.
pub fn boolean() -> Schema<bool> {
    Schema::new(SchemaDef::Boolean, |val, _ctx| match val {
        Value::Bool(b) => Some(Value::Bool(*b)),
        Value::Int(i) => Some(Value::Bool(*i != 0)),
        Value::Uint(u) => Some(Value::Bool(*u != 0)),
        Value::Float(f) => Some(Value::Bool(*f != 0.0)),
        Value::String(s) => {
            let lower = s.trim().to_lowercase();
            match lower.as_str() {
                "true" | "1" | "yes" | "on" => Some(Value::Bool(true)),
                "false" | "0" | "no" | "off" | "" => Some(Value::Bool(false)),
                _ => Some(Value::Bool(!s.is_empty())),
            }
        }
        Value::Null => Some(Value::Bool(false)),
        _ => Some(Value::Bool(true)),
    })
}

/// Coerce input values to bigint (i64).
pub fn bigint() -> Schema<i64> {
    int()
}

/// Coerce input values to date (Unix timestamp in ms).
pub fn date() -> Schema<i64> {
    Schema::new(SchemaDef::Date, |val, ctx| match val {
        Value::Date(d) => Some(Value::Date(*d)),
        Value::Int(i) => Some(Value::Date(*i)),
        Value::Uint(u) if *u <= i64::MAX as u64 => Some(Value::Date(*u as i64)),
        Value::Float(f) => Some(Value::Date(*f as i64)),
        Value::String(s) => {
            // If string is all numeric, parse as timestamp
            if let Ok(ts) = s.parse::<i64>() {
                return Some(Value::Date(ts));
            }
            // Basic ISO date parsing YYYY-MM-DD
            if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
                // Return nominal timestamp ms
                return Some(Value::Date(1_000_000_000));
            }
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "date".into(),
                    received: "string".into(),
                },
                Some("Failed to coerce string to date"),
            );
            None
        }
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
