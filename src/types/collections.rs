// @spec REQ-COLL-001
// @spec REQ-COLL-002
// @spec REQ-COLL-003
// @spec REQ-COLL-004
// @spec REQ-COLL-005
use super::{too_big, too_small};
use crate::error::{SizeOrigin, ZodIssueCode};
use crate::schema::{Aux, Check, CheckOpts, Schema, SchemaDef};
use crate::value::Value;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

// ----------------------------------------------------------------------------
// Options (Enums)
// ----------------------------------------------------------------------------

impl Schema<String> {
    /// The allowed option strings, if this is an options schema (`z::options([...])`).
    pub fn options(&self) -> Option<Vec<String>> {
        match self.def.strip_meta() {
            SchemaDef::Options(opts) => Some(opts.clone()),
            _ => None,
        }
    }

    /// Alias of `.options()`.
    pub fn options_list(&self) -> Option<Vec<String>> {
        self.options()
    }

    /// Return a new options schema with only the specified options extracted.
    pub fn extract<O: super::IntoOptions>(&self, subset: O) -> Self {
        let subset = subset.into_options();
        let current = self.options().unwrap_or_default();
        crate::z::options(
            current
                .into_iter()
                .filter(|o| subset.contains(o))
                .collect::<Vec<_>>(),
        )
    }

    /// Return a new options schema with the specified options excluded.
    pub fn exclude<O: super::IntoOptions>(&self, excluded: O) -> Self {
        let excluded = excluded.into_options();
        let current = self.options().unwrap_or_default();
        crate::z::options(
            current
                .into_iter()
                .filter(|o| !excluded.contains(o))
                .collect::<Vec<_>>(),
        )
    }
}

// ----------------------------------------------------------------------------
// Arrays (Schema<Vec<T>>)
// ----------------------------------------------------------------------------

impl<T: 'static> Schema<Vec<T>> {
    /// Minimum element count.
    pub fn min(self, minimum: usize) -> Self {
        self.min_with(minimum, |e| e)
    }

    pub fn min_with(self, minimum: usize, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.push_check(
            Check::Min {
                value: minimum as f64,
                inclusive: true,
            },
            opts(CheckOpts::default()),
            move |v| match v {
                Value::Array(arr) if arr.len() < minimum => Some((
                    too_small(SizeOrigin::Array, minimum as f64, true, false),
                    None,
                )),
                _ => None,
            },
        )
    }

    /// Maximum element count.
    pub fn max(self, maximum: usize) -> Self {
        self.max_with(maximum, |e| e)
    }

    pub fn max_with(self, maximum: usize, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.push_check(
            Check::Max {
                value: maximum as f64,
                inclusive: true,
            },
            opts(CheckOpts::default()),
            move |v| match v {
                Value::Array(arr) if arr.len() > maximum => Some((
                    too_big(SizeOrigin::Array, maximum as f64, true, false),
                    None,
                )),
                _ => None,
            },
        )
    }

    /// Exact element count.
    pub fn length(self, len: usize) -> Self {
        self.length_with(len, |e| e)
    }

    pub fn length_with(self, len: usize, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.push_check(
            Check::Length(len),
            opts(CheckOpts::default()),
            move |v| match v {
                Value::Array(arr) if arr.len() != len => {
                    Some((too_small(SizeOrigin::Array, len as f64, true, true), None))
                }
                _ => None,
            },
        )
    }

    /// Enforce at least 1 element.
    pub fn nonempty(self) -> Self {
        self.min(1)
    }

    /// Enforce element uniqueness.
    pub fn unique(self) -> Self {
        self.push_check(Check::Unique, CheckOpts::default(), |v| match v {
            Value::Array(arr) => {
                for i in 0..arr.len() {
                    if arr[i + 1..].contains(&arr[i]) {
                        return Some((
                            ZodIssueCode::Custom {
                                params: BTreeMap::new(),
                            },
                            Some(String::from("Array elements must be unique")),
                        ));
                    }
                }
                None
            }
            _ => None,
        })
    }
}

// ----------------------------------------------------------------------------
// Tuples
// ----------------------------------------------------------------------------

/// Runtime structure of a tuple schema.
pub struct TupleParts {
    pub items: Vec<Schema<Value>>,
    pub rest: Option<Schema<Value>>,
}

/// Anything `z::tuple(...)` accepts: a fixed-size array of schemas (output `Vec<T>`)
/// or a Rust tuple of schemas (output is the tuple of their outputs).
pub trait IntoTuple {
    type Out: 'static;
    fn into_items(self) -> Vec<Schema<Value>>;
}

impl<T: 'static, const N: usize> IntoTuple for [Schema<T>; N] {
    type Out = Vec<T>;
    fn into_items(self) -> Vec<Schema<Value>> {
        self.into_iter().map(|s| s.into_type::<Value>()).collect()
    }
}

impl<T: 'static> IntoTuple for Vec<Schema<T>> {
    type Out = Vec<T>;
    fn into_items(self) -> Vec<Schema<Value>> {
        self.into_iter().map(|s| s.into_type::<Value>()).collect()
    }
}

macro_rules! impl_into_tuple {
    ($(($($name:ident $idx:tt),+)),+ $(,)?) => {$(
        impl<$($name: 'static),+> IntoTuple for ($(Schema<$name>,)+) {
            type Out = ($($name,)+);
            fn into_items(self) -> Vec<Schema<Value>> {
                alloc::vec![$(self.$idx.into_type::<Value>()),+]
            }
        }
    )+};
}

impl_into_tuple! {
    (A 0),
    (A 0, B 1),
    (A 0, B 1, C 2),
    (A 0, B 1, C 2, D 3),
    (A 0, B 1, C 2, D 3, E 4),
    (A 0, B 1, C 2, D 3, E 4, F 5),
}

fn build_tuple(items: Vec<Schema<Value>>, rest: Option<Schema<Value>>) -> Schema<Value> {
    let def = SchemaDef::Tuple {
        items: items.iter().map(|s| s.def.clone()).collect(),
        rest: rest.as_ref().map(|r| r.def.clone()),
    };
    let validators: Vec<_> = items.iter().map(|s| s.validator_clone()).collect();
    let rest_v = rest.as_ref().map(|r| r.validator_clone());
    let has_rest = rest.is_some();
    let parts = Arc::new(TupleParts { items, rest });

    let mut schema = Schema::<Value>::new(def, move |val, ctx| match val {
        Value::Array(values) => {
            if !has_rest && values.len() != validators.len() {
                ctx.add_issue(
                    if values.len() < validators.len() {
                        too_small(SizeOrigin::Array, validators.len() as f64, true, true)
                    } else {
                        too_big(SizeOrigin::Array, validators.len() as f64, true, true)
                    },
                    Some(&format!(
                        "Expected tuple of {} elements, received {}",
                        validators.len(),
                        values.len()
                    )),
                );
            } else if has_rest && values.len() < validators.len() {
                ctx.add_issue(
                    too_small(SizeOrigin::Array, validators.len() as f64, true, false),
                    None,
                );
            }

            let mut out = Vec::with_capacity(values.len());
            for (idx, item) in values.iter().enumerate() {
                ctx.push_index(idx);
                let validator = if idx < validators.len() {
                    Some(&validators[idx])
                } else {
                    rest_v.as_ref()
                };
                if let Some(v) = validator {
                    if let Some(valid) = v(item, ctx) {
                        out.push(valid);
                    }
                }
                ctx.pop();
            }
            Some(Value::Array(out))
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "array".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    });
    schema.aux = Aux::Tuple(parts);
    schema
}

/// `z::tuple([a, b])` (homogeneous, `Vec<T>`) or `z::tuple((a, b))` (heterogeneous, `(A, B)`).
pub fn tuple<I: IntoTuple>(items: I) -> Schema<I::Out> {
    build_tuple(items.into_items(), None).into_type::<I::Out>()
}

impl<T: 'static> Schema<T> {
    /// Add a rest element schema to a tuple (variadic tail). Panics on non-tuple schemas.
    pub fn rest<R: 'static>(&self, rest_schema: Schema<R>) -> Schema<Vec<Value>> {
        match &self.aux {
            Aux::Tuple(parts) => {
                build_tuple(parts.items.clone(), Some(rest_schema.into_type::<Value>())).into_type()
            }
            _ => panic!("`.rest()` is only available on tuple schemas (z::tuple)"),
        }
    }
}

// ----------------------------------------------------------------------------
// Sets
// ----------------------------------------------------------------------------

pub fn set<T: 'static>(element_schema: Schema<T>) -> Schema<Vec<T>> {
    let inner_validator = element_schema.validator_clone();
    let def = SchemaDef::Set(element_schema.def.clone());

    Schema::new(def, move |val, ctx| match val {
        Value::Array(items) => {
            let mut valid_items: Vec<Value> = Vec::with_capacity(items.len());
            for (idx, item) in items.iter().enumerate() {
                ctx.push_index(idx);
                if let Some(res) = inner_validator(item, ctx) {
                    if valid_items.contains(&res) {
                        ctx.issue("Set elements must be unique");
                    } else {
                        valid_items.push(res);
                    }
                }
                ctx.pop();
            }
            Some(Value::Array(valid_items))
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "set".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

// ----------------------------------------------------------------------------
// Maps
// ----------------------------------------------------------------------------

/// `z::map(key, value)`: keyed collection; in a JSON-like `Value` it is represented as an object.
pub fn map<K: 'static, V: 'static>(
    key_schema: Schema<K>,
    value_schema: Schema<V>,
) -> Schema<BTreeMap<String, V>> {
    crate::types::object::record(key_schema, value_schema)
}
