# Spec: 01 — Core Value Model, Metadata Builder, Error Formatting & i18n

## Summary
Establishes the `#![no_std]` (`alloc`) foundation of `rust-zod` aligned with [INSIGHT.md](../../../INSIGHT.md) and `colinhacks/zod` (`src/v4/core`): the runtime `Value` representation (including `Date(i64)` and `Bytes(Vec<u8>)` for domain types and codecs), the closure-based `.meta(|m| ...)` builder (replacing `.describe()`), the 11-variant Zod v4 issue taxonomy (`ZodIssue`, `ZodIssueCode`, `ZodError`), all 4 Zod v4 error formatters (`flatten`, `format`, `treeify`, `prettify`), the internationalized message system (`Locale`, built-in `en`, `pt-BR`, `es` catalogs with dynamic global/per-parse locale switching), and the `Arc<SchemaDef>` Copy-on-Write execution engine supporting `.parse()`, `.parse_async()`, `.decode()`, `.encode()`, `.decode_async()`, and `.encode_async()` returning native Rust `Result` (eliminating `safe_parse`/`spa` redundancy).

## Requirements
- REQ-CORE-001: Provide a runtime `Value` enum (`Null`, `Bool`, `Int`, `Uint`, `Float`, `String`, `Bytes`, `Date`, `Array`, `Object`) compatible with `#![no_std]` + `alloc`, with `From` / `TryFrom` conversions, `serde` deserialization helper (`parse_into::<T>()`), and optional `serde_json::Value` interoperability.
- REQ-CORE-002: Implement the Zod v4 11-variant issue taxonomy (`ZodIssueCode`): `InvalidType`, `TooBig`, `TooSmall`, `InvalidFormat`, `NotMultipleOf`, `UnrecognizedKeys`, `InvalidUnion`, `InvalidKey`, `InvalidElement`, `InvalidValue`, and `Custom`.
- REQ-CORE-003: Implement `ZodIssue` (code, path segments `Key`/`Index`, input summary, resolved message) and `ZodError` containing the collected issues with path tracking during nested validation.
- REQ-CORE-004: Provide all 4 Zod v4 error formatting utilities on `ZodError`: `.flatten()`, `.format()`, `.treeify()`, and `.prettify()` (`✖ message \n  → at path`).
- REQ-CORE-005: Provide default validation messages out of the box for every `ZodIssueCode`, sizing origin (`string`, `array`, `set`, `map`, `file`/`bytes`), and string format without requiring custom error strings.
- REQ-CORE-006: Implement the i18n locale system (`Locale` catalog) with built-in locales (`en`, `pt-BR`, `es` modeled on `v4/locales/`) supporting global (`z::set_locale(...)`) and per-parse (`parse_with(&val, |opt| opt.locale(...))`) switching, plus custom check/schema error overrides (`.error(...)`).
- REQ-CORE-007: Implement the closure-based metadata builder `.meta(|m| m.id(...).title(...).description(...).example(...).deprecated(...).custom(k, v))` on `Schema` (eliminating `.describe()`).
- REQ-CORE-008: Define the introspectable `Arc<SchemaDef>` AST with `Arc::make_mut` Copy-on-Write builders and runtime-agnostic execution (`.parse(&Value) -> Result<Value, ZodError>`, `.parse_async(&Value)`, `.decode(&Value)`, `.encode(&Value)`), adhering to strict `target/` disk footprint limits (`debug = 0`, `strip = "debuginfo"`, `incremental = false`, zero proc-macros, zero `tokio` dependency, single integration test binary).
- REQ-CORE-009: Implement `Schema<T>` Type Binding & Return: `.parse(&val)` directly returns `Result<T, ZodError>` when `T: DeserializeOwned`, enabling single-line binding via `.into_type::<DomainStruct>()` on object schemas and automatic type composition in `z::array(schema)` (`Schema<Vec<T>>`) and `schema.optional()` (`Schema<Option<T>>`).
- REQ-CORE-010: Support dual raw/typed parsing: `.parse_value(&Value) -> Result<Value, ZodError>` provides direct access to the validated AST `Value`, while `.parse()` and `.decode()` return idiomatic `Result<T, ZodError>` natively without JS-style exception wrappers.

## Affected Components (from graph)
- `src/lib.rs` — Root `z` namespace facade and crate configuration
- `src/value.rs` — `Value` model (`Null`, `Bool`, `Int`, `Uint`, `Float`, `String`, `Bytes`, `Date`, `Array`, `Object`) and conversions
- `src/meta.rs` — `Meta` and `MetaBuilder` closure builder (`.meta(|m| ...)`)
- `src/error.rs` — `ZodIssueCode`, `ZodIssue`, `ZodError`, and `flatten`/`format`/`treeify`/`prettify`
- `src/i18n.rs` — `Locale`, built-in catalogs (`en`, `pt-BR`, `es`), global/context locale resolution
- `src/schema.rs` — `Schema` (`Arc<SchemaDef>`), `ParseContext`, `ParseOptions`, sync & async decode/encode engine

## Out of Scope
- Procedural derive macros (`#[derive(Zod)]`) and `.describe()`.
- Redundant JS exception wrappers (`.safe_parse()`, `.spa()`, `.safe_decode()`, `.safe_encode()`) — Rust's `Result` handles both `?` propagation and pattern matching natively.
- JS-only types (`undefined`, `void`, `nan`, `symbol`, `promise`, `function`, `instanceof`).

## Open Questions
- None.
