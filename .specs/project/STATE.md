# State

Last synced commit: pending
**Last Updated:** 2026-10-27

## Current Work
Completed Phase 0 (Init & Index Build) and Phase 1 (Specify) for Feature `01-core-error-i18n`. Enforced strict `target/` disk size policy via `.cargo/config.toml` (`debug = 0`, `strip = "debuginfo"`, `incremental = false`, single-crate modular layout, zero proc-macros). Ready to proceed to Design -> Tasks -> Execute for `01-core-error-i18n` or adjust roadmap/spec if requested.

## Todos
- [ ] Design phase for `01-core-error-i18n` (`.specs/features/01-core-error-i18n/design.md`)
- [ ] Tasks breakdown for `01-core-error-i18n` (`.specs/features/01-core-error-i18n/tasks.md`)
- [ ] Execute `01-core-error-i18n` and verify `target/` size and tests

## Active Blockers
- none

## Recent Decisions (Last 15)
- [2026-10-27] Feature "01-core-error-i18n" specified. Affects modules: `value`, `error`, `i18n`, `schema`. REQ count: 8 (`REQ-CORE-001`..`REQ-CORE-008`). Index queried: yes.
- [2026-10-27] Disk Footprint Policy: Configured `.cargo/config.toml` with `debug = 0`, `strip = "debuginfo"`, `incremental = false`, single crate + single integration test binary to prevent `target/` explosion.
- [2026-10-27] Requirement Traceability enabled (`Traceability: on` in `PROJECT.md`).
- [2026-10-27] API Paradigm: Pure runtime fluent DSL (`z::string()`, `z::object()`, `z::enum_()`) without macros or decorators.
- [2026-10-27] Built-in default validation messages + i18n with dynamic locale switching (`en`, `pt-BR`, `es`).
- [2026-10-27] Multi-target exporters: JSON Schema, OpenAPI 3.0/3.1, and GraphQL (objects/types/SDL).

## Recent Progress (Last 10)
- [2026-10-27] Specified Feature `01-core-error-i18n` (`REQ-CORE-001` to `REQ-CORE-008`).
- [2026-10-27] Initialized `.specs/` directory layout, `.cargo/config.toml`, `nexspec` index, and git hooks.

## Lessons Learned (Last 5)
- [2026-10-27] Disabling debug info (`debug = 0`), stripping symbols, disabling incremental cache, and avoiding proc-macro crates reduces Rust `target/` folder size by >85% on Windows/MSVC.

## Deferred Ideas
- Optional integration adapters for web frameworks (`axum`, `actix-web`).

## Cost
- Initial `nexspec sync`: running on initial commit.
