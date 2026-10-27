# Spec: 04 — Objects & Records

## Summary
Implements Milestone 4 (Objects and Records) based on [INSIGHT.md](../../../INSIGHT.md):
1. Closure-based object builders:
   - `z::object(|o| ...)` (strip unknown keys by default)
   - `z::strict_object(|o| ...)` (reject unknown keys with `UnrecognizedKeys`)
   - `z::loose_object(|o| ...)` (preserve unknown keys)
2. Unknown keys handling & modifiers:
   - `.strict()`, `.loose()`, `.strip()`, `.catchall(schema)`.
3. Object shape transformations:
   - `.extend(|o| ...)`, `.pick([...])`, `.omit([...])`, `.partial()`, `.partial_keys([...])`, `.required()`, `.keyof()`.
4. Records:
   - `z::record(key, val)`, `z::partial_record(key, val)`, `z::loose_record(key, val)`.

## Requirements
- REQ-OBJ-001: Implement `z::object(|o| ...)`, `z::strict_object(|o| ...)`, and `z::loose_object(|o| ...)`.
- REQ-OBJ-002: Implement object modifiers `.strict()`, `.loose()`, `.strip()`, and `.catchall(schema)`.
- REQ-OBJ-003: Implement shape combinators `.extend(|o| ...)`, `.pick([...])`, `.omit([...])`, `.partial()`, `.partial_keys([...])`, `.required()`, and `.keyof()`.
- REQ-OBJ-004: Implement record constructors `z::record(k, v)`, `z::partial_record(k, v)`, and `z::loose_record(k, v)`.
