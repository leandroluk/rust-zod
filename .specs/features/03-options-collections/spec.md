# Spec: 03 — Options (Enums) & Collections

## Summary
Implements Milestone 3 based on [INSIGHT.md](../../../INSIGHT.md):
1. `z::options([...])`:
   - Enforce list-only options via `IntoOptions` (avoiding closure builder inference E0282 and keyword collision with `enum`).
   - `.options()`: returns the list of allowed strings.
   - `.extract([...])`: returns a new schema with a subset of options.
   - `.exclude([...])`: returns a new schema excluding the specified options.
2. `z::array(schema: Schema<T>)`:
   - Typed as `Schema<Vec<T>>`.
   - `.min(n)` / `.max(n)` / `.length(n)` / `.nonempty()`.
   - `.unique()`: enforces element uniqueness.
   - `.element()` / `.unwrap()`: returns the inner item schema.
3. `z::tuple([...])`:
   - Validates fixed-length heterogeneous array elements.
   - `.rest(schema)`: validates trailing rest elements.
4. `z::set(schema: Schema<T>)`:
   - Validates sets, enforcing item schema and uniqueness.
   - `.min(n)` / `.max(n)` / `.size(n)` / `.nonempty()`.
5. `z::map(key_schema: Schema<K>, value_schema: Schema<V>)`:
   - Validates maps/objects with strongly typed key and value schemas.
   - `.min(n)` / `.max(n)` / `.size(n)` / `.nonempty()`.

## Requirements
- REQ-COLL-001: Implement `z::options([...])` with `.options()`, `.extract([...])`, and `.exclude([...])`.
- REQ-COLL-002: Implement `z::array(item)` with `.min()`, `.max()`, `.length()`, `.nonempty()`, `.unique()`, and `.element()`.
- REQ-COLL-003: Implement `z::tuple([...])` with heterogeneous element validation and `.rest(schema)`.
- REQ-COLL-004: Implement `z::set(schema)` with `.min()`, `.max()`, `.size()`, and `.nonempty()`.
- REQ-COLL-005: Implement `z::map(key, val)` with `.min()`, `.max()`, `.size()`, and `.nonempty()`.
