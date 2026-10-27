# Objects, Records & Shape Operations

In Rust, anonymous object literals (`{ name: z.string() }`) do not exist. `rust-zod` adopts the idiomatic **Closure Builder** pattern (`|o| ...`), allowing rich schema definitions with full IDE autocomplete.

---

## 1. Object Constructors

```rust
use rust_zod::z;

// 1. Standard Object (strips unknown keys by default)
let user = z::object(|o| {
    o.field("id", z::uuid())
     .field("name", z::string())
});

// 2. Strict Object (rejects unknown keys with UnrecognizedKeys error)
let strict_user = z::strict_object(|o| {
    o.field("id", z::uuid())
     .field("name", z::string())
});

// 3. Loose Object (preserves unknown keys untouched)
let loose_user = z::loose_object(|o| {
    o.field("id", z::uuid())
     .field("name", z::string())
});
```

---

## 2. Shape Transformations

Derive new schemas dynamically without duplicating fields:

### Extend (`.extend(|o| ...)`)
Add new fields to an existing object schema:
```rust
let admin_schema = user.extend(|o| {
    o.field("permissions", z::array(z::string()))
     .field("super_admin", z::boolean())
});
```

### Pick (`.pick([...])`) & Omit (`.omit([...])`)
Unlike TypeScript which requires object masks `{ id: true }`, Rust takes clean slices:
```rust
// Pick only "id" and "email"
let public_user = user.pick(["id", "email"]);

// Omit sensitive fields
let sanitized_user = user.omit(["password", "secret_token"]);
```

### Partial (`.partial()`) & Required (`.required()`)
```rust
// Make every field optional
let patch_payload = user.partial();

// Make only specific keys optional
let update_payload = user.partial_keys(["bio", "avatar"]);
```

### Keyof (`.keyof()`)
Extracts the schema keys as an options/enum schema:
```rust
let keys = user.keyof(); // Schema accepting "id" | "name"
```

---

## 3. Catchall Schemas

Validate unknown keys using a specific schema instead of simply stripping or preserving them:

```rust
let dynamic_config = z::object(|o| {
    o.field("version", z::int32())
})
.catchall(z::string().min(1));
```

---

## 4. Records

Represent map-like dictionary objects with typed keys and values:

```rust
// Maps string slugs to numeric scores
let score_board = z::record(z::string().slugify(), z::float64());

// Loose record preserving extra dictionary metadata
let loose_dict = z::loose_record(z::string(), z::any());
```

---

## 5. Objects Are Plain Schemas, Recursion Uses `z::lazy`

`z::object(...)`, `z::strict_object(...)` and `z::loose_object(...)` return a regular `Schema<Value>`, so they nest and
compose directly (`z::array(user_schema)`, `user_schema.optional()`, `.meta(...)`, `.into_type::<User>()`).
Shape operations (`.extend`, `.pick`, `.omit`, `.partial`, `.partial_keys`, `.required`, `.required_keys`, `.strict`,
`.loose`, `.strip`, `.catchall`, `.keyof`) are available on any object schema and return a new `Schema<Value>`
(calling them on a non-object schema panics with a clear message, like Zod's `TypeError`).

Recursive schemas are written as a plain function that references itself through `z::lazy`:

```rust
fn category() -> Schema<Value> {
    z::object(|o| {
        o.field("name", z::string())
            .field("sub", z::array(z::lazy(category)).optional())
    })
}
```

`z::lazy` evaluates the function once (`OnceLock`). Capture-free closures and `fn` items share a single lazy node, so
validation stays finite and `to_json_schema` / `to_graphql_sdl` detect the cycle by node identity and emit
`$ref` / a named type. Capturing closures get their own node: give the recursive schema a `.meta(|m| m.id("..."))`
so exporters can name it (a depth guard still guarantees termination).
