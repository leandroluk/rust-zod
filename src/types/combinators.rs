// @spec REQ-COMB-001
// @spec REQ-COMB-002
// @spec REQ-COMB-003
// @spec REQ-COMB-004
// @spec REQ-COMB-005
use crate::error::ZodIssueCode;
use crate::schema::{LazyDef, Schema, SchemaDef, ValidatorFn};
use crate::value::Value;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::TypeId;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn parts<T: 'static, I: IntoIterator<Item = Schema<T>>>(
    options: I,
) -> (Vec<Arc<SchemaDef>>, Vec<ValidatorFn>) {
    options
        .into_iter()
        .map(|s| (s.def.clone(), s.validator_clone()))
        .unzip()
}

/// `z::union([a, b, ...])`: first matching option wins.
pub fn union<T: 'static, I: IntoIterator<Item = Schema<T>>>(options: I) -> Schema<T> {
    let (defs, validators) = parts(options);

    Schema::new(SchemaDef::Union(defs), move |val, ctx| {
        let mut all_errors = Vec::new();
        for opt in &validators {
            let mut sub = ctx.fork();
            let res = opt(val, &mut sub);
            ctx.absorb_failure(&sub);
            if let Some(res) = res {
                if !sub.has_issues() && sub.failure.is_none() {
                    return Some(res);
                }
            }
            if let Some(err) = sub.into_error() {
                all_errors.push(err);
            }
        }
        if ctx.failure.is_some() {
            return None;
        }

        ctx.add_issue(
            ZodIssueCode::InvalidUnion {
                errors: all_errors,
                discriminator: None,
                options: Vec::new(),
                inclusive: false,
                matches: 0,
            },
            None,
        );
        None
    })
}

/// `z::xor([a, b, ...])`: exactly one option must match.
pub fn xor<T: 'static, I: IntoIterator<Item = Schema<T>>>(options: I) -> Schema<T> {
    let (defs, validators) = parts(options);

    Schema::new(SchemaDef::Xor(defs), move |val, ctx| {
        let mut matches = 0;
        let mut matched_val = None;
        let mut all_errors = Vec::new();

        for opt in &validators {
            let mut sub = ctx.fork();
            let res = opt(val, &mut sub);
            ctx.absorb_failure(&sub);
            match res {
                Some(res) if !sub.has_issues() && sub.failure.is_none() => {
                    matches += 1;
                    matched_val = Some(res);
                }
                _ => {
                    if let Some(err) = sub.into_error() {
                        all_errors.push(err);
                    }
                }
            }
        }
        if ctx.failure.is_some() {
            return None;
        }

        if matches == 1 {
            matched_val
        } else {
            ctx.add_issue(
                ZodIssueCode::InvalidUnion {
                    errors: all_errors,
                    discriminator: None,
                    options: Vec::new(),
                    inclusive: false,
                    matches,
                },
                Some(&format!(
                    "Expected exactly one matching union option, matched {}",
                    matches
                )),
            );
            None
        }
    })
}

/// `z::discriminated_union("kind", [a, b, ...])`: options are objects tagged by `discriminator`.
pub fn discriminated_union<T: 'static, I: IntoIterator<Item = Schema<T>>>(
    discriminator: &'static str,
    options: I,
) -> Schema<T> {
    let (defs, validators) = parts(options);
    let def = SchemaDef::DiscriminatedUnion {
        discriminator: discriminator.to_string(),
        options: defs,
    };

    Schema::new(def, move |val, ctx| match val {
        Value::Object(map) => {
            let disc_val: String = match map.get(discriminator) {
                Some(Value::String(s)) => s.to_string(),
                other => {
                    ctx.push_key(discriminator);
                    ctx.add_issue(
                        ZodIssueCode::InvalidType {
                            expected: "string".into(),
                            received: other.map_or("missing", |v| v.type_name()).into(),
                        },
                        Some("Missing or invalid discriminator key"),
                    );
                    ctx.pop();
                    return None;
                }
            };

            for opt in &validators {
                let mut sub = ctx.fork();
                let res = opt(val, &mut sub);
                ctx.absorb_failure(&sub);
                if let Some(res) = res {
                    if !sub.has_issues() && sub.failure.is_none() {
                        return Some(res);
                    }
                }
            }
            if ctx.failure.is_some() {
                return None;
            }

            ctx.add_issue(
                ZodIssueCode::InvalidValue {
                    values: alloc::vec![disc_val.clone()],
                },
                Some(&format!(
                    "No matching union option for discriminator '{}' = '{}'",
                    discriminator, disc_val
                )),
            );
            None
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "object".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

/// `z::intersection(a, b)`: both must match; object results are merged.
pub fn intersection<A: 'static, B: 'static>(left: Schema<A>, right: Schema<B>) -> Schema<Value> {
    let left_val = left.validator_clone();
    let right_val = right.validator_clone();
    let def = SchemaDef::Intersection(left.def.clone(), right.def.clone());

    Schema::new(def, move |val, ctx| {
        let res_left = left_val(val, ctx);
        let res_right = right_val(val, ctx);

        match (res_left, res_right) {
            (Some(Value::Object(mut o1)), Some(Value::Object(o2))) => {
                for (k, v) in o2 {
                    o1.insert(k, v);
                }
                Some(Value::Object(o1))
            }
            (Some(l), Some(r)) => {
                if l == r {
                    Some(l)
                } else {
                    ctx.issue("Intersection results could not be merged");
                    None
                }
            }
            _ => None,
        }
    })
}

// ----------------------------------------------------------------------------
// Lazy
// ----------------------------------------------------------------------------

static SHARED_LAZY: OnceLock<Mutex<HashMap<TypeId, Arc<LazyDef>>>> = OnceLock::new();

/// Capture-free closures / fn items are all equivalent, so one `LazyDef` per type is shared:
/// `fn category() -> Schema { z::object(.. z::lazy(category) ..) }` then resolves to a single node,
/// which makes the recursion finite for validation and detectable (by pointer) for exporters.
fn shared_lazy(id: TypeId, make: impl FnOnce() -> LazyDef) -> Arc<LazyDef> {
    let mut registry = SHARED_LAZY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    registry
        .entry(id)
        .or_insert_with(|| Arc::new(make()))
        .clone()
}

/// Lazy recursive schema: the closure runs once (`OnceLock`) on first use and is memoized.
pub fn lazy<T: 'static, F>(schema_fn: F) -> Schema<T>
where
    F: Fn() -> Schema<T> + Send + Sync + 'static,
{
    let is_zst = core::mem::size_of::<F>() == 0;
    let id = TypeId::of::<F>();
    let make = move || LazyDef::new(move || schema_fn().into_type::<Value>());
    let node: Arc<LazyDef> = if is_zst {
        shared_lazy(id, make)
    } else {
        Arc::new(make())
    };

    let runtime = node.clone();
    Schema::new(SchemaDef::Lazy(node), move |val, ctx| {
        let inner = runtime.get();
        (inner.validator)(val, ctx)
    })
}
