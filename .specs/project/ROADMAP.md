# Roadmap: rust-zod (Pure Zod v4 Port for Rust)

> See [INSIGHT.md](../../INSIGHT.md) for the complete mapping of pure Zod v4 top-level constructors, discarded v3 deprecations, Rust limits (`E0282`, `Arc<SchemaDef>`, `OnceLock`, `Result`), and Closure Builder patterns.

## Milestone 1: Foundation, Value Model, Metadata, Errors & i18n (`M1-foundation`)
- **Feature 01 (`01-core-error-i18n`)**:
  - `no_std` + `alloc` runtime `Value` (`Null`, `Bool`, `Int`, `Uint`, `Float`, `String`, `Bytes`, `Date`, `Array`, `Object`) with ergonomic `From` conversions, `parse_into::<T>()`, and optional `serde_json` interoperability.
  - Closure-based metadata builder `.meta(|m| m.id(...).title(...).description(...).example(...).deprecated(...).custom(k, v))` (replaces `.describe()`).
  - Zod v4 11-variant issue taxonomy (`ZodIssue`, `ZodIssueCode`, `ZodError`), path tracking, and all 4 Zod v4 formatters (`flatten()`, `format()`, `treeify()`, `prettify()`).
  - i18n system: `Locale` (`en`, `pt-BR`, `es`, custom), default validation messages out of the box, global (`z::set_locale`) and request-scoped (`parse_with`) locale switching.
  - Base `Schema` AST (`Arc<SchemaDef>` with `Arc::make_mut` Copy-on-Write) supporting `.parse()`, `.parse_async()`, `.decode()`, `.encode()`, `.decode_async()`, `.encode_async()` returning native `Result` (no `safe_parse`/`spa` bloat).

## Milestone 2: Pure Zod v4 Primitives, Top-Level String Formats & Coercion (`M2-primitives`)
- **Feature 02 (`02-primitives-formats-coercion`)**:
  - `z::string()` with generic text checks/transforms (`min`, `max`, `length`, `nonempty`, `regex`, `includes`, `starts_with`, `ends_with`, `lowercase`, `uppercase`, `trim`, `normalize`, `to_lower_case`, `to_upper_case`, `slugify`).
  - Top-level Zod v4 string format constructors:
    - `z::email()`, `z::url()`, `z::http_url()`, `z::hostname()`
    - `z::uuid()`, `z::uuidv4()`, `z::uuidv6()`, `z::uuidv7()`, `z::guid()`
    - `z::nanoid()`, `z::cuid2()`, `z::ulid()`, `z::xid()`, `z::ksuid()`
    - `z::ipv4()`, `z::ipv6()`, `z::mac()`, `z::cidrv4()`, `z::cidrv6()`
    - `z::base64()`, `z::base64url()`, `z::hex()`, `z::jwt()`, `z::emoji()`, `z::e164()`, `z::credit_card()`, `z::iban()`, `z::currency_code()`, `z::hash(...)`, `z::string_format(...)`
    - `z::iso::date()`, `z::iso::time()`, `z::iso::datetime()`, `z::iso::duration()`
  - Top-level Zod v4 numeric constructors:
    - `z::number()` / `z::float64()`, `z::float32()`
    - `z::int()`, `z::int32()`, `z::uint32()`, `z::int64()`, `z::uint64()`, `z::bigint()`
    - Checks: `gt`, `gte`/`min`, `lt`, `lte`/`max`, `positive`, `nonnegative`, `negative`, `nonpositive`, `multiple_of`.
  - `z::boolean()`, `z::null()`, `z::any()`, `z::unknown()`, `z::never()`, `z::date()`, `z::file()`, `z::json()`.
  - `z::literal(...)` (single and multi-value literal via `IntoLiterals`).
  - `z::coerce::*` (`string`, `number`, `int`, `boolean`, `bigint`, `date`).

## Milestone 3: Macro-Free Options (Enums) & Collections (`M3-options-collections`)
- **Feature 03 (`03-options-collections`)**:
  - `z::options([...])` via `IntoOptions` trait (always receives a list of options: array, slice, or `Vec`, avoiding reserved keyword `enum` and `E0282` closure inference issues), with `.options()`, `.extract([...])`, and `.exclude([...])`.
  - `z::array(schema)` (`min`, `max`, `length`, `nonempty`, `unique`, `unwrap`).
  - `z::tuple([...])` with `.rest(schema)` and `.partial()`.
  - `z::set(schema)` and `z::map(key_schema, value_schema)` (`min`, `max`, `size`, `nonempty`).

## Milestone 4: Objects, Records & Structural Combinators (`M4-objects-combinators`)
- **Feature 04 (`04-objects-records`)**:
  - Closure-based object builders: `z::object(|o| ...)`, `z::strict_object(|o| ...)`, `z::loose_object(|o| ...)`.
  - Unknown keys handling: `.strict()`, `.loose()`, `.strip()`, `.catchall(schema)`.
  - Object manipulation: `.keyof()`, `.extend(|o| ...)`, `.pick(["a", "b"])`, `.omit(["a", "b"])`, `.partial()`, `.partial_keys(["a"])`, `.required()`, `.required_keys(["a"])`.
  - Records: `z::record(key, val)`, `z::partial_record(key, val)`, `z::loose_record(key, val)`.
- **Feature 05 (`05-unions-intersections-lazy`)**:
  - `z::union([...])`, `z::discriminated_union(discriminator, [...])`, `z::xor([...])`.
  - `z::intersection(left, right)`.
  - `z::lazy(|| ...)` with `OnceLock` memoization for cycle-safe recursive schemas.

## Milestone 5: Modifiers, Refinements, Codecs & Pipelines (`M5-modifiers-pipelines`)
- **Feature 06 (`06-modifiers-refinements-pipelines`)**:
  - Modifiers: `.optional()`, `.nullable()`, `.nullish()`, `.nonoptional()`, `.default(val)`, `.prefault(val)`, `.catch(val)`, `.readonly()`.
  - Sync & Async hooks: `.check(...)`, `.refine(...)`, `.super_refine(...)`, `.overwrite(...)`.
  - Bidirectional Codecs & Pipelines: `z::codec(in, out, decode, encode)`, `z::codec_async(...)`, `z::stringbool()`, `z::codecs::*`, `.transform(...)`, `.preprocess(...)`, `.pipe(...)` (with `.input()` and `.output()`).

## Milestone 6: JSON Schema, OpenAPI & GraphQL Exporters/Importers (`M6-exporters`)
- **Feature 07 (`07-json-schema-openapi`)**:
  - `to_json_schema(|opt| ...)` supporting `Draft202012`, `Draft07`, `Draft04`, `OpenApi30`, `OpenApi31`, `io: Input | Output`, cycle `$defs`, and `override_with(...)`.
  - `z::from_json_schema(&Value)` runtime importer.
- **Feature 08 (`08-graphql-builder`)**:
  - GraphQL SDL & Type builder (`to_graphql_sdl()`, `Object` vs `InputObject`, `Enum` from `z::options([...])`, `Union`, automatic `NonNull` `!` inversion, and PascalCase nested type naming).

## Milestone 7: Low-Level Rust Optimizations, Zero-Heap Happy Path & Benchmarks (`M7-optimizations`)
- **Feature 09 (`09-optimizations`)**:
  - Zero-heap path tracking in `ParseContext` via inline contiguous stack buffer (`SmallVec<[PathSegment; 8]>` or equivalent) with automatic spillover and seamless `.into_vec()` issue conversion.
  - Small String Optimization (SSO) for object keys, formats, and identifiers (<= 24 bytes) without lifetime contamination (`'a`).
  - Fast-path direct deserialization bypass for identity conversions (`Schema<Value>`) and primitives.
  - Microbenchmark suite (`criterion` or custom harness) tracking validation latency, memory allocations, and target disk footprint.
