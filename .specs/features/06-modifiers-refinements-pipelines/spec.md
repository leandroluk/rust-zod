# Spec: 06 — Modifiers, Refinements, Pipelines & Codecs

## Summary
Implements Milestone 5 based on [INSIGHT.md](../../../INSIGHT.md):
1. Schema modifiers:
   - `.default(val)` / `.prefault(val)` fallback when input is `Null`.
   - `.catch(val)` fallback when validation fails.
   - `.readonly()` marker.
2. Refinements & Validation hooks:
   - `.refine(pred, message)`: custom boolean check with localized/custom error.
   - `.super_refine(func)`: fine-grained issue reporting via `ParseContext`.
   - `.check(func)`: lightweight assertion.
3. Transformations & Pipelines:
   - `.transform(func)`: unidirectional value transformation.
   - `.preprocess(func)`: preprocess input before schema validation.
   - `.pipe(target_schema)`: compose output of one schema into another.
4. Bidirectional Codecs:
   - `z::codec(input, output, decode, encode)`:
     - `.decode(&val) -> Result<Value, ZodError>` (input -> output)
     - `.encode(&val) -> Result<Value, ZodError>` (output -> input, traverses pipes in reverse)
     - Safe and async variants.
   - `z::stringbool()` built-in codec with `.truthy()`, `.falsy()`, and `.case_sensitive()`.

## Requirements
- REQ-PIPE-001: Implement `.default()`, `.prefault()`, `.catch()`, and `.readonly()`.
- REQ-PIPE-002: Implement `.refine()`, `.super_refine()`, and `.check()`.
- REQ-PIPE-003: Implement `.transform()`, `.preprocess()`, and `.pipe()`.
- REQ-PIPE-004: Implement `z::codec(input, output, decode, encode)` with reverse pipeline traversal on encode.
- REQ-PIPE-005: Implement `z::stringbool()` with custom truthy/falsy overrides.

## Reopened (audit 2026-10-27 vs INSIGHT.md §2.3, §3.2, §3.4, §5, §6)
Status: **DONE (2026-10-27)** — `safe_*`/`SafeParseResult` removed; `refine_async`/`transform_async`/`z::codec_async` + `Failure::Async`; `.input()/.output()/.required()`; `.error()` and `*_with(|e| e.error(..).abort(..))`; codecs are plain `Schema<Out>` and `.encode()` propagates (`Direction::Encode`), `.transform()/.preprocess()` fail with `Failure::Encode`; `z::codecs::{json_string, base64_bytes}` added. Tests: `test_sd3_*`, `test_sd4_*`, `test_sd6_*`. Design note: async uses a replay model (async steps are leaf closures resolved between synchronous passes) so every combinator stays single-implementation; futures are `Send` (not `Sync`).
- REQ-PIPE-006: Remove `safe_parse`, `safe_parse_async`, `safe_parse_with`, `safe_decode`, `safe_encode` and `SafeParseResult` (INSIGHT §3.2/§6: `Result` already covers them).
- REQ-PIPE-007: Real async: `refine_async`, `transform_async`, `z::codec_async`; `.parse()` on a schema with async steps returns a clean error (equivalent to `$ZodAsyncError`), never panics. No `tokio`/`futures` in `[dependencies]`.
- REQ-PIPE-008: Rename `pipe.in/out` to `.input()` / `.output()`; add `.required()`.
- REQ-PIPE-009: Error customization per §2.3: `.error("...")` and `min_with(n, |e| e.error(..).abort(true))` (and `_with` for other checks).
- REQ-PIPE-010: `.encode()` traverses objects/arrays/unions (codec propagation, §5.2) and `.transform()` during encode yields an encode error, not a `ZodIssue`/panic.
- REQ-PIPE-011: Optional `z::codecs::*` recipe helpers (§5.3) beyond `stringbool`, `string_int`, `string_number`, `epoch_date`.
