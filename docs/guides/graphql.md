# GraphQL SDL Generator

`rust-zod` can serialize schemas directly into **GraphQL Schema Definition Language (SDL)** strings, with automatic type naming and inverted nullability.

---

## 1. Output Types

By default, `to_graphql_sdl` exports object schemas as GraphQL `type` definitions:

```rust
use rust_zod::{z, GraphqlOptions};

let user_schema = z::object(|o| {
    o.field("id", z::uuid())
     .field("name", z::string())
     .field("bio", z::string().optional())
     .field("tags", z::array(z::string()))
})
.meta(|m| m.id("User"));

let sdl = z::to_graphql_sdl(&user_schema, GraphqlOptions::default());
println!("{}", sdl);
```

Generated SDL:
```graphql
type User {
  id: ID!
  name: String!
  bio: String
  tags: [String!]!
}
```

---

## 2. Inverted Nullability

In GraphQL, fields without an exclamation mark (`!`) are nullable by default. In Zod, fields are required by default. `rust-zod` automatically handles this inversion:
- **Required fields** receive `!` (e.g. `String!`, `ID!`, `[String!]!`).
- **Optional/nullable fields** omit `!` (e.g. `String`).

---

## 3. Input Types (`input`)

Generate GraphQL mutation inputs using `GraphqlTarget::Input`:

```rust
use rust_zod::{GraphqlOptions, GraphqlTarget};

let input_opts = GraphqlOptions::new()
    .target(GraphqlTarget::Input)
    .type_name("UserInput");

let input_sdl = z::to_graphql_sdl(&user_schema, input_opts);
```

Generated SDL:
```graphql
input UserInput {
  id: ID!
  name: String!
  bio: String
  tags: [String!]!
}
```

---

## 4. Enums

Schemas created with `z::options([...])` export directly as GraphQL `enum`:

```rust
let role_schema = z::options(["ADMIN", "MEMBER", "GUEST"])
    .meta(|m| m.id("Role"));

let enum_sdl = z::to_graphql_sdl(&role_schema, GraphqlOptions::default());
```

Generated SDL:
```graphql
enum Role {
  ADMIN
  MEMBER
  GUEST
}
```
