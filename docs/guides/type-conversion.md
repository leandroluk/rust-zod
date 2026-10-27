# Static Typing & Type Conversion

`rust-zod` solves the hardest challenge of schema validation in Rust: **how to provide rich compile-time static types without exploding the `target/` directory with thousands of monomorphized generic variants**.

---

## 1. The `Schema<T>` Architecture

```rust
pub struct Schema<T = Value> {
    def: Arc<SchemaDef>,
    _marker: PhantomData<fn() -> T>,
}
```

- **Runtime AST Erased (`Arc<SchemaDef>`)**: The underlying validation engine, internationalization, and export logic operate on a single non-generic AST tree.
- **Phantom Output Type `T`**: The generic parameter `T` has zero size in memory and acts solely at the final boundary when deserializing data into domain types.
- **Zero Target Bloat**: Chaining 10 validation methods (`z::string().trim().min(3).max(20).to_lower_case()`) **remains `Schema<String>`**, never creating nested types like `Trim<Min<Max<StringSchema>>>`.

---

## 2. Direct Typed Return via `.parse()`

When a schema is typed with any Rust type implementing `serde::de::DeserializeOwned`, `.parse()` returns that type directly:

```rust
#[derive(Debug, serde::Deserialize)]
struct User {
    id: String,
    age: i32,
}

let user_schema = z::object(|o| {
    o.field("id", z::uuid())
     .field("age", z::int32().min(18))
})
.into_type::<User>();

// .parse() returns Result<User, ZodError> directly:
let user: User = user_schema.parse(&payload)?;
println!("User ID: {}, Age: {}", user.id, user.age);
```

---

## 3. Automatic Type Propagation

When composing schemas, the static type flows automatically through wrappers:

### Arrays (`z::array`)
Wrapping a `Schema<T>` in `z::array` produces **`Schema<Vec<T>>`**:
```rust
let users_schema: Schema<Vec<User>> = z::array(user_schema.clone());

let users: Vec<User> = users_schema.parse(&payload_array)?;
```

### Optionals (`.optional()` & `.nullable()`)
Calling `.optional()` on a `Schema<T>` produces **`Schema<Option<T>>`**:
```rust
let maybe_user_schema: Schema<Option<User>> = user_schema.optional();

let user: Option<User> = maybe_user_schema.parse(&payload)?;
```

---

## 4. Universal Deserialization: `.parse_into::<Target>()`

If you want to parse data from a schema into a different target type on the fly without rebinding the schema, use `.parse_into::<Target>()`:

```rust
// Parse from options directly into an enum
let role: Role = z::options(["admin", "user"]).parse_into(&role_val)?;

// Parse from tuple into a Rust tuple pair
let coords: (f64, f64) = z::tuple((z::float64(), z::float64())).parse_into(&coords_val)?;
```

---

## 5. Raw Value Access: `.parse_value()`

If you need the validated AST `Value` without running serde deserialization, call `.parse_value(&val)`:

```rust
let raw_val: Value = user_schema.parse_value(&payload)?;
assert!(raw_val.is_object());
```
