# JSON Schema & OpenAPI Exporters

Because `rust-zod` maintains an introspectable runtime AST (`Arc<SchemaDef>`), schemas can be exported directly to standard JSON Schema specifications or OpenAPI 3.0 / 3.1 definitions.

---

## 1. Exporting Schemas

```rust
use rust_zod::{z, JsonSchemaOptions, JsonSchemaTarget, SchemaIO};

let schema = z::object(|o| {
    o.field("id", z::uuid())
     .field("email", z::email())
     .field("tags", z::array(z::string()))
})
.meta(|m| {
    m.id("UserAccount")
     .title("User Account")
     .description("User account details")
});

// Export using default Draft 2020-12
let json_schema = z::to_json_schema(&schema, JsonSchemaOptions::default());
```

---

## 2. Supported Targets

Configure the output target via `JsonSchemaOptions`:

- `JsonSchemaTarget::Draft202012` (Default)
- `JsonSchemaTarget::Draft07`
- `JsonSchemaTarget::Draft04`
- `JsonSchemaTarget::OpenApi30`
- `JsonSchemaTarget::OpenApi31`

```rust
let openapi_options = JsonSchemaOptions::new()
    .target(JsonSchemaTarget::OpenApi31)
    .io(SchemaIO::Output);

let openapi_spec = z::to_json_schema(&schema, openapi_options);
```

---

## 3. Recursive Schemas & `$defs`

When schemas are annotated with `.meta(|m| m.id("..."))` or contain recursive `z::lazy` nodes, `to_json_schema` automatically generates `#/$defs/...` definitions, preventing cycles and infinite recursion:

```rust
let category = z::object(|o| {
    o.field("name", z::string())
})
.meta(|m| m.id("Category"));
```

---

## 4. Reverse Importer (`z::from_json_schema`)

Convert any external JSON Schema document directly into a dynamic `rust-zod` `Schema`:

```rust
let raw_schema_json = serde_json::json!({
    "type": "object",
    "required": ["username"],
    "properties": {
        "username": { "type": "string" }
    }
});

let runtime_schema = z::from_json_schema(&Value::from(raw_schema_json));

assert!(runtime_schema.parse(&Value::from(serde_json::json!({ "username": "alice" }))).is_ok());
assert!(runtime_schema.parse(&Value::from(serde_json::json!({ "username": 123 }))).is_err());
```
