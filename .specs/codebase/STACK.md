# Stack & Footprint Policy

## Language & Toolchain
- **Rust Edition**: 2024 (or 2021 compatible MSRV 1.80+)
- **Core Compatibility**: `#![no_std]` + `alloc` in `zod-core` and `zod-i18n`

## Disk & `target/` Footprint Control (Strict Policy)
Rust workspaces on Windows/MSVC quickly balloon into tens of gigabytes due to `.pdb` debug symbols, incremental compilation caches, and heavy proc-macro trees (`syn`/`quote`). To prevent disk bloat:
1. **Zero Debug Symbols in Dev/Test**: Configured via `.cargo/config.toml` and root `Cargo.toml`:
   - `debug = 0` and `strip = "debuginfo"` in `[profile.dev]` and `[profile.test]` (eliminates massive `.pdb` files).
   - `incremental = false` (prevents unbounded growth in `target/debug/incremental/`).
2. **Macro-Free / Lean Dependency Tree**:
   - No `syn` / `quote` / `proc-macro2` required in the core architecture.
   - Prefer `regex-lite` over `regex` for string format validations to avoid multi-megabyte Unicode tables and slow builds.
   - Keep external dependencies to an absolute minimum.
3. **Single Integration Test Binary Pattern**:
   - Avoid dozens of separate files in `tests/*.rs` (each top-level file in `tests/` links a separate multi-MB executable). Instead, use unit tests inside modules or a single consolidated `tests/suite/main.rs` binary.
