# Error Handling & Internationalization (i18n)

`rust-zod` adheres strictly to the 11-variant issue taxonomy of Zod v4, providing rich path tracking, 4 official formatters, and native internationalization.

---

## 1. The 11 Zod v4 Issues (`ZodIssueCode`)

1. `InvalidType { expected, received }`
2. `TooBig { origin, maximum, inclusive, exact }`
3. `TooSmall { origin, minimum, inclusive, exact }`
4. `InvalidFormat { format, pattern, prefix, suffix, includes }`
5. `NotMultipleOf { divisor }`
6. `UnrecognizedKeys { keys }`
7. `InvalidUnion { errors, discriminator, options, inclusive, matches }`
8. `InvalidKey { origin, issues }`
9. `InvalidElement { origin, key, issues }`
10. `InvalidValue { values }`
11. `Custom { params }`

---

## 2. The 4 Error Formatters

On any `ZodError`, you can choose from 4 built-in formatting utilities:

### Flatten (`.flatten()`)
Separates top-level form errors from nested field errors:
```rust
let flattened = err.flatten();
println!("Field errors: {:?}", flattened.field_errors);
// { "email": ["Invalid email"], "age": ["Expected number, received string"] }
```

### Format (`.format()`)
Produces a nested tree where each field contains `_errors: Vec<String>`:
```rust
let formatted = err.format();
```

### Treeify (`.treeify()`)
Produces a structured schema hierarchy with `errors`, `properties`, and `items`:
```rust
let tree = err.treeify();
```

### Prettify (`.prettify()`)
Formats a terminal-friendly string with error markers and property paths:
```rust
println!("{}", err.prettify());
/*
✖ Expected string, received number
  → at users.0.name

✖ Invalid email
  → at users.0.email
*/
```

---

## 3. Pre-Translated Locales (`Locale`)

Default validation messages are pre-configured in 3 locales:
- `Locale::En` (English — default)
- `Locale::PtBr` (Brazilian Portuguese)
- `Locale::Es` (Spanish)

### Global Locale Switch
```rust
use rust_zod::{z, Locale};

z::set_locale(Locale::PtBr);
```

### Scoped Locale Switch per Request
```rust
use rust_zod::{ParseOptions, Locale};

let options = ParseOptions::new().locale(Locale::PtBr);
let err = schema.parse_with(&payload, options).unwrap_err();
assert_eq!(err.issues[0].message, "Esperado string, recebido number");
```

---

## 4. Custom Messages (`.error`, `_with`) and Non-Validation Failures

By default no method needs a message: the active locale's catalog generates it. To override one:

```rust
// Override the message of a schema's own issues (nested children keep theirs):
z::email().error("E-mail inválido");

// Override (and optionally abort) a single check with a closure builder:
z::string().min_with(8, |e| e.error("A senha deve ter pelo menos 8 caracteres").abort(true));
```

`abort(true)` stops later checks of the same schema from running once this one fails. The `_with` variants exist for
every bound/format check (`min_with`, `max_with`, `length_with`, `gt_with`, `gte_with`, `lt_with`, `lte_with`,
`multiple_of_with`, `regex_with`, `includes_with`, `starts_with_with`, `ends_with_with`).

Some failures are **not** data issues, so they are not `ZodIssue`s. They are reported in `ZodError::failure`
(`issues` is empty) instead of panicking:

| `failure` | When |
| :--- | :--- |
| `Failure::Async` | an async step (`refine_async`, `transform_async`, `z::codec_async`) was reached by a synchronous `.parse()` / `.decode()` / `.encode()` |
| `Failure::Encode(msg)` | a one-way step (`.transform()`, `.preprocess()`) was reached by `.encode()` |
