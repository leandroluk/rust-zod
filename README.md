<div align="center">
  <img src="docs/assets/social-preview.png" alt="rust-zod" width="100%" />
</div>

<br/>

<div align="center">
  <h1>rust-zod</h1>
  <p><strong>Idiomatic, macro-free, pure runtime AST schema validation and serialization library for Rust, faithfully ported from pure Zod v4.</strong></p>
</div>

<br/>

<div align="center">
  <a href="https://github.com/leandroluk/rust-zod/actions">
    <img src="https://github.com/leandroluk/rust-zod/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI Build Status" />
  </a>
  <a href="https://codecov.io/gh/leandroluk/rust-zod">
    <img src="https://img.shields.io/codecov/c/github/leandroluk/rust-zod/main.svg" alt="Coverage Status" />
  </a>
  <a href="https://docs.rs/rust-zod">
    <img src="https://docs.rs/rust-zod/badge.svg" alt="Reference / Docs.rs" />
  </a>
  <a href="https://crates.io/crates/rust-zod">
    <img src="https://img.shields.io/crates/v/rust-zod.svg?style=flat-square" alt="Version" />
  </a>
</div>

<br/>

📖 **[Read the Documentation](https://leandroluk.github.io/rust-zod)**

---

## Contents
- [Contents](#contents)
- [Getting started](#getting-started)
- [Implementation Status](#implementation-status)
- [About the Project](#about-the-project)
- [Why rust-zod?](#why-rust-zod)
- [Documentation](#documentation)
  - [Primitives & Formats](#primitives--formats)
  - [Objects & Records](#objects--records)
  - [Static Typing & Type Conversion](#static-typing--type-conversion)
  - [Codecs & Pipelines](#codecs--pipelines)
  - [Error Handling & i18n](#error-handling--i18n)
  - [Exporters & Importers](#exporters--importers)
- [Performance & Zero Heap](#performance--zero-heap)
- [License](#license)

---

## Getting started

Add `rust-zod` and `serde` to your `Cargo.toml`:

```toml
[dependencies]
rust-zod = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Quick Example

```rust
use rust_zod::z;
use rust_zod::{Value, ZodError};

// 1. Declare schema using pure Zod v4 closure builders
let user_schema = z::object(|o| {
    o.field("id", z::uuid())
     .field("email", z::email())
     .field("name", z::string().trim().min(2))
     .field("role", z::options(["admin", "member", "guest"]))
})
.meta(|m| m.id("User").description("User profile"));

// 2. Strongly typed target struct
#[derive(Debug, PartialEq, serde::Deserialize)]
struct UserProfile {
    id: String,
    email: String,
    name: String,
    role: String,
}

// 3. Bind domain struct in one line
let typed_schema = user_schema.into_type::<UserProfile>();

// 4. Validate & Parse directly into UserProfile
let input = Value::from(serde_json::json!({
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "email": "alice@example.com",
    "name": " Alice ",
    "role": "admin"
}));

let user: UserProfile = typed_schema.parse(&input)?;
println!("Welcome, {}! Role: {}", user.name, user.role);
```

See [Documentation](#documentation) below for the full API.

---

## Implementation Status

- [x] **M1 - Foundation**: `Value` runtime model (`Null`, `Bool`, `Int`, `Uint`, `Float`, `String`, `Bytes`, `Date`, `Array`, `Object`), 11 `ZodIssueCode` variants, all 4 formatters (`flatten`, `format`, `treeify`, `prettify`), `Locale` (`en`, `pt-BR`, `es`), `Schema` runtime engine.
- [x] **M2 - Primitives & Formats**: Top-level Zod v4 constructors (`z::email()`, `z::uuid()`, `z::jwt()`, `z::iso::datetime()`, `z::int32()`, `z::float64()`, etc.) and `z::coerce::*`.
- [x] **M3 - Options & Collections**: List-only `z::options([...])` (`.extract()`, `.exclude()`), `z::array()`, `z::tuple()`, `z::set()`, `z::map()`.
- [x] **M4 - Objects & Combinators**: `z::object()`, `strict_object()`, `loose_object()`, `.extend()`, `.pick()`, `.omit()`, `.partial()`, `z::record()`, `z::union()`, `z::discriminated_union()`, `z::xor()`, `z::lazy()` (with `OnceLock` recursion safety).
- [x] **M5 - Modifiers & Codecs**: `.optional()`, `.nullable()`, `.nullish()`, `.unwrap()`, `.default()`, `.catch()`, `.refine()`, `.pipe()`, `z::codec(in, out, decode, encode)`, `z::stringbool()`, `z::codecs::*`.
- [x] **M6 - Exporters & Importers**: `to_json_schema()` (Draft 2020-12, Draft-07, OpenAPI 3.0/3.1, `$defs`, `override_with`), `from_json_schema()` reverse importer, `to_graphql_sdl()`.
- [x] **M7 - Low-Level Optimizations**: `Schema<T>` type conversion architecture, serde fast-path bypass for `Value` and primitives, inline path stack (`SmallVec`) and inline small strings (`CompactString`) with no lifetimes in the public API, a counting-allocator benchmark (`cargo run --release --example bench`), and a strict target footprint (< 115 MB).

See `.specs/project/ROADMAP.md` for the full milestone and requirement breakdown.

---

## About the Project

`rust-zod` was built from the ground up by auditing the pure TypeScript source code of **Zod v4** (`colinhacks/zod`, `packages/zod/src/v4/`). It drops all legacy Zod v3 deprecations and adapts TypeScript-specific idioms into clean, ergonomic, idiomatic Rust.

---

## Why rust-zod?

In traditional Rust libraries (such as `validator` or `garde`):
- **Validation occurs after deserialization**: `serde` must instantiate an invalid struct before validation can even run. If a field has the wrong type, serde errors out with a generic message without executing your custom validators or i18n rules.
- **Duplicated DTO structs**: You are forced to create 4 or 5 repetitive structs (`CreateUser`, `UpdateUser`, `PatchUser`) with identical fields just to tweak validation rules.
- **Macros obscure IDE autocomplete**: Writing rules inside attribute strings (`#[validate(custom = "...")]`) deprives you of compiler help and autocomplete.

With `rust-zod`:
- **Closure Builders**: `z::object(|o| ...)` and `.meta(|m| ...)` give you 100% native `rust-analyzer` autocomplete at every step.
- **Dynamic Derivations**: Derive new schemas in one line using `.pick(["id", "email"])`, `.omit(["password"])`, and `.partial()`.
- **First-Class i18n**: Errors are localized automatically into English, Brazilian Portuguese, or Spanish without requiring custom error strings.

---

## Documentation

### Primitives & Formats
- Identifiers: `z::uuid()`, `z::uuidv4()`, `z::uuidv6()`, `z::uuidv7()`, `z::guid()`, `z::nanoid()`, `z::cuid2()`, `z::ulid()`, `z::xid()`, `z::ksuid()`.
- Network & Web: `z::email()`, `z::url()`, `z::http_url()`, `z::hostname()`, `z::ipv4()`, `z::ipv6()`, `z::mac()`, `z::cidrv4()`, `z::cidrv6()`.
- Tokens & Data: `z::base64()`, `z::base64url()`, `z::hex()`, `z::jwt()`, `z::emoji()`, `z::e164()`, `z::credit_card()`, `z::iban()`, `z::currency_code()`.
- ISO Dates: `z::iso::date()`, `z::iso::time()`, `z::iso::datetime()`, `z::iso::duration()`.
- Numbers: `z::number()`, `z::float64()`, `z::float32()`, `z::int()`, `z::int32()`, `z::uint32()`, `z::int64()`, `z::uint64()`, `z::bigint()`.

### Objects & Records
```rust
let schema = z::object(|o| {
    o.field("id", z::uuid())
     .field("bio", z::string().optional())
})
.strict(); // or .loose()

let patch_schema = schema.partial();
let picked = schema.pick(["id"]);
let omitted = schema.omit(["bio"]);
```

### Static Typing & Type Conversion
```rust
let user_schema = z::object(|o| {
    o.field("id", z::uuid())
     .field("age", z::int32().min(18))
}).into_type::<User>();

// .parse() directly returns Result<User, ZodError>
let user: User = user_schema.parse(&payload)?;

// Propagates automatically into arrays and optionals:
let users: Vec<User> = z::array(user_schema.clone()).parse(&payload_list)?;
let maybe_user: Option<User> = user_schema.optional().parse(&payload)?;
```

### Codecs & Pipelines
```rust
let epoch_to_iso = z::codec(
    z::int64(),
    z::iso::date(),
    |v, ctx| v.as_i64().map(|_ts| Value::String("2026-10-27".into())),
    |v, ctx| v.as_str().map(|_s| Value::Int(1700000000000)),
);

let date = epoch_to_iso.decode(&Value::from(1700000000000i64))?;
let epoch = epoch_to_iso.encode(&date)?;
```

### Error Handling & i18n
```rust
// Choose your formatter
let flattened = err.flatten();
let formatted = err.format();
let treeified = err.treeify();
let prettified = err.prettify();

// Scope locale per request
let err_pt = schema.parse_with(&val, ParseOptions::new().locale(Locale::PtBr)).unwrap_err();
```

### Exporters & Importers
```rust
// Export to JSON Schema / OpenAPI 3.1
let json_schema = z::to_json_schema(&schema, JsonSchemaOptions::default());

// Export to GraphQL SDL
let sdl = z::to_graphql_sdl(&schema, GraphqlOptions::default());

// Reverse import JSON Schema
let runtime_schema = z::from_json_schema(&raw_json_schema);
```

---

## Performance & Zero Heap

- **Zero Monomorphization Bloat**: `Schema<T>` delegates runtime validation to non-generic `&SchemaDef` AST nodes, keeping disk usage under ~115 MB.
- **Copy-on-Write Chaining**: Fluent methods (`.min().max().trim()`) use `Arc::make_mut` to modify schemas in-place without heap reallocation.
- **Cycle-Safe Recursion**: Recursive schemas via `z::lazy` memoize evaluation in a thread-safe `OnceLock`.

---

## License

Dual-licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
