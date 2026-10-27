# State

Last synced commit: 3e2df83
**Last Updated:** 2026-10-27

## Current Work
Audit 2026-10-27 against INSIGHT.md found spec deviations SD-1..SD-6; all were fixed the same day (features 05/06/07/08 re-done) and feature 09 (optimizations) was implemented. Verified: 29 integration + 7 unit tests, clippy clean, clean-build target 111 MB. Earlier text follows. Completed the full implementation, idiomatic adaptations, and verification of all 8 features (Milestones 1 through 6) of `rust-zod`, including pure Zod v4 top-levels, closure builders, error formatting & i18n, options & collections, objects & records, combinators & cycle-safe lazy schemas, refinements & codecs, JSON Schema & OpenAPI exporters, reverse importer `from_json_schema`, GraphQL SDL generation, and the `Schema<T>` type conversion architecture.

## Todos
- [x] Review and refine `INSIGHT.md` master architectural blueprint
- [x] Implement and verify Milestone 1: `01-core-error-i18n` (Value model, errors, i18n, metadata builder)
- [x] Implement and verify Milestone 2: `02-primitives-formats-coercion` (primitives, string formats, numeric top-levels, coercion)
- [x] Implement and verify Milestone 3: `03-options-collections` (`z::options`, arrays, tuples, sets, maps)
- [x] Implement and verify Milestone 4: `04-objects-records` and `05-unions-intersections-lazy`
- [x] Implement and verify Milestone 5: `06-modifiers-refinements-pipelines` (codecs, pipelines, transforms)
- [x] Implement and verify Milestone 6: `07-json-schema-openapi` and `08-graphql-builder` (including `z::from_json_schema`)
- [x] Implement & verify `Schema<T>` Type Conversion Architecture (`.parse(&val) -> Result<T, ZodError>`, `.into_type::<T>()`, and type propagation)
- [x] Reopened and fixed: 05 (`lazy` OnceLock + SchemaDef nodes), 06 (remove `safe_*`, real async, `.input()/.output()`, `_with` errors, codec propagation), 07/08 (richer `SchemaDef`, io Input/Output, cycles, GraphQL union/scalar)
- [x] Implement Milestone 7: `09-optimizations` (inline path stack, inline strings, fast-path bypass, bench harness) — object-key SSO deferred (SD-7)

## Active Blockers
- none

## SPEC_DEVIATION (audit 2026-10-27 vs INSIGHT.md)
Resolved on 2026-10-27 (features 05/06/07/08 re-verified, feature 09 implemented):
- SD-1 resolved: `safe_parse`, `safe_*`, `SafeParseResult` removed.
- SD-2 resolved: `z::lazy` uses `OnceLock`; capture-free closures share one node; exporters resolve cycles by node pointer / `meta.id` (depth guard as a last resort).
- SD-3 resolved: `refine_async`, `transform_async`, `z::codec_async`, `Failure::Async` for sync parse of async schemas.
- SD-4 resolved: `.input()/.output()`, `.required()/.required_keys()`, `.error()`, `*_with(|e| e.error(..).abort(..))`, `.regex()`.
- SD-5 resolved: rich `SchemaDef`; JSON Schema/OpenAPI/GraphQL keep constraints, unions, tuples, records, scalars.
- SD-6 resolved: codec is a `Schema<Out>`; `.encode()` propagates through object/array/optional/union/pipe; one-way steps return `Failure::Encode`.
Open / accepted deviations:
- SD-7: object keys in `Value::Object` are still `String` (INSIGHT 8.2 asks for inline keys). `Value::String` and `PathSegment::Key` are inline `CompactString`.
- SD-8: async futures are `Send` but not `Sync` (INSIGHT 3.4 wrote `Send + Sync`); async uses a replay model, so async callbacks must be deterministic per input.
- SD-9: `z::object` returns `Schema<Value>` (INSIGHT 2.2); shape ops (`pick/omit/extend/...`) panic on non-object schemas (no compile-time guard).
- SD-10: `no_std` support dropped (`OnceLock`/`std::sync` required); `std`/`alloc` features kept as no-ops.
- SD-11: `loose_record` keeps non-matching keys untouched; `z::map` is a record alias (JSON-like `Value` has no Map type).

## Recent Decisions (Last 15)
- [2026-10-27] `Schema<T>` now carries runtime `Aux` structure (object shape, tuple parts, pipe halves, wrapped inner) next to the AST so `z::object` is a plain `Schema<Value>` and `.unwrap()/.input()/.output()/.pick()` work without a second type.
- [2026-10-27] Async via replay: async steps are leaf closures keyed by `(step id, input)`; the synchronous walk is re-run between awaits. Avoids a second async implementation of every combinator.
- [2026-10-27] `ParseContext.direction` (`Decode`/`Encode`) drives codecs, pipes, checks and one-way steps; checks on `.encode()` run on the incoming (output-side) value before the reverse mapping.
- [2026-10-27] Audit vs INSIGHT.md: recorded SD-1..SD-6, reopened features 05/06/07/08; earlier claims of "verified" for those items are superseded.
- [2026-10-27] `Schema<T>` strongly typed output: `.parse(&val)` directly returns `Result<T, ZodError>` for any `T: DeserializeOwned` (while preserving `.parse_value(&val)` for raw `Value` and universal `.parse_into::<Target>()`).
- [2026-10-27] Manual `Clone` implementation on `Schema<T>` decouples schema cloning from `T: Clone`, making all schemas unconditionally cloneable without macro overhead.
- [2026-10-27] Added `.into_type::<T>()` directly on `ObjectSchema` and `Schema<Value>`, enabling single-line domain struct binding like `z::object(...).into_type::<User>()`.
- [2026-10-27] Eliminated `.safe_parse()`, `.spa()`, `.safe_decode()`, `.safe_encode()` because Rust has no `throw` — `.parse()`, `.decode()`, and `.encode()` already return `Result<T, ZodError>`.
- [2026-10-27] `Schema` wraps `Arc<SchemaDef>` using `Arc::make_mut` (Copy-on-Write) for zero-allocation chaining and cheap $O(1)$ thread-safe cloning.
- [2026-10-27] `z::lazy` caches its inner `Schema` via `OnceLock` so recursive schemas do not stack-overflow during validation or JSON Schema/GraphQL export.
- [2026-10-27] `z::options([...])` accepts only a list of options via `IntoOptions` (no closure builder overload that would trigger Rust `E0282`).
- [2026-10-27] `z::codec(input, output, decode, encode)` uses positional concrete closure parameters and supports `Value::Date(i64)` and `Value::Bytes(Vec<u8>)`.
- [2026-10-27] Pure Zod v4 API: top-level string formats (`z::email()`, `z::uuid()`, etc.), top-level numbers (`z::int()`, `z::float64()`), `.meta(|m| ...)` without `.describe()`.

## Recent Progress (Last 10)
- [2026-10-27] Implemented `Schema<T>` direct type return on `.parse()`, `parse_into`, and `parse_value`, verified with typed object schemas, array composition, and optional composition in `tests/integration.rs`.
- [2026-10-27] Implemented and verified Feature 07 (`to_json_schema`, `from_json_schema`) and Feature 08 (`to_graphql_sdl`).
- [2026-10-27] Implemented and verified Feature 06 (`modifiers`, `refinements`, `pipelines`, `z::codec`, `z::stringbool`).
- [2026-10-27] Implemented and verified Feature 04 & 05 (`z::object`, `strict_object`, `loose_object`, `record`, `union`, `discriminated_union`, `xor`, `lazy`).
- [2026-10-27] Implemented and verified Feature 02 & 03 (`primitives`, `z::string` formats, `numeric` types, `coerce`, `z::options`, `z::array`, `tuple`, `set`, `map`).
- [2026-10-27] Implemented and verified Feature 01 (`01-core-error-i18n`): `Value` model, `MetaBuilder`, 11 `ZodIssueCode` variants, all 4 formatters, `Locale` (`en`, `pt-BR`, `es`).
- [2026-10-27] Passed full test suite (22 integration tests, 7 unit tests, 0 doc-test failures, 0 clippy warnings) with target footprint at 111.5 MB.

## Lessons Learned (Last 5)
- [2026-10-27] Deriving `Clone` on a struct with `PhantomData<fn() -> T>` adds an unnecessary `where T: Clone` bound; implementing `Clone` manually decouples the schema from the inner phantom type.
- [2026-10-27] Implementing `serde::Serialize` and `serde::Deserialize` for `Value` enables `Schema<T>` to seamlessly return `Result<T, ZodError>` whether `T` is `Value`, a primitive, or a domain struct.
- [2026-10-27] Passing closures to generic traits (`T: IntoOptions`) triggers Rust `E0282`; keeping `z::options` list-only and `z::codec` positional avoids `E0282` completely.
- [2026-10-27] Recursive schema functions in Rust allocate a new `Arc` on each call unless memoized inside `z::lazy` with `OnceLock`.

## Deferred Ideas
- Optional integration adapters for web frameworks (`axum`, `actix-web`).

## Cost
- Initial `nexspec sync`: 0ms.
