# Structure & Testing

## Planned Layout
To minimize `target/` artifact duplication while maintaining clean separation of concerns, `rust-zod` uses a single cohesive crate with clean internal domain modules (or thin workspace members sharing a single test binary):

```text
.cargo/
  config.toml           # Enforces debug=0, strip="debuginfo", incremental=false
.specs/                 # NexSpec specifications, state, and index
src/
  lib.rs                # `z` facade and public exports
  value.rs              # Runtime Value model & conversions
  error.rs              # ZodIssue, ZodIssueCode, ZodError, formatters
  i18n/                 # Locales (en, pt-BR, es), ErrorMap, default messages
  schema/               # Schema AST, ParseContext, sync/async execution
  types/                # Primitives, Options, Object, Union, Array, etc.
  exporters/
    json_schema.rs      # JSON Schema exporter
    openapi.rs          # OpenAPI 3.0 / 3.1 exporter
    graphql.rs          # GraphQL Type / SDL builder
tests/
  integration.rs        # Single test binary entry point to avoid target/ bloat
```

## Concerns & Risk Tracking
- **Target Directory Size**: Monitored via `(Get-ChildItem target -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB`. Using a single crate with modular namespaces + single integration test binary + `debug = 0` + `incremental = false` keeps `target/` under ~50–100 MB instead of 5–15 GB.
