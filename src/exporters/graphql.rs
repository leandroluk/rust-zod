// @spec REQ-GQL-001
// @spec REQ-GQL-002
// @spec REQ-GQL-003
use crate::meta::Meta;
use crate::schema::{NumKind, Schema, SchemaDef};
use crate::value::Value;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphqlTarget {
    #[default]
    Type,
    Input,
}

#[derive(Clone, Default)]
pub struct GraphqlOptions {
    pub target: GraphqlTarget,
    pub type_name: Option<String>,
}

impl GraphqlOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn target(mut self, target: GraphqlTarget) -> Self {
        self.target = target;
        self
    }

    pub fn type_name(mut self, name: impl Into<String>) -> Self {
        self.type_name = Some(name.into());
        self
    }
}

fn pascal_case(text: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if upper {
                out.extend(c.to_uppercase());
            } else {
                out.push(c);
            }
            upper = false;
        } else {
            upper = true;
        }
    }
    out
}

fn gql_name(text: &str) -> String {
    let mut out: String = text
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if out.is_empty() || out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn doc_block(meta: Option<&Meta>, indent: &str) -> String {
    match meta.and_then(|m| m.description.as_ref()) {
        Some(desc) => format!("{indent}\"\"\"\n{indent}{desc}\n{indent}\"\"\"\n"),
        None => String::new(),
    }
}

struct Gql {
    target: GraphqlTarget,
    /// Definition slots in creation order (`None` while a type is still being built).
    defs: Vec<(String, Option<String>)>,
    scalars: BTreeSet<&'static str>,
    lazy_names: Vec<(usize, String)>,
    depth: usize,
}

const MAX_DEPTH: usize = 48;

impl Gql {
    fn reserve(&mut self, name: &str) -> bool {
        if self.defs.iter().any(|(n, _)| n == name) {
            return false;
        }
        self.defs.push((name.to_string(), None));
        true
    }

    fn fill(&mut self, name: &str, sdl: String) {
        if let Some(slot) = self.defs.iter_mut().find(|(n, _)| n == name) {
            slot.1 = Some(sdl);
        }
    }

    fn scalar(&mut self, name: &'static str) -> (String, bool) {
        self.scalars.insert(name);
        (name.to_string(), true)
    }

    /// Resolve a definition to `(type reference, required)`; nested named types are registered.
    fn type_ref(&mut self, def: &SchemaDef, hint: &str) -> (String, bool) {
        if self.depth >= MAX_DEPTH {
            return self.scalar("JSON");
        }
        self.depth += 1;
        let out = self.type_ref_inner(def, hint, None);
        self.depth -= 1;
        out
    }

    fn type_ref_inner(&mut self, def: &SchemaDef, hint: &str, meta: Option<&Meta>) -> (String, bool) {
        match def {
            SchemaDef::Meta { inner, meta } => {
                let hint = meta.id.clone().unwrap_or_else(|| hint.to_string());
                self.type_ref_inner(inner, &hint, Some(meta))
            }
            SchemaDef::Optional(inner) | SchemaDef::Nullable(inner) => {
                let (t, _) = self.type_ref(inner, hint);
                (t, false)
            }
            SchemaDef::Default { inner, .. } => {
                let (t, _) = self.type_ref(inner, hint);
                (t, self.target == GraphqlTarget::Type)
            }
            SchemaDef::Readonly(inner) => self.type_ref(inner, hint),
            SchemaDef::String { format, .. } => match format.as_deref() {
                Some("uuid") | Some("uuidv4") | Some("uuidv6") | Some("uuidv7") | Some("guid") => {
                    ("ID".to_string(), true)
                }
                _ => ("String".to_string(), true),
            },
            SchemaDef::Number { kind, .. } => match kind {
                NumKind::Float64 | NumKind::Float32 => ("Float".to_string(), true),
                NumKind::Int | NumKind::Int32 | NumKind::Uint32 => ("Int".to_string(), true),
                NumKind::Int64 | NumKind::Uint64 => self.scalar("BigInt"),
            },
            SchemaDef::BigInt => self.scalar("BigInt"),
            SchemaDef::Boolean => ("Boolean".to_string(), true),
            SchemaDef::Date => self.scalar("DateTime"),
            SchemaDef::Bytes => self.scalar("Bytes"),
            SchemaDef::Literal(values) => match values.first() {
                Some(Value::Bool(_)) => ("Boolean".to_string(), true),
                Some(Value::Int(_)) | Some(Value::Uint(_)) => ("Int".to_string(), true),
                Some(Value::Float(_)) => ("Float".to_string(), true),
                _ => ("String".to_string(), true),
            },
            SchemaDef::Array { item, .. } | SchemaDef::Set(item) => {
                let (t, req) = self.type_ref(item, hint);
                (format!("[{}{}]", t, if req { "!" } else { "" }), true)
            }
            SchemaDef::Tuple { .. } => {
                self.scalars.insert("JSON");
                ("[JSON!]".to_string(), true)
            }
            SchemaDef::Options(values) => {
                let name = gql_name(&pascal_case(hint));
                if self.reserve(&name) {
                    let mut sdl = doc_block(meta, "");
                    sdl.push_str(&format!("enum {} {{\n", name));
                    for v in values {
                        sdl.push_str(&format!("  {}\n", gql_name(v)));
                    }
                    sdl.push('}');
                    self.fill(&name, sdl);
                }
                (name, true)
            }
            SchemaDef::Object { .. } => {
                let name = gql_name(&pascal_case(hint));
                self.define_object(&name, def, meta);
                (name, true)
            }
            SchemaDef::Intersection(l, r) => {
                let mut fields = BTreeMap::new();
                if let (Some(lf), Some(rf)) = (object_fields(l), object_fields(r)) {
                    fields.extend(lf);
                    fields.extend(rf);
                    let merged = SchemaDef::Object {
                        fields,
                        strict: false,
                        loose: false,
                        catchall: None,
                    };
                    let name = gql_name(&pascal_case(hint));
                    self.define_object(&name, &merged, meta);
                    (name, true)
                } else {
                    self.scalar("JSON")
                }
            }
            SchemaDef::Union(options) | SchemaDef::Xor(options) | SchemaDef::DiscriminatedUnion { options, .. } => {
                self.define_union(hint, options, meta)
            }
            SchemaDef::Lazy(lazy) => {
                let ptr = Arc::as_ptr(lazy) as usize;
                if let Some((_, name)) = self.lazy_names.iter().find(|(p, _)| *p == ptr) {
                    return (name.clone(), true);
                }
                let inner = lazy.get();
                let name = gql_name(&pascal_case(
                    &inner.def.meta().and_then(|m| m.id.clone()).unwrap_or_else(|| hint.to_string()),
                ));
                self.lazy_names.push((ptr, name.clone()));
                self.type_ref(&inner.def, &name)
            }
            SchemaDef::Pipe { input, output } | SchemaDef::Codec { input, output } => match self.target {
                GraphqlTarget::Type => self.type_ref(output, hint),
                GraphqlTarget::Input => self.type_ref(input, hint),
            },
            SchemaDef::Any
            | SchemaDef::Never
            | SchemaDef::Null
            | SchemaDef::Record { .. }
            | SchemaDef::Transform(_)
            | SchemaDef::Custom { .. } => self.scalar("JSON"),
        }
    }

    fn define_object(&mut self, name: &str, def: &SchemaDef, meta: Option<&Meta>) {
        if !self.reserve(name) {
            return;
        }
        let fields = match def.strip_meta() {
            SchemaDef::Object { fields, .. } => fields,
            _ => return,
        };
        let keyword = match self.target {
            GraphqlTarget::Type => "type",
            GraphqlTarget::Input => "input",
        };
        let mut sdl = doc_block(meta, "");
        sdl.push_str(&format!("{} {} {{\n", keyword, name));
        for (field_name, field_def) in fields {
            let hint = format!("{}{}", name, pascal_case(field_name));
            let (t, req) = self.type_ref(field_def, &hint);
            sdl.push_str(&format!(
                "  {}: {}{}\n",
                gql_name(field_name),
                t,
                if req { "!" } else { "" }
            ));
        }
        sdl.push('}');
        self.fill(name, sdl);
    }

    fn define_union(&mut self, hint: &str, options: &[Arc<SchemaDef>], meta: Option<&Meta>) -> (String, bool) {
        // GraphQL input types cannot be unions, and union members must be object types.
        let all_objects = options.iter().all(|o| matches!(o.strip_meta(), SchemaDef::Object { .. }));
        if self.target == GraphqlTarget::Input || !all_objects {
            return self.scalar("JSON");
        }
        let name = gql_name(&pascal_case(hint));
        if self.reserve(&name) {
            let mut members = Vec::new();
            for (i, option) in options.iter().enumerate() {
                let member_hint = match option.meta().and_then(|m| m.id.clone()) {
                    Some(id) => id,
                    None => format!("{}{}", name, i + 1),
                };
                let (t, _) = self.type_ref(option, &member_hint);
                members.push(t);
            }
            let mut sdl = doc_block(meta, "");
            sdl.push_str(&format!("union {} = {}", name, members.join(" | ")));
            self.fill(&name, sdl);
        }
        (name, true)
    }
}

fn object_fields(def: &SchemaDef) -> Option<BTreeMap<String, Arc<SchemaDef>>> {
    match def.strip_meta() {
        SchemaDef::Object { fields, .. } => Some(fields.clone()),
        _ => None,
    }
}

/// Generate GraphQL SDL (the root definition plus every nested named type and custom scalar)
/// from any `rust-zod` Schema.
///
/// Nullability is inverted from Zod: fields are `!` unless `.optional()` / `.nullable()`.
/// Named types come from `.meta(|m| m.id("..."))` or are inferred in PascalCase from the path
/// (`UserAddress`). Unions become `union` (objects only), anything else JSON-like becomes `scalar JSON`.
pub fn to_graphql_sdl<T>(schema: &Schema<T>, options: GraphqlOptions) -> String {
    let root_name = options
        .type_name
        .clone()
        .or_else(|| schema.meta.id.clone())
        .or_else(|| schema.def.meta().and_then(|m| m.id.clone()))
        .unwrap_or_else(|| "Anonymous".into());
    let root_name = gql_name(&root_name);

    let mut gql = Gql {
        target: options.target,
        defs: Vec::new(),
        scalars: BTreeSet::new(),
        lazy_names: Vec::new(),
        depth: 0,
    };

    let root_def = schema.def.strip_meta();
    let meta = schema.def.meta();
    let named_root = matches!(
        root_def,
        SchemaDef::Object { .. }
            | SchemaDef::Options(_)
            | SchemaDef::Union(_)
            | SchemaDef::Xor(_)
            | SchemaDef::DiscriminatedUnion { .. }
            | SchemaDef::Intersection(..)
            | SchemaDef::Lazy(_)
    );

    let mut out = String::new();
    if named_root {
        let hint = root_name.clone();
        gql.type_ref_inner(root_def, &hint, meta);
        for (_, sdl) in &gql.defs {
            if let Some(sdl) = sdl {
                out.push_str(sdl);
                out.push_str("\n\n");
            }
        }
    } else {
        let (t, req) = gql.type_ref(&schema.def, &root_name);
        out.push_str(&format!("{}{}", t, if req { "!" } else { "" }));
        out.push_str("\n\n");
        for (_, sdl) in &gql.defs {
            if let Some(sdl) = sdl {
                out.push_str(sdl);
                out.push_str("\n\n");
            }
        }
    }
    for scalar in &gql.scalars {
        out.push_str(&format!("scalar {}\n\n", scalar));
    }
    out.trim_end().to_string()
}
