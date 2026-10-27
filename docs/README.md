<div align="center">
  <img src="assets/social-preview.png" alt="rust-zod" width="100%" />
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

`rust-zod` brings the beloved developer experience (DX) of TypeScript's [Zod v4](https://github.com/colinhacks/zod) to the Rust ecosystem: **zero procedural macros**, **zero compile-time code bloat**, **first-class i18n out of the box**, and an introspectable runtime schema model that exports effortlessly to **JSON Schema**, **OpenAPI 3.1**, and **GraphQL SDL**.

---

## Key Features

- 🎯 **Pure Zod v4 API**: Top-level format constructors (`z::email()`, `z::uuid()`, `z::iso::datetime()`, `z::int32()`), `.meta(|m| ...)` closure builders, and zero v3 deprecation baggage.
- 🚀 **Zero Macros, Zero Bloat**: No `#[derive(...)]` proc-macro compilation overhead. Fast compilation times and minimal binary size footprint.
- ⚡ **Strong Static Typing Without Monomorphization Explosion**: `Schema<T>` exposes typed `.parse(&val) -> Result<T, ZodError>` for any domain struct via `.into_type::<T>()`, propagating automatically into `z::array` and `.optional()`.
- 🌍 **Built-in i18n Error Catalog**: Full 11-variant Zod v4 issue taxonomy with pre-translated messages in English (`en`), Brazilian Portuguese (`pt-BR`), and Spanish (`es`).
- 🔀 **Bidirectional Codecs**: `z::codec(in, out, decode, encode)` and presets (`z::stringbool()`, `z::codecs::*`) for reverse encoding and deserialization pipelines.
- 📐 **Universal Schema Exporters**: Generate JSON Schema (Draft 2020-12, Draft-07), OpenAPI 3.0/3.1, and GraphQL SDL with inverted nullability (`!`).
- 🔄 **Reverse JSON Schema Importer**: `z::from_json_schema(&val)` builds dynamic runtime validation schemas from external schema specs.

---

## Quickstart

Add `rust-zod` and `serde` to your `Cargo.toml`:

```toml
[dependencies]
rust-zod = "0.1"
serde = { version = "1.0", features = ["derive"] }
```

### 1. Declaring Schemas with Closure Builders

```rust
use rust_zod::z;

let user_schema = z::object(|o| {
    o.field("id", z::uuid())
     .field("email", z::email())
     .field("name", z::string().min(2).trim())
     .field("age", z::int32().min(18).optional())
     .field("role", z::options(["admin", "member", "guest"]))
})
.meta(|m| {
    m.id("User")
     .title("User Profile")
     .description("Registered system user entity")
});
```

### 2. Validating Payloads

```rust
use rust_zod::{Value, ZodError};

let input = Value::from(serde_json::json!({
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "email": "alice@example.com",
    "name": " Alice ",
    "age": 28,
    "role": "admin"
}));

// Synchronous validation returning typed Result
let validated_user = user_schema.parse(&input)?;
```

### 3. Parsing Directly into Domain Structs

```rust
#[derive(Debug, serde::Deserialize)]
struct UserProfile {
    id: String,
    email: String,
    name: String,
    age: Option<i32>,
    role: String,
}

// Bind domain struct directly in one line:
let typed_schema = user_schema.into_type::<UserProfile>();

// .parse() returns Result<UserProfile, ZodError>:
let profile: UserProfile = typed_schema.parse(&input)?;
println!("Welcome, {}! Role: {}", profile.name, profile.role);
```

---

## Why rust-zod?

In traditional Rust web development, libraries like `validator` or `garde` rely heavily on derive macros. This creates significant friction:

1. **Validation After Instantiation**: With derive macros, `serde` must first instantiate an invalid struct before validation even runs. If the JSON structure is malformed, `serde` errors out immediately with generic error messages without running your validation rules or i18n logic.
2. **Duplicated DTO Structs**: Deriving different validation rules for Create, Update, and Patch endpoints forces developers to write 4 or 5 nearly identical structs. With `rust-zod`, you simply derive schemas with `.omit(...)`, `.pick(...)`, and `.partial()`.
3. **IDE Autocomplete (No Macro DSL Strings)**: Instead of programming validation logic inside string attributes (`#[validate(custom(function = "..."))]`), you get 100% full compiler autocomplete from `rust-analyzer` at every step.

---

## Zod v4 vs. rust-zod

| Zod v4 (TypeScript) | rust-zod (Rust) | Notes |
| :--- | :--- | :--- |
| `z.object({ name: z.string() })` | `z::object(\|o\| o.field("name", z::string()))` | Closure builder solves lack of anonymous object literals |
| `z.enum(["admin", "user"])` | `z::options(["admin", "user"])` | Avoids Rust reserved keyword `enum` |
| `z.string().email()` *(v3 deprecated)* | `z::email()` | Pure Zod v4 top-level constructors |
| `schema.safeParse(val)` | `schema.parse(&val)` | Native Rust `Result` replaces JS exceptions |
| `schema.describe("...")` | `schema.meta(\|m\| m.description("..."))` | Unifies all metadata into a single builder |
| `z.codec(in, out, { decode, encode })` | `z::codec(in, out, decode_fn, encode_fn)` | Type-safe bidirectional transformation |

---

## Exporting to JSON Schema & GraphQL

```rust
use rust_zod::{z, JsonSchemaOptions, JsonSchemaTarget, GraphqlOptions};

// 1. Export to JSON Schema / OpenAPI 3.1
let json_schema = z::to_json_schema(&user_schema, JsonSchemaOptions::default());

// 2. Export to GraphQL SDL
let sdl = z::to_graphql_sdl(&user_schema, GraphqlOptions::default());
println!("{}", sdl);
/*
type User {
  id: ID!
  email: String!
  name: String!
  age: Int
  role: String!
}
*/
```
