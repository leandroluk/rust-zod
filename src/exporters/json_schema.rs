// @spec REQ-EXP-001
// @spec REQ-EXP-002
// @spec REQ-EXP-003
use crate::meta::Meta;
use crate::schema::{Check, NumKind, Schema, SchemaDef};
use crate::value::Value;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JsonSchemaTarget {
    #[default]
    Draft202012,
    Draft07,
    Draft04,
    OpenApi30,
    OpenApi31,
}

impl JsonSchemaTarget {
    fn is_openapi(&self) -> bool {
        matches!(self, JsonSchemaTarget::OpenApi30 | JsonSchemaTarget::OpenApi31)
    }

    /// Draft-04 style booleans for exclusive bounds, `nullable`, tuple `items` arrays.
    fn is_legacy(&self) -> bool {
        matches!(
            self,
            JsonSchemaTarget::Draft04 | JsonSchemaTarget::Draft07 | JsonSchemaTarget::OpenApi30
        )
    }
}

/// Which side of a codec / pipe / transform to describe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SchemaIO {
    #[default]
    Output,
    Input,
}

pub type SchemaOverrideFn = Arc<dyn Fn(&SchemaDef, &mut BTreeMap<String, Value>) + Send + Sync>;

#[derive(Clone, Default)]
pub struct JsonSchemaOptions {
    pub target: JsonSchemaTarget,
    pub io: SchemaIO,
    pub override_with: Option<SchemaOverrideFn>,
}

impl JsonSchemaOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn target(mut self, target: JsonSchemaTarget) -> Self {
        self.target = target;
        self
    }

    pub fn io(mut self, io: SchemaIO) -> Self {
        self.io = io;
        self
    }

    pub fn override_with<F>(mut self, hook: F) -> Self
    where
        F: Fn(&SchemaDef, &mut BTreeMap<String, Value>) + Send + Sync + 'static,
    {
        self.override_with = Some(Arc::new(hook));
        self
    }
}

/// Recursion guard for cyclic schemas that carry neither a shared lazy node nor a `meta.id`.
const MAX_DEPTH: usize = 48;

struct Exporter<'a> {
    opts: &'a JsonSchemaOptions,
    defs: BTreeMap<String, Value>,
    /// Ids currently being expanded (cycle breaking).
    in_progress: Vec<String>,
    /// `Lazy` node pointer -> definition name.
    lazy_names: Vec<(usize, String)>,
    anon_counter: usize,
    depth: usize,
}

type Obj = BTreeMap<String, Value>;

fn s(text: &str) -> Value {
    Value::String((text.to_string()).into())
}

fn num(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9.0e15 {
        Value::Int(n as i64)
    } else {
        Value::Float(n)
    }
}

fn regex_escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if "\\^$.|?*+()[]{}".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

impl<'a> Exporter<'a> {
    fn ref_path(&self, name: &str) -> String {
        match self.opts.target {
            JsonSchemaTarget::OpenApi30 | JsonSchemaTarget::OpenApi31 => format!("#/components/schemas/{}", name),
            JsonSchemaTarget::Draft07 | JsonSchemaTarget::Draft04 => format!("#/definitions/{}", name),
            JsonSchemaTarget::Draft202012 => format!("#/$defs/{}", name),
        }
    }

    fn make_ref(&self, name: &str) -> Obj {
        let mut m = Obj::new();
        m.insert("$ref".into(), Value::String((self.ref_path(name)).into()));
        m
    }

    fn apply_meta(out: &mut Obj, meta: &Meta, target: JsonSchemaTarget) {
        if let Some(ref title) = meta.title {
            out.insert("title".into(), Value::String((title.clone()).into()));
        }
        if let Some(ref desc) = meta.description {
            out.insert("description".into(), Value::String((desc.clone()).into()));
        }
        if meta.deprecated {
            out.insert("deprecated".into(), Value::Bool(true));
        }
        if !meta.examples.is_empty() {
            if target == JsonSchemaTarget::Draft04 {
                // Draft-04 has no `examples`.
            } else if target.is_openapi() {
                out.insert("example".into(), meta.examples[0].clone());
            } else {
                out.insert("examples".into(), Value::Array(meta.examples.clone()));
            }
        }
        for (k, v) in &meta.custom {
            out.insert(k.clone(), v.clone());
        }
    }

    fn convert(&mut self, def: &SchemaDef, root: bool) -> Obj {
        if self.depth >= MAX_DEPTH {
            return Obj::new();
        }
        self.depth += 1;
        let mut out = self.convert_node(def, root);
        self.depth -= 1;

        if let Some(ref hook) = self.opts.override_with {
            if !out.contains_key("$ref") {
                hook(def, &mut out);
            }
        }
        out
    }

    fn convert_node(&mut self, def: &SchemaDef, root: bool) -> Obj {
        match def {
            SchemaDef::Meta { inner, meta } => {
                let id = meta.id.clone();
                if let Some(ref id) = id {
                    if !root && (self.in_progress.contains(id) || self.defs.contains_key(id)) {
                        return self.make_ref(id);
                    }
                    self.in_progress.push(id.clone());
                }
                let mut out = self.convert(inner, false);
                if !out.contains_key("$ref") {
                    Self::apply_meta(&mut out, meta, self.opts.target);
                }
                match id {
                    Some(id) => {
                        self.in_progress.retain(|i| i != &id);
                        self.defs.insert(id.clone(), Value::Object(out.clone()));
                        if root {
                            out
                        } else {
                            self.make_ref(&id)
                        }
                    }
                    None => out,
                }
            }
            SchemaDef::Lazy(lazy) => {
                let ptr = Arc::as_ptr(lazy) as usize;
                if let Some((_, name)) = self.lazy_names.iter().find(|(p, _)| *p == ptr) {
                    let name = name.clone();
                    return self.make_ref(&name);
                }
                let inner = lazy.get();
                let name = match inner.def.meta().and_then(|m| m.id.clone()) {
                    Some(id) => id,
                    None => {
                        let n = format!("__schema{}", self.anon_counter);
                        self.anon_counter += 1;
                        n
                    }
                };
                self.lazy_names.push((ptr, name.clone()));
                let had_id = inner.def.meta().and_then(|m| m.id.as_ref()).is_some();
                let json = self.convert(&inner.def, false);
                if !had_id {
                    self.defs.insert(name.clone(), Value::Object(json));
                }
                self.make_ref(&name)
            }
            SchemaDef::Any | SchemaDef::Custom { .. } => Obj::new(),
            SchemaDef::Never => {
                let mut out = Obj::new();
                out.insert("not".into(), Value::Object(Obj::new()));
                out
            }
            SchemaDef::Null => {
                let mut out = Obj::new();
                out.insert("type".into(), s("null"));
                out
            }
            SchemaDef::Boolean => {
                let mut out = Obj::new();
                out.insert("type".into(), s("boolean"));
                out
            }
            SchemaDef::String { format, checks } => self.string_schema(format.as_deref(), checks),
            SchemaDef::Number { kind, checks } => self.number_schema(*kind, checks),
            SchemaDef::BigInt => {
                let mut out = Obj::new();
                out.insert("type".into(), s("integer"));
                out.insert("format".into(), s("int64"));
                out
            }
            SchemaDef::Date => {
                let mut out = Obj::new();
                out.insert("type".into(), s("string"));
                out.insert("format".into(), s("date-time"));
                out
            }
            SchemaDef::Bytes => {
                let mut out = Obj::new();
                out.insert("type".into(), s("string"));
                if self.opts.target.is_openapi() {
                    out.insert("format".into(), s("byte"));
                } else {
                    out.insert("contentEncoding".into(), s("base64"));
                }
                out
            }
            SchemaDef::Literal(values) => {
                let mut out = Obj::new();
                if values.len() == 1 && !self.opts.target.is_legacy() {
                    out.insert("const".into(), values[0].clone());
                } else {
                    out.insert("enum".into(), Value::Array(values.clone()));
                }
                out
            }
            SchemaDef::Array { item, checks } => {
                let mut out = Obj::new();
                out.insert("type".into(), s("array"));
                let item_json = self.convert(item, false);
                out.insert("items".into(), Value::Object(item_json));
                for check in checks {
                    match check {
                        Check::Min { value, .. } => {
                            out.insert("minItems".into(), num(*value));
                        }
                        Check::Max { value, .. } => {
                            out.insert("maxItems".into(), num(*value));
                        }
                        Check::Length(n) => {
                            out.insert("minItems".into(), num(*n as f64));
                            out.insert("maxItems".into(), num(*n as f64));
                        }
                        Check::Unique => {
                            out.insert("uniqueItems".into(), Value::Bool(true));
                        }
                        _ => {}
                    }
                }
                out
            }
            SchemaDef::Tuple { items, rest } => {
                let mut out = Obj::new();
                out.insert("type".into(), s("array"));
                let prefix: Vec<Value> = items
                    .iter()
                    .map(|i| Value::Object(self.convert(i, false)))
                    .collect();
                let rest_json = rest.as_ref().map(|r| Value::Object(self.convert(r, false)));
                if self.opts.target.is_legacy() {
                    out.insert("items".into(), Value::Array(prefix));
                    out.insert("additionalItems".into(), rest_json.unwrap_or(Value::Bool(false)));
                } else {
                    out.insert("prefixItems".into(), Value::Array(prefix));
                    out.insert("items".into(), rest_json.unwrap_or(Value::Bool(false)));
                }
                out.insert("minItems".into(), num(items.len() as f64));
                if rest.is_none() {
                    out.insert("maxItems".into(), num(items.len() as f64));
                }
                out
            }
            SchemaDef::Set(item) => {
                let mut out = Obj::new();
                out.insert("type".into(), s("array"));
                out.insert("uniqueItems".into(), Value::Bool(true));
                let item_json = self.convert(item, false);
                out.insert("items".into(), Value::Object(item_json));
                out
            }
            SchemaDef::Record { key, value } => {
                let mut out = Obj::new();
                out.insert("type".into(), s("object"));
                let key_json = self.convert(key, false);
                if key_json.len() > 1 && self.opts.target == JsonSchemaTarget::Draft202012 {
                    out.insert("propertyNames".into(), Value::Object(key_json));
                }
                let value_json = self.convert(value, false);
                out.insert("additionalProperties".into(), Value::Object(value_json));
                out
            }
            SchemaDef::Optional(inner) => self.convert(inner, false),
            SchemaDef::Nullable(inner) => {
                let inner_json = self.convert(inner, false);
                if self.opts.target == JsonSchemaTarget::OpenApi30 {
                    let mut out = inner_json;
                    out.insert("nullable".into(), Value::Bool(true));
                    out
                } else {
                    let mut out = Obj::new();
                    let mut null_type = Obj::new();
                    null_type.insert("type".into(), s("null"));
                    out.insert(
                        "anyOf".into(),
                        Value::Array(alloc::vec![Value::Object(inner_json), Value::Object(null_type)]),
                    );
                    out
                }
            }
            SchemaDef::Default { inner, value } => {
                let mut out = self.convert(inner, false);
                out.insert("default".into(), value.clone());
                out
            }
            SchemaDef::Readonly(inner) => {
                let mut out = self.convert(inner, false);
                out.insert("readOnly".into(), Value::Bool(true));
                out
            }
            SchemaDef::Object {
                fields,
                strict,
                catchall,
                ..
            } => {
                let mut out = Obj::new();
                out.insert("type".into(), s("object"));
                let mut props = Obj::new();
                let mut required = Vec::new();

                for (name, field_def) in fields {
                    let field_json = self.convert(field_def, false);
                    props.insert(name.clone(), Value::Object(field_json));
                    let stripped = field_def.strip_meta();
                    let optional = match stripped {
                        SchemaDef::Optional(_) => true,
                        SchemaDef::Default { .. } => self.opts.io == SchemaIO::Input,
                        _ => false,
                    };
                    if !optional {
                        required.push(Value::String((name.clone()).into()));
                    }
                }

                out.insert("properties".into(), Value::Object(props));
                if !required.is_empty() {
                    out.insert("required".into(), Value::Array(required));
                }
                if let Some(ca) = catchall {
                    let ca_json = self.convert(ca, false);
                    out.insert("additionalProperties".into(), Value::Object(ca_json));
                } else if *strict {
                    out.insert("additionalProperties".into(), Value::Bool(false));
                }
                out
            }
            SchemaDef::Options(opts_list) => {
                let mut out = Obj::new();
                out.insert("type".into(), s("string"));
                let enums: Vec<Value> = opts_list.iter().map(|v| Value::String((v.clone()).into())).collect();
                out.insert("enum".into(), Value::Array(enums));
                out
            }
            SchemaDef::Union(options) => self.combinator("anyOf", options),
            SchemaDef::Xor(options) => self.combinator("oneOf", options),
            SchemaDef::DiscriminatedUnion { discriminator, options } => {
                let mut out = self.combinator("oneOf", options);
                if self.opts.target.is_openapi() {
                    let mut d = Obj::new();
                    d.insert("propertyName".into(), Value::String((discriminator.clone()).into()));
                    out.insert("discriminator".into(), Value::Object(d));
                }
                out
            }
            SchemaDef::Intersection(left, right) => {
                let mut out = Obj::new();
                let l = self.convert(left, false);
                let r = self.convert(right, false);
                out.insert(
                    "allOf".into(),
                    Value::Array(alloc::vec![Value::Object(l), Value::Object(r)]),
                );
                out
            }
            SchemaDef::Transform(inner) => match self.opts.io {
                SchemaIO::Input => self.convert(inner, false),
                // The output type of a transform is unknowable from the AST.
                SchemaIO::Output => Obj::new(),
            },
            SchemaDef::Pipe { input, output } | SchemaDef::Codec { input, output } => match self.opts.io {
                SchemaIO::Input => self.convert(input, false),
                SchemaIO::Output => self.convert(output, false),
            },
        }
    }

    fn combinator(&mut self, key: &str, options: &[Arc<SchemaDef>]) -> Obj {
        let mut out = Obj::new();
        let items: Vec<Value> = options
            .iter()
            .map(|o| Value::Object(self.convert(o, false)))
            .collect();
        out.insert(key.into(), Value::Array(items));
        out
    }

    fn string_schema(&mut self, format: Option<&str>, checks: &[Check]) -> Obj {
        let mut out = Obj::new();
        out.insert("type".into(), s("string"));

        if let Some(f) = format {
            let json_format = match f {
                "email" => Some("email"),
                "url" | "http_url" => Some("uri"),
                "uuid" | "uuidv4" | "uuidv6" | "uuidv7" | "guid" => Some("uuid"),
                "ipv4" => Some("ipv4"),
                "ipv6" => Some("ipv6"),
                "hostname" => Some("hostname"),
                "iso_date" => Some("date"),
                "iso_time" => Some("time"),
                "iso_datetime" => Some("date-time"),
                "iso_duration" => Some("duration"),
                "base64" => {
                    if self.opts.target.is_openapi() {
                        Some("byte")
                    } else {
                        None
                    }
                }
                _ => None,
            };
            match json_format {
                Some(jf) => {
                    out.insert("format".into(), s(jf));
                }
                None => {
                    // Formats without a JSON Schema equivalent are kept as an annotation.
                    out.insert("x-format".into(), s(f));
                }
            }
        }

        let mut patterns = Vec::new();
        for check in checks {
            match check {
                Check::Min { value, .. } => {
                    out.insert("minLength".into(), num(*value));
                }
                Check::Max { value, .. } => {
                    out.insert("maxLength".into(), num(*value));
                }
                Check::Length(n) => {
                    out.insert("minLength".into(), num(*n as f64));
                    out.insert("maxLength".into(), num(*n as f64));
                }
                Check::Pattern(p) => patterns.push(p.clone()),
                Check::StartsWith(p) => patterns.push(format!("^{}", regex_escape(p))),
                Check::EndsWith(p) => patterns.push(format!("{}$", regex_escape(p))),
                Check::Includes(p) => patterns.push(regex_escape(p)),
                Check::Lowercase => patterns.push("^[^A-Z]*$".to_string()),
                Check::Uppercase => patterns.push("^[^a-z]*$".to_string()),
                _ => {}
            }
        }
        match patterns.len() {
            0 => {}
            1 => {
                out.insert("pattern".into(), Value::String((patterns.remove(0)).into()));
            }
            _ => {
                let all: Vec<Value> = patterns
                    .into_iter()
                    .map(|p| {
                        let mut m = Obj::new();
                        m.insert("pattern".into(), Value::String((p).into()));
                        Value::Object(m)
                    })
                    .collect();
                out.insert("allOf".into(), Value::Array(all));
            }
        }
        out
    }

    fn number_schema(&mut self, kind: NumKind, checks: &[Check]) -> Obj {
        let mut out = Obj::new();
        out.insert("type".into(), s(if kind.is_integer() { "integer" } else { "number" }));
        match kind {
            NumKind::Int32 => {
                out.insert("minimum".into(), Value::Int(i32::MIN as i64));
                out.insert("maximum".into(), Value::Int(i32::MAX as i64));
            }
            NumKind::Uint32 => {
                out.insert("minimum".into(), Value::Int(0));
                out.insert("maximum".into(), Value::Int(u32::MAX as i64));
            }
            NumKind::Uint64 => {
                out.insert("minimum".into(), Value::Int(0));
            }
            NumKind::Int64 => {
                out.insert("format".into(), s("int64"));
            }
            NumKind::Float32 => {
                out.insert("format".into(), s("float"));
            }
            _ => {}
        }

        let legacy = matches!(self.opts.target, JsonSchemaTarget::Draft04 | JsonSchemaTarget::OpenApi30);
        for check in checks {
            match check {
                Check::Min { value, inclusive } => {
                    if *inclusive {
                        out.insert("minimum".into(), num(*value));
                    } else if legacy {
                        out.insert("minimum".into(), num(*value));
                        out.insert("exclusiveMinimum".into(), Value::Bool(true));
                    } else {
                        out.insert("exclusiveMinimum".into(), num(*value));
                    }
                }
                Check::Max { value, inclusive } => {
                    if *inclusive {
                        out.insert("maximum".into(), num(*value));
                    } else if legacy {
                        out.insert("maximum".into(), num(*value));
                        out.insert("exclusiveMaximum".into(), Value::Bool(true));
                    } else {
                        out.insert("exclusiveMaximum".into(), num(*value));
                    }
                }
                Check::MultipleOf(d) => {
                    out.insert("multipleOf".into(), num(*d));
                }
                _ => {}
            }
        }
        out
    }
}

/// Export any `Schema` to a JSON Schema or OpenAPI document as a `Value::Object`.
///
/// Cycles (via `z::lazy`) are emitted as `$ref`s: a shared lazy node (one defined by a capture-free
/// closure / fn item) or a `meta.id` on the recursive schema names the definition.
pub fn to_json_schema<T>(schema: &Schema<T>, options: JsonSchemaOptions) -> Value {
    let mut exporter = Exporter {
        opts: &options,
        defs: BTreeMap::new(),
        in_progress: Vec::new(),
        lazy_names: Vec::new(),
        anon_counter: 0,
        depth: 0,
    };
    let mut root_schema = exporter.convert(&schema.def, true);
    let defs = core::mem::take(&mut exporter.defs);

    if !defs.is_empty() {
        match options.target {
            JsonSchemaTarget::Draft202012 => {
                root_schema.insert("$defs".into(), Value::Object(defs));
            }
            JsonSchemaTarget::Draft07 | JsonSchemaTarget::Draft04 => {
                root_schema.insert("definitions".into(), Value::Object(defs));
            }
            JsonSchemaTarget::OpenApi30 | JsonSchemaTarget::OpenApi31 => {
                let mut components = Obj::new();
                components.insert("schemas".into(), Value::Object(defs));
                root_schema.insert("components".into(), Value::Object(components));
            }
        }
    }

    match options.target {
        JsonSchemaTarget::Draft202012 => {
            root_schema.insert("$schema".into(), s("https://json-schema.org/draft/2020-12/schema"));
        }
        JsonSchemaTarget::Draft07 => {
            root_schema.insert("$schema".into(), s("http://json-schema.org/draft-07/schema#"));
        }
        JsonSchemaTarget::Draft04 => {
            root_schema.insert("$schema".into(), s("http://json-schema.org/draft-04/schema#"));
        }
        _ => {}
    }

    Value::Object(root_schema)
}

// ----------------------------------------------------------------------------
// Reverse Importer: from_json_schema
// ----------------------------------------------------------------------------

/// Reconstruct a dynamic `rust-zod` Schema from a JSON Schema / OpenAPI document.
///
/// Supports `type` (incl. arrays), `enum`, `const`, `anyOf` / `oneOf` / `allOf`, `$ref` into
/// `$defs` / `definitions` / `components.schemas` (recursive refs become `z::lazy`), formats,
/// string / number / array constraints, `properties` + `required`, `additionalProperties`,
/// `prefixItems` / tuple `items`, `nullable`, `default`, and `title` / `description` / `deprecated`.
pub fn from_json_schema(json: &Value) -> Schema<Value> {
    let doc = Arc::new(json.clone());
    import(&doc, json, 0)
}

const MAX_IMPORT_DEPTH: usize = 64;

fn resolve_ref<'a>(doc: &'a Value, reference: &str) -> Option<&'a Value> {
    let path = reference.strip_prefix("#/")?;
    let mut cur = doc;
    for part in path.split('/') {
        let part = part.replace("~1", "/").replace("~0", "~");
        cur = cur.as_object()?.get(&part)?;
    }
    Some(cur)
}

fn as_f64(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn import(doc: &Arc<Value>, node: &Value, depth: usize) -> Schema<Value> {
    use crate::z;

    if depth > MAX_IMPORT_DEPTH {
        return z::any();
    }
    let map = match node {
        Value::Object(map) => map,
        Value::Bool(false) => return z::never(),
        _ => return z::any(),
    };

    // $ref: resolve lazily so recursive documents terminate.
    if let Some(reference) = map.get("$ref").and_then(|r| r.as_str()) {
        if resolve_ref(doc, reference).is_none() {
            return z::any();
        }
        let doc = doc.clone();
        let reference = reference.to_string();
        return z::lazy(move || {
            let target = resolve_ref(&doc, &reference).cloned().unwrap_or(Value::Null);
            import(&doc, &target, 0)
        });
    }

    let mut schema = import_structure(doc, map, depth);

    if map.get("nullable").and_then(|n| n.as_bool()) == Some(true) {
        schema = schema.nullable().into_type::<Value>();
    }
    if let Some(default) = map.get("default") {
        schema = schema.default(default.clone());
    }

    let title = map.get("title").and_then(|t| t.as_str()).map(String::from);
    let description = map.get("description").and_then(|t| t.as_str()).map(String::from);
    let deprecated = map.get("deprecated").and_then(|d| d.as_bool()).unwrap_or(false);
    if title.is_some() || description.is_some() || deprecated {
        schema = schema.meta(|mut m| {
            if let Some(t) = title {
                m = m.title(t);
            }
            if let Some(d) = description {
                m = m.description(d);
            }
            m.deprecated(deprecated)
        });
    }
    schema
}

fn import_structure(doc: &Arc<Value>, map: &BTreeMap<String, Value>, depth: usize) -> Schema<Value> {
    use crate::z;

    if let Some(values) = map.get("const") {
        return z::literal(values.clone());
    }
    if let Some(values) = map.get("enum").and_then(|e| e.as_array()) {
        if values.iter().all(|v| v.as_str().is_some()) {
            let opts: Vec<String> = values.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            return z::options(opts).into_type::<Value>();
        }
        return z::literal(values.clone());
    }
    for (key, xor) in [("anyOf", false), ("oneOf", true)] {
        if let Some(options) = map.get(key).and_then(|a| a.as_array()) {
            let schemas: Vec<Schema<Value>> = options.iter().map(|o| import(doc, o, depth + 1)).collect();
            return if xor && schemas.len() > 1 {
                z::xor(schemas)
            } else {
                z::union(schemas)
            };
        }
    }
    if let Some(parts) = map.get("allOf").and_then(|a| a.as_array()) {
        let mut iter = parts.iter().map(|p| import(doc, p, depth + 1));
        if let Some(first) = iter.next() {
            return iter.fold(first, z::intersection);
        }
    }
    if map.get("not").is_some_and(|n| n.as_object().is_some_and(|o| o.is_empty())) {
        return z::never();
    }

    match map.get("type") {
        Some(Value::Array(types)) => {
            let schemas: Vec<Schema<Value>> = types
                .iter()
                .filter_map(|t| t.as_str())
                .map(|t| {
                    let mut single = map.clone();
                    single.insert("type".into(), Value::String((t.to_string()).into()));
                    import_structure(doc, &single, depth + 1)
                })
                .collect();
            z::union(schemas)
        }
        Some(Value::String(t)) => match t.as_str() {
            "string" => import_string(map),
            "number" => import_number(map, false),
            "integer" => import_number(map, true),
            "boolean" => z::boolean().into_type::<Value>(),
            "null" => z::null().into_type::<Value>(),
            "array" => import_array(doc, map, depth),
            "object" => import_object(doc, map, depth),
            _ => z::any(),
        },
        _ => {
            if map.contains_key("properties") {
                import_object(doc, map, depth)
            } else {
                z::any()
            }
        }
    }
}

fn import_string(map: &BTreeMap<String, Value>) -> Schema<Value> {
    use crate::z;

    let mut schema = match map.get("format").and_then(|f| f.as_str()) {
        Some("email") => z::email(),
        Some("uuid") => z::uuid(),
        Some("uri") | Some("url") => z::url(),
        Some("date-time") => z::iso::datetime(),
        Some("date") => z::iso::date(),
        Some("time") => z::iso::time(),
        Some("duration") => z::iso::duration(),
        Some("ipv4") => z::ipv4(),
        Some("ipv6") => z::ipv6(),
        Some("hostname") => z::hostname(),
        Some("byte") => z::base64(),
        _ => z::string(),
    };
    if let Some(n) = map.get("minLength").and_then(as_f64) {
        schema = schema.min(n as usize);
    }
    if let Some(n) = map.get("maxLength").and_then(as_f64) {
        schema = schema.max(n as usize);
    }
    #[cfg(feature = "regex")]
    if let Some(p) = map.get("pattern").and_then(|p| p.as_str()) {
        if regex::Regex::new(p).is_ok() {
            schema = schema.regex(p);
        }
    }
    schema.into_type::<Value>()
}

fn import_number(map: &BTreeMap<String, Value>, integer: bool) -> Schema<Value> {
    use crate::z;

    macro_rules! constrain {
        ($schema:expr) => {{
            let mut sch = $schema;
            if let Some(v) = map.get("minimum").and_then(as_f64) {
                let exclusive = map.get("exclusiveMinimum").and_then(|e| e.as_bool()) == Some(true);
                sch = if exclusive { sch.gt(v as _) } else { sch.gte(v as _) };
            }
            if let Some(v) = map.get("exclusiveMinimum").and_then(as_f64) {
                sch = sch.gt(v as _);
            }
            if let Some(v) = map.get("maximum").and_then(as_f64) {
                let exclusive = map.get("exclusiveMaximum").and_then(|e| e.as_bool()) == Some(true);
                sch = if exclusive { sch.lt(v as _) } else { sch.lte(v as _) };
            }
            if let Some(v) = map.get("exclusiveMaximum").and_then(as_f64) {
                sch = sch.lt(v as _);
            }
            if let Some(v) = map.get("multipleOf").and_then(as_f64) {
                sch = sch.multiple_of(v as _);
            }
            sch.into_type::<Value>()
        }};
    }

    if integer {
        constrain!(z::int())
    } else {
        constrain!(z::number())
    }
}

fn import_array(doc: &Arc<Value>, map: &BTreeMap<String, Value>, depth: usize) -> Schema<Value> {
    use crate::z;

    let prefix = map.get("prefixItems").and_then(|p| p.as_array()).cloned().or_else(|| {
        map.get("items").and_then(|i| i.as_array()).cloned()
    });

    if let Some(prefix) = prefix {
        let items: Vec<Schema<Value>> = prefix.iter().map(|p| import(doc, p, depth + 1)).collect();
        let tuple = z::tuple(items);
        let rest_node = if map.contains_key("prefixItems") {
            map.get("items")
        } else {
            map.get("additionalItems")
        };
        return match rest_node {
            Some(r) if r.as_object().is_some() => tuple.rest(import(doc, r, depth + 1)).into_type::<Value>(),
            _ => tuple.into_type::<Value>(),
        };
    }

    let item = map
        .get("items")
        .map(|i| import(doc, i, depth + 1))
        .unwrap_or_else(z::any);
    let mut schema = z::array(item);
    if let Some(n) = map.get("minItems").and_then(as_f64) {
        schema = schema.min(n as usize);
    }
    if let Some(n) = map.get("maxItems").and_then(as_f64) {
        schema = schema.max(n as usize);
    }
    if map.get("uniqueItems").and_then(|u| u.as_bool()) == Some(true) {
        schema = schema.unique();
    }
    schema.into_type::<Value>()
}

fn import_object(doc: &Arc<Value>, map: &BTreeMap<String, Value>, depth: usize) -> Schema<Value> {
    use crate::z;

    let additional = map.get("additionalProperties");
    let props = map.get("properties").and_then(|p| p.as_object());

    // Pure dictionary: `additionalProperties` schema and no declared properties -> record.
    if props.is_none_or(|p| p.is_empty()) {
        if let Some(ap) = additional.filter(|a| a.as_object().is_some()) {
            return z::record(z::string(), import(doc, ap, depth + 1)).into_type::<Value>();
        }
    }

    let required: Vec<&str> = map
        .get("required")
        .and_then(|r| r.as_array())
        .map(|r| r.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut fields = Vec::new();
    if let Some(props) = props {
        for (name, prop) in props {
            let sub = import(doc, prop, depth + 1);
            let sub = if required.contains(&name.as_str()) {
                sub
            } else {
                sub.optional().into_type::<Value>()
            };
            fields.push((name.clone(), sub));
        }
    }
    let obj = z::object(|mut o| {
        for (name, sub) in fields {
            o = o.field(name, sub);
        }
        o
    });

    match additional {
        Some(Value::Bool(false)) => obj.strict(),
        Some(a) if a.as_object().is_some() => obj.catchall(import(doc, a, depth + 1)),
        Some(Value::Bool(true)) => obj.loose(),
        _ => obj,
    }
}
