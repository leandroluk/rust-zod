# Spec: 01 — Core Value Model, Validation Engine, Error Formatting & i18n

## Summary
Establishes the `#![no_std]` (`alloc`) foundation of `rust-zod`: the runtime `Value` representation (with ergonomic conversions and optional `serde_json` interoperability), the Zod v4 issue/error model (`ZodIssue`, `ZodIssueCode`, `ZodError`), error formatting utilities (`flatten`, `format`, `prettify_error`), the internationalized message system (`Locale`, default validation messages, runtime/scoped locale switching for `en`, `pt-BR`, `es`, and custom error maps), and the base `ZodSchema` / `SchemaDef` AST execution engine supporting both sync (`parse`, `safe_parse`) and async (`parse_async`, `safe_parse_async`) execution without macros or decorators.

## Requirements
- REQ-CORE-001: Provide a runtime `Value` enum (`Null`, `Undefined`, `Bool`, `Number`, `BigInt`, `String`, `Array`, `Object`, `Date`, `Set`, `Map`) compatible with `#![no_std]` + `alloc` and seamlessly convertible from Rust primitives, tuples/arrays, maps, and `serde_json::Value`.
- REQ-CORE-002: Define the Zod v4 issue taxonomy (`ZodIssueCode`) covering `InvalidType`, `InvalidLiteral`, `InvalidEnumValue`, `UnrecognizedKeys`, `InvalidUnion`, `InvalidUnionDiscriminator`, `InvalidIntersectionTypes`, `NotMultipleOf`, `TooSmall`, `TooBig`, `InvalidString`, `InvalidDate`, `Custom`, and `NotFinite`.
- REQ-CORE-003: Implement `ZodIssue` (code, path segments `Prop`/`Index`, expected/received metadata, custom message override) and `ZodError` containing a list of issues with path tracking during nested validation.
- REQ-CORE-004: Provide Zod error formatting utilities on `ZodError`: `.flatten()` (form and field errors), `.format()` (nested tree structure), and `.prettify()` (human-readable Zod v4 style summary string).
- REQ-CORE-005: Provide default validation messages out of the box for every `ZodIssueCode` and constraint without requiring the user to pass custom error strings.
- REQ-CORE-006: Implement an i18n locale system (`Locale` trait / catalog) with built-in locales (`en`, `pt-BR`, `es`) and allow switching the active locale globally (`z::set_locale(...)`) or per parse call (`parse_with_locale` / `ParseOptions`), plus per-schema and per-check custom error message/map overrides.
- REQ-CORE-007: Define the introspectable `SchemaDef` AST and `Schema` runtime wrapper with `.parse(&Value)`, `.safe_parse(&Value)`, `.parse_async(&Value)`, and `.safe_parse_async(&Value)` methods (no procedural macros or struct decorators required).
- REQ-CORE-008: Enforce strict `target/` disk footprint limits (`debug = 0`, `strip = "debuginfo"`, `incremental = false`, zero proc-macro dependencies, single integration test binary).

## Affected Components (from graph)
- `src/lib.rs` — Root `z` namespace facade and crate configuration
- `src/value.rs` — `Value` model and `From` / `TryFrom` conversions
- `src/error.rs` — `ZodIssueCode`, `ZodIssue`, `ZodError`, and formatters
- `src/i18n.rs` — `Locale`, built-in catalogs (`en`, `pt-BR`, `es`), global/context locale resolution
- `src/schema.rs` — `SchemaDef` AST, `ParseContext`, sync & async validation engine

## Out of Scope
- Procedural derive macros (`#[derive(Zod)]`) — explicitly excluded by design in favor of pure runtime DSL.
- Heavy regex Unicode tables (use lightweight validation / `regex-lite` in M2).

## Open Questions
- None — all architectural and ergonomic requirements resolved with the user.
