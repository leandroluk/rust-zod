# Spec: 07 — JSON Schema, OpenAPI & Reverse Importer (from_json_schema)

## Summary
Implements Milestone 6 (JSON Schema, OpenAPI, and Reverse Importer) based on [INSIGHT.md](../../../INSIGHT.md):
1. `to_json_schema(schema, options)` supporting:
   - Targets: `Draft202012` (default), `Draft07`, `Draft04`, `OpenApi30`, `OpenApi31`.
   - Direction: `Input` vs `Output` (for codecs and transforms).
   - `$defs` generation and cycle detection using `meta.id`.
   - `override_with` hook for custom vendor extensions.
2. `z::from_json_schema(json_val)`:
   - Dynamic schema reconstruction from JSON Schema documents.

## Requirements
- REQ-EXP-001: Implement `to_json_schema` for Draft 2020-12, Draft-07, and OpenAPI 3.0/3.1.
- REQ-EXP-002: Implement cycle detection and `$defs` sharing based on `meta.id`.
- REQ-EXP-003: Implement `z::from_json_schema` reverse importer.

## Reopened (audit 2026-10-27 vs INSIGHT.md §9)
Status: **DONE (2026-10-27)** — `SchemaDef` now models string formats/checks, number kinds/checks, literal, array/tuple/set/record, union/xor/discriminated/intersection, default/readonly, lazy, transform/pipe/codec and a `Meta` wrapper; exporter emits constraints, `prefixItems` vs legacy `items`, `io: Input|Output`, `$ref` cycles. OpenAPI targets use `#/components/schemas`. `from_json_schema` handles `$ref`, `anyOf/oneOf/allOf`, `const/enum`, constraints, tuples, `additionalProperties`, `nullable`. Test: `test_sd5_*`.
- REQ-JSON-006: Enrich `SchemaDef` (tuple, record, union, intersection, literal, pipe, codec, lazy, string formats, numeric/length constraints) so the exporter stops dropping them: emit `format`, `minLength/maxLength/pattern`, `minimum/maximum/multipleOf`, `oneOf/anyOf/allOf`, `prefixItems`, `additionalProperties`, `const/enum`.
- REQ-JSON-007: `io: Input | Output` must pick codec input/output schema (§5.4).
- REQ-JSON-008: Cycles via lazy node pointer and `meta.id` resolve into `$defs`/`definitions` without infinite loop.
