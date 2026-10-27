// @spec REQ-PRIM-007
use super::{too_big, too_small};
use crate::error::{SizeOrigin, ZodIssueCode};
use crate::schema::{Check, CheckOpts, NumKind, Schema, SchemaDef};
use crate::value::Value;
use core::fmt::Debug;

pub trait Numeric: Copy + PartialOrd + Debug + Send + Sync + 'static {
    fn to_f64(self) -> f64;
    fn zero() -> Self;
}

impl Numeric for f64 {
    fn to_f64(self) -> f64 {
        self
    }
    fn zero() -> Self {
        0.0
    }
}

impl Numeric for f32 {
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn zero() -> Self {
        0.0
    }
}

impl Numeric for i64 {
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn zero() -> Self {
        0
    }
}

impl Numeric for i32 {
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn zero() -> Self {
        0
    }
}

impl Numeric for u32 {
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn zero() -> Self {
        0
    }
}

impl Numeric for u64 {
    fn to_f64(self) -> f64 {
        self as f64
    }
    fn zero() -> Self {
        0
    }
}

impl<N: Numeric> Schema<N> {
    fn bound_min(self, bound: N, inclusive: bool, opts: CheckOpts) -> Self {
        let b = bound.to_f64();
        self.push_check(Check::Min { value: b, inclusive }, opts, move |v| match v.as_f64() {
            Some(num) if (inclusive && num < b) || (!inclusive && num <= b) => {
                Some((too_small(SizeOrigin::Number, b, inclusive, false), None))
            }
            _ => None,
        })
    }

    fn bound_max(self, bound: N, inclusive: bool, opts: CheckOpts) -> Self {
        let b = bound.to_f64();
        self.push_check(Check::Max { value: b, inclusive }, opts, move |v| match v.as_f64() {
            Some(num) if (inclusive && num > b) || (!inclusive && num >= b) => {
                Some((too_big(SizeOrigin::Number, b, inclusive, false), None))
            }
            _ => None,
        })
    }

    pub fn gt(self, bound: N) -> Self {
        self.gt_with(bound, |e| e)
    }

    pub fn gt_with(self, bound: N, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.bound_min(bound, false, opts(CheckOpts::default()))
    }

    pub fn gte(self, bound: N) -> Self {
        self.gte_with(bound, |e| e)
    }

    pub fn gte_with(self, bound: N, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.bound_min(bound, true, opts(CheckOpts::default()))
    }

    pub fn min(self, bound: N) -> Self {
        self.gte(bound)
    }

    pub fn min_with(self, bound: N, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.gte_with(bound, opts)
    }

    pub fn lt(self, bound: N) -> Self {
        self.lt_with(bound, |e| e)
    }

    pub fn lt_with(self, bound: N, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.bound_max(bound, false, opts(CheckOpts::default()))
    }

    pub fn lte(self, bound: N) -> Self {
        self.lte_with(bound, |e| e)
    }

    pub fn lte_with(self, bound: N, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.bound_max(bound, true, opts(CheckOpts::default()))
    }

    pub fn max(self, bound: N) -> Self {
        self.lte(bound)
    }

    pub fn max_with(self, bound: N, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.lte_with(bound, opts)
    }

    pub fn positive(self) -> Self {
        self.gt(N::zero())
    }

    pub fn nonnegative(self) -> Self {
        self.gte(N::zero())
    }

    pub fn negative(self) -> Self {
        self.lt(N::zero())
    }

    pub fn nonpositive(self) -> Self {
        self.lte(N::zero())
    }

    pub fn multiple_of(self, step: N) -> Self {
        self.multiple_of_with(step, |e| e)
    }

    pub fn multiple_of_with(self, step: N, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        let divisor = step.to_f64();
        self.push_check(Check::MultipleOf(divisor), opts(CheckOpts::default()), move |v| match v.as_f64() {
            Some(num) => {
                let rem = (num % divisor).abs();
                if rem > 1e-9 && (rem - divisor.abs()).abs() > 1e-9 {
                    Some((ZodIssueCode::NotMultipleOf { divisor }, None))
                } else {
                    None
                }
            }
            None => None,
        })
    }
}

// ----------------------------------------------------------------------------
// Numeric Constructors
// ----------------------------------------------------------------------------

pub fn float64() -> Schema<f64> {
    Schema::new(SchemaDef::number(NumKind::Float64), |val, ctx| match val {
        Value::Float(f) if !f.is_nan() && !f.is_infinite() => Some(Value::Float(*f)),
        Value::Int(i) => Some(Value::Float(*i as f64)),
        Value::Uint(u) => Some(Value::Float(*u as f64)),
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

pub fn float32() -> Schema<f32> {
    Schema::new(SchemaDef::number(NumKind::Float32), |val, ctx| match val {
        Value::Float(f) if !f.is_nan() && !f.is_infinite() => Some(Value::Float(*f)),
        Value::Int(i) => Some(Value::Float(*i as f64)),
        Value::Uint(u) => Some(Value::Float(*u as f64)),
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

pub fn int() -> Schema<i64> {
    int64()
}

pub fn int64() -> Schema<i64> {
    Schema::new(SchemaDef::number(NumKind::Int64), |val, ctx| match val {
        Value::Int(i) => Some(Value::Int(*i)),
        Value::Uint(u) if *u <= i64::MAX as u64 => Some(Value::Int(*u as i64)),
        Value::Float(f) if f.fract() == 0.0 && *f >= i64::MIN as f64 && *f <= i64::MAX as f64 => {
            Some(Value::Int(*f as i64))
        }
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

pub fn int32() -> Schema<i32> {
    Schema::new(SchemaDef::number(NumKind::Int32), |val, ctx| match val {
        Value::Int(i) if *i >= i32::MIN as i64 && *i <= i32::MAX as i64 => Some(Value::Int(*i)),
        Value::Uint(u) if *u <= i32::MAX as u64 => Some(Value::Int(*u as i64)),
        Value::Float(f) if f.fract() == 0.0 && *f >= i32::MIN as f64 && *f <= i32::MAX as f64 => {
            Some(Value::Int(*f as i64))
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "int32".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

pub fn uint32() -> Schema<u32> {
    Schema::new(SchemaDef::number(NumKind::Uint32), |val, ctx| match val {
        Value::Uint(u) if *u <= u32::MAX as u64 => Some(Value::Uint(*u)),
        Value::Int(i) if *i >= 0 && *i <= u32::MAX as i64 => Some(Value::Uint(*i as u64)),
        Value::Float(f) if f.fract() == 0.0 && *f >= 0.0 && *f <= u32::MAX as f64 => {
            Some(Value::Uint(*f as u64))
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "uint32".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

pub fn uint64() -> Schema<u64> {
    Schema::new(SchemaDef::number(NumKind::Uint64), |val, ctx| match val {
        Value::Uint(u) => Some(Value::Uint(*u)),
        Value::Int(i) if *i >= 0 => Some(Value::Uint(*i as u64)),
        Value::Float(f) if f.fract() == 0.0 && *f >= 0.0 && *f <= u64::MAX as f64 => {
            Some(Value::Uint(*f as u64))
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "uint64".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

pub fn bigint() -> Schema<i64> {
    Schema::new(SchemaDef::BigInt, |val, ctx| match val {
        Value::Int(i) => Some(Value::Int(*i)),
        Value::Uint(u) if *u <= i64::MAX as u64 => Some(Value::Int(*u as i64)),
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "bigint".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}
