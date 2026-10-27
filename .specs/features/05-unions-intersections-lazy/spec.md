# Spec: 05 — Unions, Intersections & Lazy Recursion

## Summary
Implements Milestone 4 (Combinators & Recursion) based on [INSIGHT.md](../../../INSIGHT.md):
1. `z::union([...])`: matches if any option succeeds.
2. `z::discriminated_union("discriminator", [...])`: optimized O(1) matching based on discriminator key.
3. `z::xor([...])`: matches if exactly one option succeeds.
4. `z::intersection(left, right)`: matches if both succeed and merges validated values.
5. `z::lazy(|| ...)`: memoized recursive schema evaluation via `OnceLock`.

## Requirements
- REQ-COMB-001: Implement `z::union([...])` with comprehensive union issue reporting.
- REQ-COMB-002: Implement `z::discriminated_union(discriminator, [...])`.
- REQ-COMB-003: Implement `z::xor([...])`.
- REQ-COMB-004: Implement `z::intersection(left, right)`.
- REQ-COMB-005: Implement `z::lazy(|| ...)` with `OnceLock` memoization to prevent infinite recursion and stack overflow.

## Reopened (audit 2026-10-27 vs INSIGHT.md §3.3)
Status: **DONE (2026-10-27)** — reopened by the INSIGHT audit, now implemented and tested (`test_sd2_*`). `z::lazy` uses `std::sync::OnceLock`; capture-free closures/fn items share one `LazyDef` node (type-keyed registry), which is what makes `fn category() { ... z::lazy(category) ... }` finite and lets exporters detect cycles by pointer. `union/xor/discriminated_union/intersection/lazy` are first-class `SchemaDef` nodes.
- REQ-COMB-005a: `z::lazy` must memoize with `std::sync::OnceLock<Schema>` (current code uses a hand-rolled `mod spin` mutex).
- REQ-COMB-005b: `lazy`/`union`/`intersection` must be first-class `SchemaDef` nodes (currently `SchemaDef::Custom { name: "lazy" }`), so exporters can track the lazy node pointer and resolve cycles into `$defs`/`$ref`.
- REQ-COMB-005c: Test: recursive `Category` schema parses, and exports to JSON Schema and GraphQL without looping.
