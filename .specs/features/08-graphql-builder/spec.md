# Spec: 08 — GraphQL SDL & Type Generator

## Summary
Implements Milestone 6 (GraphQL Exporter) based on [INSIGHT.md](../../../INSIGHT.md):
1. `to_graphql_sdl(schema, options)` generating:
   - `type` (Output Object) vs `input` (Input Object).
   - `enum` variants from `z::options([...])`.
   - `union` types.
   - Automatic inverted nullability: `!` by default for required fields, omitting `!` when `.optional()` or `.nullable()`.
   - PascalCase type naming from `meta.id` or field path.

## Requirements
- REQ-GQL-001: Implement GraphQL SDL generation (`to_graphql_sdl`) for Object, InputObject, Enum, Union, Scalar, and NonNull (!).
- REQ-GQL-002: Invert nullability correctly (! required by default, unwrap ! on optional/nullable).
- REQ-GQL-003: Type naming via `meta.id` with fallback.

## Reopened (audit 2026-10-27 vs INSIGHT.md §9.3)
Status: **DONE (2026-10-27)** — nested object/enum/union types are emitted with inferred PascalCase names (`ThingAddress`), `union` for object members, custom scalars (`DateTime`, `BigInt`, `Bytes`, `JSON`) declared, recursive types resolved by lazy node. Tests: `test_sd5_*`, `test_sd2_*`.
- REQ-GQL-006: Generate `union` (from union/discriminated_union), `scalar` (date/bytes/bigint/custom formats) and `input` types; verify `!` nullability inversion for optional/nullable.
- REQ-GQL-007: Resolve recursive (lazy) types without looping; PascalCase naming for nested objects (`UserAddress`).
