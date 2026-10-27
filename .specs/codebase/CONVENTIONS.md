# Conventions

## Code & Design Conventions
- **Requirement Traceability**: Every feature requirement has a `REQ-XXX` ID in `.specs/features/*/spec.md`. Implementing code includes `// @spec REQ-XXX` doc/line comments, and commits include `[REQ-XXX]`.
- **No Decorators / No Proc-Macros Required**: All schemas (`string`, `number`, `enum_`, `object`, `interface`, `union`, etc.) must be constructible via pure Rust functions/methods under the `z` namespace.
- **Disk Footprint Discipline**:
  - Never add multiple top-level `tests/*.rs` binaries; group integration tests under a single test harness (`tests/integration.rs` with submodules) so Cargo links only one test executable.
  - Keep `debug = 0`, `strip = "debuginfo"`, and `incremental = false` enabled.
