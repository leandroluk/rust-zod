# Spec: 09 — Low-Level Rust Optimizations & Zero-Heap Execution

## Summary
Specifies Milestone 7 based on Section 8 of [INSIGHT.md](../../../INSIGHT.md):
1. Zero-Heap Path Tracking on Happy Path:
   - Contiguous inline stack buffer for `PathSegment` hierarchy during validation.
   - Zero heap allocation when validating valid payloads.
   - Transparent spillover and auto-conversion to standard `Vec<PathSegment>` on issue emission.
2. Lifetime-Free Small String Optimization (SSO):
   - Zero lifetime contamination (`'a`) preserving `'static`, `Send + Sync` schemas.
   - Inline stack storage for strings <= 24 bytes in `Value`.
3. Fast-Path Serde Bypass:
   - Direct zero-copy return for `Schema<Value>`.
   - Direct variant extraction for primitive scalar types (`f64`, `i64`, `bool`, `String`).
4. Microbenchmarking:
   - Automated benchmark tracking for execution time and allocations.

## Requirements
- REQ-OPT-001: Implement contiguous inline stack path tracking in `ParseContext` eliminating heap allocation during successful validation traversals.
- REQ-OPT-002: Maintain zero lifetime (`'a`) propagation in public types (`Schema`, `Value`), supporting SSO transparently via standard `Deref<Target = str>`.
- REQ-OPT-003: Implement serde bypass fast-paths for identity conversions (`Schema<Value>`) and scalar primitives in `.parse()`.
- REQ-OPT-004: Ensure all optimizations maintain 100% compatibility with existing test suites, error formatters, and target disk size constraints (< 150 MB).

## Affected Components (from graph)
- `src/schema.rs` — `ParseContext` and fast-path execution
- `src/value.rs` — Inline string optimizations and conversions
- `src/error.rs` — Path conversion to `Vec<PathSegment>`

## Status: DONE (2026-10-27) with one documented limit
- REQ-OPT-001 done: `ParseContext.path` is `SmallVec<[PathSegment; 8]>`; `PathSegment::Key` holds a `CompactString`, so pushing a key allocates nothing; the heap `Vec` is only built when an issue is emitted (`path.to_vec()`). Test: `test_feature_09_paths_and_fast_paths` (12-level spillover).
- REQ-OPT-002 partial: `Value::String` is a `CompactString` (<= 24 bytes inline, `Deref<Target = str>`, no lifetimes in `Schema`/`Value`). **Limit (SD-7):** object keys in `Value::Object` remain `String` (`BTreeMap<String, Value>`), so each output object key still allocates. Moving keys to inline strings changes the public map type and was deferred.
- REQ-OPT-003 done: `parse`/`parse_into` extract `Value`, `String`, `bool`, `f64`, `f32`, `i64`, `u64`, `i32`, `u32` directly from `Value` without serde.
- REQ-OPT-004 done: all tests pass, clippy clean, clean-build `target/` = 111 MB (< 150 MB).
- Benchmark harness: `cargo run --release --example bench` (counting allocator). Reference run: primitives 30 ns / 0 allocs, email 74 ns / 1 alloc, nested object (6 fields) 601 ns / 9 allocs (output map keys), invalid object with issue paths 1.2 us / 15 allocs.
