# Example: REST API Payload Validation

This example demonstrates how to use `rust-zod` to validate, sanitize, and deserialize incoming JSON HTTP payloads in a web service.

---

## 1. Domain Struct & Schema Declaration

```rust
use rust_zod::{z, Schema, Value, ZodError};
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
pub struct CreateUserDTO {
    pub email: String,
    pub username: String,
    pub age: i32,
    pub role: String,
}

pub fn create_user_schema() -> Schema<CreateUserDTO> {
    z::object(|o| {
        o.field("email", z::email())
         .field("username", z::string().trim().min(3).max(30).to_lower_case())
         .field("age", z::int32().gte(18).lte(120))
         .field("role", z::options(["user", "moderator", "admin"]))
    })
    .into_type::<CreateUserDTO>()
}
```

---

## 2. Validation Handler Function

```rust
use rust_zod::{Locale, ParseOptions};

pub fn handle_registration(payload: &serde_json::Value, client_lang: &str) -> Result<CreateUserDTO, serde_json::Value> {
    let schema = create_user_schema();
    let value_input = Value::from(payload.clone());

    // Resolve locale from Accept-Language header
    let locale = match client_lang {
        "pt-BR" | "pt" => Locale::PtBr,
        "es" => Locale::Es,
        _ => Locale::En,
    };

    let options = ParseOptions::new().locale(locale);

    match schema.parse_with(&value_input, options) {
        Ok(dto) => Ok(dto),
        Err(err) => {
            // Return structured flattened field errors to the client
            let flattened = err.flatten();
            Err(serde_json::json!({
                "status": "error",
                "form_errors": flattened.form_errors,
                "field_errors": flattened.field_errors,
            }))
        }
    }
}
```

---

## 3. Test Cases

```rust
#[test]
fn test_registration_validation() {
    // Valid input
    let valid_json = serde_json::json!({
        "email": "dev@example.com",
        "username": "  RustDev  ",
        "age": 25,
        "role": "user"
    });

    let user = handle_registration(&valid_json, "en").unwrap();
    assert_eq!(user.username, "rustdev"); // auto-trimmed and lowercased
    assert_eq!(user.age, 25);

    // Invalid input (underage + bad role) with Portuguese error messages
    let invalid_json = serde_json::json!({
        "email": "not-an-email",
        "username": "a",
        "age": 16,
        "role": "superadmin"
    });

    let errors = handle_registration(&invalid_json, "pt-BR").unwrap_err();
    let field_errors = errors.get("field_errors").unwrap();

    assert!(field_errors.get("email").is_some());
    assert!(field_errors.get("username").is_some());
    assert!(field_errors.get("age").is_some());
}
```
