// @spec REQ-OBJ-001
// @spec REQ-OBJ-002
// @spec REQ-OBJ-003
// @spec REQ-OBJ-004
use crate::error::ZodIssueCode;
use crate::schema::{Aux, Schema, SchemaDef};
use crate::value::Value;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownKeysMode {
    Strip,
    Strict,
    Loose,
}

/// Field mask accepted by `.pick(...)`, `.omit(...)`, `.partial_keys(...)` and `.required_keys(...)`:
/// one polymorphic entry point instead of TypeScript's `{ key: true }` objects.
pub trait IntoFieldMask {
    fn into_field_mask(self) -> Vec<String>;
}

impl<const N: usize> IntoFieldMask for [&str; N] {
    fn into_field_mask(self) -> Vec<String> {
        self.into_iter().map(String::from).collect()
    }
}

impl IntoFieldMask for &[&str] {
    fn into_field_mask(self) -> Vec<String> {
        self.iter().map(|s| String::from(*s)).collect()
    }
}

impl IntoFieldMask for Vec<&str> {
    fn into_field_mask(self) -> Vec<String> {
        self.into_iter().map(String::from).collect()
    }
}

impl IntoFieldMask for Vec<String> {
    fn into_field_mask(self) -> Vec<String> {
        self
    }
}

/// Closure-builder target of `z::object(|o| o.field(...))`.
#[derive(Default)]
pub struct ObjectBuilder {
    pub(crate) fields: BTreeMap<String, Schema<Value>>,
}

impl ObjectBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn field<T: 'static>(mut self, name: impl Into<String>, schema: Schema<T>) -> Self {
        self.fields.insert(name.into(), schema.into_type::<Value>());
        self
    }
}

/// Runtime structure of an object schema (kept next to the AST so `.pick/.omit/.extend/...` can rebuild it).
pub struct ObjectShape {
    pub fields: BTreeMap<String, Schema<Value>>,
    pub mode: UnknownKeysMode,
    pub catchall: Option<Schema<Value>>,
}

fn build_object(
    fields: BTreeMap<String, Schema<Value>>,
    mode: UnknownKeysMode,
    catchall: Option<Schema<Value>>,
) -> Schema<Value> {
    let ast_fields: BTreeMap<String, Arc<SchemaDef>> = fields
        .iter()
        .map(|(k, v)| (k.clone(), v.def.clone()))
        .collect();

    let def = SchemaDef::Object {
        fields: ast_fields,
        strict: mode == UnknownKeysMode::Strict,
        loose: mode == UnknownKeysMode::Loose,
        catchall: catchall.as_ref().map(|c| c.def.clone()),
    };

    let shape = Arc::new(ObjectShape {
        fields: fields.clone(),
        mode,
        catchall: catchall.clone(),
    });

    let validators: Vec<(String, crate::schema::ValidatorFn)> = fields
        .iter()
        .map(|(k, v)| (k.clone(), v.validator_clone()))
        .collect();
    let catchall_v = catchall.as_ref().map(|c| c.validator_clone());

    let mut schema = Schema::<Value>::new(def, move |val, ctx| match val {
        Value::Object(input_map) => {
            let mut out = BTreeMap::new();
            let mut unrecognized = Vec::new();

            // 1. Validate declared fields
            for (field_name, field_validator) in &validators {
                ctx.push_key(field_name);
                let present = input_map.get(field_name);
                let field_input = present.unwrap_or(&Value::Null);
                if let Some(valid_field) = field_validator(field_input, ctx) {
                    // Absent optional keys stay absent (as in Zod).
                    if present.is_some() || !valid_field.is_null() {
                        out.insert(field_name.clone(), valid_field);
                    }
                }
                ctx.pop();
            }

            // 2. Handle unknown keys
            for (k, v) in input_map {
                if validators.iter().any(|(name, _)| name == k) {
                    continue;
                }
                if let Some(ref ca) = catchall_v {
                    ctx.push_key(k);
                    if let Some(valid_ca) = ca(v, ctx) {
                        out.insert(k.clone(), valid_ca);
                    }
                    ctx.pop();
                    continue;
                }
                match mode {
                    UnknownKeysMode::Strict => unrecognized.push(k.clone()),
                    UnknownKeysMode::Loose => {
                        out.insert(k.clone(), v.clone());
                    }
                    UnknownKeysMode::Strip => {}
                }
            }

            if !unrecognized.is_empty() {
                ctx.add_issue(ZodIssueCode::UnrecognizedKeys { keys: unrecognized }, None);
            }

            Some(Value::Object(out))
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
    });
    schema.aux = Aux::Object(shape);
    schema
}

// ----------------------------------------------------------------------------
// Constructors
// ----------------------------------------------------------------------------

/// `z::object(|o| ...)`: unknown keys are stripped.
pub fn object<F>(builder_fn: F) -> Schema<Value>
where
    F: FnOnce(ObjectBuilder) -> ObjectBuilder,
{
    build_object(builder_fn(ObjectBuilder::new()).fields, UnknownKeysMode::Strip, None)
}

/// `z::strict_object(|o| ...)`: unknown keys raise `unrecognized_keys`.
pub fn strict_object<F>(builder_fn: F) -> Schema<Value>
where
    F: FnOnce(ObjectBuilder) -> ObjectBuilder,
{
    build_object(builder_fn(ObjectBuilder::new()).fields, UnknownKeysMode::Strict, None)
}

/// `z::loose_object(|o| ...)`: unknown keys are preserved.
pub fn loose_object<F>(builder_fn: F) -> Schema<Value>
where
    F: FnOnce(ObjectBuilder) -> ObjectBuilder,
{
    build_object(builder_fn(ObjectBuilder::new()).fields, UnknownKeysMode::Loose, None)
}

// ----------------------------------------------------------------------------
// Object-only operations (available on any `Schema<T>`; panics on non-object schemas)
// ----------------------------------------------------------------------------

impl<T: 'static> Schema<T> {
    fn object_shape(&self, op: &str) -> Arc<ObjectShape> {
        match &self.aux {
            Aux::Object(shape) => shape.clone(),
            _ => panic!("`.{}()` is only available on object schemas (z::object / strict_object / loose_object)", op),
        }
    }

    /// Reject unknown keys.
    pub fn strict(&self) -> Schema<Value> {
        let s = self.object_shape("strict");
        build_object(s.fields.clone(), UnknownKeysMode::Strict, s.catchall.clone())
    }

    /// Preserve unknown keys (`passthrough` in Zod v3).
    pub fn loose(&self) -> Schema<Value> {
        let s = self.object_shape("loose");
        build_object(s.fields.clone(), UnknownKeysMode::Loose, s.catchall.clone())
    }

    /// Strip unknown keys (default).
    pub fn strip(&self) -> Schema<Value> {
        let s = self.object_shape("strip");
        build_object(s.fields.clone(), UnknownKeysMode::Strip, None)
    }

    /// Validate unknown keys with the given schema instead of stripping/rejecting them.
    pub fn catchall<C: 'static>(&self, schema: Schema<C>) -> Schema<Value> {
        let s = self.object_shape("catchall");
        build_object(s.fields.clone(), s.mode, Some(schema.into_type::<Value>()))
    }

    /// Add or replace fields (`.extend` replaces the deprecated `.merge`).
    pub fn extend<F>(&self, builder_fn: F) -> Schema<Value>
    where
        F: FnOnce(ObjectBuilder) -> ObjectBuilder,
    {
        let s = self.object_shape("extend");
        let mut fields = s.fields.clone();
        for (k, v) in builder_fn(ObjectBuilder::new()).fields {
            fields.insert(k, v);
        }
        build_object(fields, s.mode, s.catchall.clone())
    }

    /// Keep only the listed fields.
    pub fn pick<K: IntoFieldMask>(&self, keys: K) -> Schema<Value> {
        let s = self.object_shape("pick");
        let keys = keys.into_field_mask();
        let fields = s
            .fields
            .iter()
            .filter(|(k, _)| keys.contains(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        build_object(fields, s.mode, s.catchall.clone())
    }

    /// Remove the listed fields.
    pub fn omit<K: IntoFieldMask>(&self, keys: K) -> Schema<Value> {
        let s = self.object_shape("omit");
        let keys = keys.into_field_mask();
        let fields = s
            .fields
            .iter()
            .filter(|(k, _)| !keys.contains(k))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        build_object(fields, s.mode, s.catchall.clone())
    }

    /// Make every field optional.
    pub fn partial(&self) -> Schema<Value> {
        let s = self.object_shape("partial");
        let fields = s
            .fields
            .iter()
            .map(|(k, v)| (k.clone(), v.clone().optional().into_type::<Value>()))
            .collect();
        build_object(fields, s.mode, s.catchall.clone())
    }

    /// Make only the listed fields optional.
    pub fn partial_keys<K: IntoFieldMask>(&self, keys: K) -> Schema<Value> {
        let s = self.object_shape("partial_keys");
        let keys = keys.into_field_mask();
        let fields = s
            .fields
            .iter()
            .map(|(k, v)| {
                if keys.contains(k) {
                    (k.clone(), v.clone().optional().into_type::<Value>())
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect();
        build_object(fields, s.mode, s.catchall.clone())
    }

    /// Make every optional field required again.
    pub fn required(&self) -> Schema<Value> {
        let s = self.object_shape("required");
        let fields = s.fields.iter().map(|(k, v)| (k.clone(), unwrap_optional(v))).collect();
        build_object(fields, s.mode, s.catchall.clone())
    }

    /// Make only the listed optional fields required.
    pub fn required_keys<K: IntoFieldMask>(&self, keys: K) -> Schema<Value> {
        let s = self.object_shape("required_keys");
        let keys = keys.into_field_mask();
        let fields = s
            .fields
            .iter()
            .map(|(k, v)| {
                if keys.contains(k) {
                    (k.clone(), unwrap_optional(v))
                } else {
                    (k.clone(), v.clone())
                }
            })
            .collect();
        build_object(fields, s.mode, s.catchall.clone())
    }

    /// Options schema made of the object's keys.
    pub fn keyof(&self) -> Schema<String> {
        let s = self.object_shape("keyof");
        let keys: Vec<String> = s.fields.keys().cloned().collect();
        crate::z::options(keys)
    }
}

fn unwrap_optional(schema: &Schema<Value>) -> Schema<Value> {
    if matches!(schema.def.strip_meta(), SchemaDef::Optional(_)) {
        schema.clone().unwrap()
    } else {
        schema.clone()
    }
}

// ----------------------------------------------------------------------------
// Records
// ----------------------------------------------------------------------------

/// `z::record(key_schema, value_schema)`: object with arbitrary keys.
pub fn record<K: 'static, V: 'static>(key_schema: Schema<K>, value_schema: Schema<V>) -> Schema<BTreeMap<String, V>> {
    let key_val = key_schema.validator_clone();
    let val_val = value_schema.validator_clone();
    let def = SchemaDef::Record {
        key: key_schema.def.clone(),
        value: value_schema.def.clone(),
    };

    Schema::new(def, move |val, ctx| match val {
        Value::Object(obj) => {
            let mut out = BTreeMap::new();
            for (k, v) in obj {
                ctx.push_key(k);
                let k_val = Value::String((k.clone()).into());
                let valid_k = key_val(&k_val, ctx);
                let valid_v = val_val(v, ctx);

                if let (Some(vk), Some(vv)) = (valid_k, valid_v) {
                    let k_str = match vk {
                        Value::String(s) => s.to_string(),
                        other => format!("{}", other),
                    };
                    out.insert(k_str, vv);
                }
                ctx.pop();
            }
            Some(Value::Object(out))
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "record".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

/// Record whose values are all optional (`z.partialRecord`).
pub fn partial_record<K: 'static, V: 'static>(
    key_schema: Schema<K>,
    value_schema: Schema<V>,
) -> Schema<BTreeMap<String, Option<V>>> {
    record(key_schema, value_schema.optional())
}

/// Record that keeps keys not matching the key schema untouched (`z.looseRecord`).
pub fn loose_record<K: 'static, V: 'static>(key_schema: Schema<K>, value_schema: Schema<V>) -> Schema<BTreeMap<String, V>> {
    let key_val = key_schema.validator_clone();
    let val_val = value_schema.validator_clone();
    let def = SchemaDef::Record {
        key: key_schema.def.clone(),
        value: value_schema.def.clone(),
    };

    Schema::new(def, move |val, ctx| match val {
        Value::Object(obj) => {
            let mut out = BTreeMap::new();
            for (k, v) in obj {
                let mut probe = ctx.fork();
                let k_val = Value::String((k.clone()).into());
                if key_val(&k_val, &mut probe).is_some() && !probe.has_issues() {
                    ctx.push_key(k);
                    if let Some(vv) = val_val(v, ctx) {
                        out.insert(k.to_string(), vv);
                    }
                    ctx.pop();
                } else {
                    out.insert(k.clone(), v.clone());
                }
            }
            Some(Value::Object(out))
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "record".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}
