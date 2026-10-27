# Project: rust-zod

Traceability: on

## Vision
A complete, idiomatic Rust port of **Zod v4** focused on a **pure runtime fluent DSL** (`z::string().min(3)`, `z::object(...)`, `z::enum_(...)`) without requiring procedural macros or struct decorators, while maintaining a strictly bounded `target/` disk footprint.

## Core Principles
1. **Zero-Macro / Decorator-Free Ergonomics**: Define primitives, objects, interfaces, enums, unions, intersections, tuples, records, refinements, and pipelines purely through a chainable runtime DSL (`z::*`).
2. **Default Validation Messages & i18n**: Built-in, human-friendly default validation messages with first-class internationalization (i18n) and runtime/scoped locale switching (`en`, `pt-BR`, `es`, plus custom locales).
3. **Multi-Target Schema Introspection & Export**: Build once with `rust-zod` and export/generate:
   - **JSON Schema** (Draft 2020-12 / Draft-07, native Zod v4 `to_json_schema` style)
   - **OpenAPI** (3.0 / 3.1 schema components & parameter definitions)
   - **GraphQL** (Input/Output Object types, Enums, Scalars, NonNull, Lists, SDL export)
4. **Sync & Async Validation**: `.parse()`, `.safe_parse()`, `.parse_async()`, and `.safe_parse_async()` supporting custom sync and async refinements/transforms.
5. **`no_std` Ready & Minimal Disk Footprint**: Core compatible with `#![no_std]` (`alloc`), single-crate modular architecture, `debug = 0`, `strip = "debuginfo"`, `incremental = false`, and zero proc-macro bloat so `target/` stays tiny.
