extern crate alloc;

pub mod error;
pub mod exporters;
pub mod i18n;
pub mod meta;
pub mod schema;
pub mod types;
pub mod value;

// Re-exports for convenient top-level access
pub use error::{
    Failure, FlattenedError, FormattedError, PathSegment, SizeOrigin, TreeifiedError, ZodError,
    ZodIssue, ZodIssueCode,
};
pub use exporters::{
    from_json_schema, to_graphql_sdl, to_json_schema, GraphqlOptions, GraphqlTarget,
    JsonSchemaOptions, JsonSchemaTarget, SchemaIO,
};
pub use i18n::Locale;
pub use meta::{Meta, MetaBuilder};
pub use schema::{
    Check, CheckOpts, Direction, IntoParseOptions, LazyDef, NumKind, ParseContext, ParseOptions,
    Schema, SchemaDef,
};
pub use types::{IntoFieldMask, IntoOptions, IntoTuple, Numeric};
pub use value::Value;

/// Top-level `z` facade mirroring the Zod v4 namespace.
pub mod z {
    use super::*;
    use alloc::string::String;
    use alloc::vec::Vec;

    /// Set the global default validation locale (e.g., `Locale::PtBr`, `Locale::En`, `Locale::Es`).
    pub fn set_locale(locale: Locale) {
        i18n::set_global_locale(locale);
    }

    /// Get the current global default validation locale.
    pub fn get_locale() -> Locale {
        i18n::get_global_locale()
    }

    // Primitives
    pub use types::primitives::{bytes, date, json, literal, null, unknown, IntoLiterals};

    pub fn any() -> Schema<Value> {
        Schema::new(SchemaDef::Any, |val, _ctx| Some(val.clone()))
    }

    pub fn never() -> Schema<Value> {
        Schema::new(SchemaDef::Never, |val, ctx| {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "never".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        })
    }

    pub fn boolean() -> Schema<bool> {
        Schema::new(SchemaDef::Boolean, |val, ctx| match val {
            Value::Bool(b) => Some(Value::Bool(*b)),
            _ => {
                ctx.add_issue(
                    ZodIssueCode::InvalidType {
                        expected: "boolean".into(),
                        received: val.type_name().into(),
                    },
                    None,
                );
                None
            }
        })
    }

    pub fn string() -> Schema<String> {
        Schema::new(SchemaDef::string(), |val, ctx| match val {
            Value::String(s) => Some(Value::String(s.clone())),
            _ => {
                ctx.add_issue(
                    ZodIssueCode::InvalidType {
                        expected: "string".into(),
                        received: val.type_name().into(),
                    },
                    None,
                );
                None
            }
        })
    }

    // Top-Level String Formats
    pub use types::string::{
        base64, base64url, cidrv4, cidrv6, credit_card, cuid2, currency_code, e164, email, emoji,
        guid, hash, hex, hostname, http_url, iban, ipv4, ipv6, jwt, ksuid, mac, nanoid,
        string_format, ulid, url, uuid, uuidv4, uuidv6, uuidv7, xid,
    };

    pub mod iso {
        pub use crate::types::string::iso::*;
    }

    // Top-Level Numeric Types
    pub use types::number::{bigint, float32, float64, int, int32, int64, uint32, uint64};

    pub fn number() -> Schema<f64> {
        float64()
    }

    // Coercion
    pub mod coerce {
        pub use crate::types::coerce::*;
    }

    // Combinators, Options, Collections & Codecs
    pub use types::codec::{codec, codec_async, codecs, stringbool, StringBoolCodec};
    pub use types::collections::{map, set, tuple};
    pub use types::combinators::{discriminated_union, intersection, lazy, union, xor};
    pub use types::object::{
        loose_object, loose_record, object, partial_record, record, strict_object, ObjectBuilder,
    };

    pub fn array<T: 'static>(element_schema: Schema<T>) -> Schema<Vec<T>> {
        let inner_validator = element_schema.validator_clone();
        let def = SchemaDef::Array {
            item: element_schema.def.clone(),
            checks: Vec::new(),
        };

        Schema::new(def, move |val, ctx| match val {
            Value::Array(items) => {
                let mut valid_items = Vec::with_capacity(items.len());
                for (idx, item) in items.iter().enumerate() {
                    ctx.push_index(idx);
                    if let Some(res) = inner_validator(item, ctx) {
                        valid_items.push(res);
                    }
                    ctx.pop();
                }
                Some(Value::Array(valid_items))
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
        })
    }

    pub use types::IntoOptions;

    pub fn options<O: IntoOptions>(opts: O) -> Schema<String> {
        let allowed = opts.into_options();
        let allowed_clone = allowed.clone();
        let def = SchemaDef::Options(allowed.clone());

        Schema::new(def, move |val, ctx| match val {
            Value::String(s) if allowed_clone.iter().any(|a| a == s.as_str()) => {
                Some(Value::String(s.clone()))
            }
            _ => {
                ctx.add_issue(
                    ZodIssueCode::InvalidValue {
                        values: allowed_clone.clone(),
                    },
                    None,
                );
                None
            }
        })
    }

    pub fn custom<T: 'static>(
        name: impl Into<String>,
        validator: impl Fn(&Value, &mut ParseContext) -> Option<Value> + Send + Sync + 'static,
    ) -> Schema<T> {
        Schema::new(SchemaDef::Custom { name: name.into() }, validator)
    }

    // Exporters & Importer
    pub use exporters::{from_json_schema, to_graphql_sdl, to_json_schema};
}
