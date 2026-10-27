# Architecture

## Overview
`rust-zod` is structured around an inspectable, runtime Schema AST (`SchemaDef`) paired with a fluent builder DSL (`z::*`). Because every schema is represented as data + composable validation/transformation steps rather than compile-time procedural macros, the same schema definition powers:
1. **Synchronous & Asynchronous Validation** (`.parse()`, `.safe_parse()`, `.parse_async()`, `.safe_parse_async()`)
2. **Internationalized Error Reporting** (`zod-i18n` with default messages and dynamic locale switching)
3. **Multi-Target Schema Export** (JSON Schema, OpenAPI 3.0/3.1, and GraphQL SDL/Types)

## Crate / Module Graph
- `zod-core`:
  - `Value`: Lightweight runtime value representation (interoperable with `serde_json::Value`).
  - `ZodIssue`, `ZodIssueCode`, `ZodError`: Structured error hierarchy with path tracking and formatting (`flatten`, `format`, `prettify`).
  - `SchemaDef` & `CheckDef`: Introspectable AST describing types, constraints, modifiers, and metadata.
  - `Schema` / `ZodType`: Fluent runtime methods.
- `zod-i18n`:
  - `Locale` trait & built-in locales (`en`, `pt_br`, `es`).
  - Global and per-context locale resolution + custom `ErrorMap` overrides.
- `zod-exporters` (or feature-gated modules for single-artifact footprint efficiency):
  - `json_schema`: Zod v4 `to_json_schema` (Draft 2020-12, Draft-07).
  - `openapi`: OpenAPI 3.0 / 3.1 schema object generator.
  - `graphql`: GraphQL SDL and type object builder (`InputObject`, `Object`, `Enum`, `Scalar`, `List`, `NonNull`).
