# Getting Started with rust-zod

This guide walks you through setting up `rust-zod`, declaring your first schemas, parsing input payloads, and handling validation results idiomatically.

---

## 1. Installation

Add `rust-zod` to your `Cargo.toml`:

```toml
[dependencies]
rust-zod = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

## 2. Basic Validation

Import the `z` facade to access top-level constructors:

```rust
use rust_zod::z;
use rust_zod::Value;

// Simple string validation with text checks
let username_schema = z::string()
    .trim()
    .min(3)
    .max(20)
    .to_lower_case();

// Parsing returns a native Result<String, ZodError>
let result = username_schema.parse(&Value::from("  Alice_99  "));

match result {
    Ok(username) => println!("Valid username: {}", username), // "alice_99"
    Err(err) => eprintln!("Validation failed: {}", err),
}
```

---

## 3. Parsing Values vs. Structs

`rust-zod` offers two primary ways to consume validated data:

### Direct Return via `.parse()`
When using built-in schemas, `.parse()` returns the strongly typed output directly:
- `z::string().parse(...)` returns `Result<String, ZodError>`
- `z::number().parse(...)` returns `Result<f64, ZodError>`
- `z::boolean().parse(...)` returns `Result<bool, ZodError>`

### Deserializing into Target Types via `.parse_into::<T>()`
Any schema can deserialize into arbitrary Rust structs or enums:

```rust
#[derive(Debug, PartialEq, serde::Deserialize)]
enum AccountRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "user")]
    User,
}

let role_schema = z::options(["admin", "user"]);
let role: AccountRole = role_schema.parse_into(&Value::from("admin"))?;
assert_eq!(role, AccountRole::Admin);
```

---

## 4. Internationalization (i18n)

Error messages are localized automatically based on the active locale.

### Setting Global Default Locale
```rust
use rust_zod::{z, Locale, Value};

// Switch global default to Brazilian Portuguese
z::set_locale(Locale::PtBr);

let schema = z::string().min(5);
let err = schema.parse(&Value::from("hi")).unwrap_err();
println!("{}", err.issues[0].message);
// "Deve conter no mínimo 5 caracteres"
```

### Overriding Locale per Request
In multi-tenant or web server environments, pass `ParseOptions` to avoid modifying global state:

```rust
use rust_zod::{ParseOptions, Locale};

let options = ParseOptions::new().locale(Locale::Es);
let err_es = schema.parse_with(&Value::from("hi"), options).unwrap_err();
// Localized in Spanish for this request only
```
