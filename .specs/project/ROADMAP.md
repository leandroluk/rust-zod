# Roadmap: rust-zod (Zod v4 Port for Rust)

## Milestone 1: Foundation, Value Model, Errors & i18n (`M1-foundation`)
- **Feature 01 (`01-core-error-i18n`)**:
  - `no_std` + `alloc` intermediate `Value` / `serde_json::Value` interoperability
  - `ZodIssue`, `ZodIssueCode` (Zod v4 issue taxonomy), `ZodError`, path tracking
  - Error formatting (`flatten()`, `format()`, `prettify_error()`)
  - i18n system: `Locale`, `ErrorMap`, default validation messages, global & per-parse locale switching (`en`, `pt-BR`, `es`)
  - Core traits: `Schema` / `ZodType`, `ParseContext`, `SafeParseResult`, `.parse()`, `.safe_parse()`, `.parse_async()`, `.safe_parse_async()`

## Milestone 2: Primitive & Literal Schemas (`M2-primitives`)
- **Feature 02 (`02-primitives-coercion`)**:
  - `z::string()` (min, max, length, email, url, uuid, cuid, cuid2, ulid, regex, includes, starts_with, ends_with, datetime, ip, cidr, base64, trim, to_lower_case, to_upper_case, nan, etc.)
  - `z::number()` / `z::int()` / `z::float()` (gt, gte/min, lt, lte/max, int, positive, non_negative, negative, non_positive, multiple_of, finite, safe)
  - `z::bigint()` / `i128` / `u128`
  - `z::boolean()`, `z::null()`, `z::undefined()` / `z::unit()`, `z::any()`, `z::unknown()`, `z::never()`
  - `z::date()` / ISO date/time handling
  - `z::literal(...)` (single and multi-literal)
  - `z::coerce::*` (`string`, `number`, `boolean`, `bigint`, `date`)

## Milestone 3: Macro-Free Enums & Collections (`M3-enums-collections`)
- **Feature 03 (`03-enums-collections`)**:
  - Macro-free `z::enum_([...])` with `.options()`, `.extract([...])`, `.exclude([...])` and ergonomic value matching without Rust proc-macros
  - `z::array(schema)` (min, max, length, non_empty)
  - `z::tuple([...])` with optional `.rest(schema)`
  - `z::set(schema)` (min, max, size, non_empty)
  - `z::map(key_schema, value_schema)`

## Milestone 4: Objects, Interfaces & Structural Combinators (`M4-objects-combinators`)
- **Feature 04 (`04-objects-interfaces`)**:
  - `z::object(...)` fluent builder without decorators
  - Unknown keys handling: `.strict()`, `.passthrough()`, `.strip()`, `.catchall(schema)`
  - Object manipulation: `.shape()`, `.keyof()`, `.extend(...)`, `.merge(...)`, `.pick([...])`, `.omit([...])`, `.partial()`, `.deep_partial()`, `.required()`
  - Zod v4 `z::interface(...)` support (optional key vs optional value semantics)
  - `z::record(key_schema, value_schema)`
- **Feature 05 (`05-unions-intersections-lazy`)**:
  - `z::union([...])`, `z::discriminated_union(discriminator, [...])`, `z::xor([...])`
  - `z::intersection(left, right)`
  - `z::lazy(|| ...)` for recursive schemas

## Milestone 5: Modifiers, Refinements, Transforms & Pipelines (`M5-modifiers-pipelines`)
- **Feature 06 (`06-modifiers-refinements-pipelines`)**:
  - Wrapper modifiers: `.optional()`, `.nullable()`, `.nullish()`, `.default(val)`, `.prefault(val)`, `.catch(val)`, `.readonly()`, `.branded()`
  - Validation hooks: `.refine(...)`, `.super_refine(...)`, `.check(...)` (Zod v4 unified check API)
  - Async hooks: `.refine_async(...)`, `.super_refine_async(...)`, `.transform_async(...)`
  - Transformations & pipelines: `.transform(...)`, `.preprocess(...)`, `.pipe(...)`, `z::stringbool()`

## Milestone 6: Schema Metadata, JSON Schema, OpenAPI & GraphQL (`M6-exporters`)
- **Feature 07 (`07-metadata-json-schema-openapi`)**:
  - `.describe(...)`, `.meta(...)`, `ZodRegistry` (Zod v4 metadata registry)
  - `z::to_json_schema(schema, options)` (JSON Schema Draft 2020-12, Draft-07, OpenAPI 3.0 / 3.1 dialect)
- **Feature 08 (`08-graphql-builder`)**:
  - GraphQL schema & type generation from `rust-zod` AST (`to_graphql_sdl`, InputObject, Object, Enum, Scalar, List, NonNull)
