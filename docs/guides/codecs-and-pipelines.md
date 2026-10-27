# Modifiers, Refinements, Pipelines & Codecs

Zod v4 introduced first-class bidirectional codecs and refined pipelines. In `rust-zod`, codecs allow seamlessly converting between transportation formats (e.g. JSON strings or Unix timestamps) and domain formats (e.g. native booleans or ISO dates).

---

## 1. Schema Modifiers

```rust
// Default fallback when input is Value::Null
let port = z::int32().default(8080);

// Catch fallback when validation encounters an error
let safe_score = z::float64().positive().catch(0.0);

// Nullish modifier (accepts both null and missing)
let bio = z::string().nullish();

// Unwrap inner schema from optional/nullable
let required_bio = bio.unwrap();
```

---

## 2. Refinements & Fine-Grained Checks

### Predicate Refinement (`.refine()`)
```rust
let even_number = z::int().refine(
    |val| val.as_i64().unwrap_or(1) % 2 == 0,
    "Number must be even"
);
```

### Super Refine (`.super_refine()`)
Access `ParseContext` directly to emit multiple custom issues with custom paths:
```rust
let password_schema = z::string().super_refine(|val, ctx| {
    if let Some(s) = val.as_str() {
        if !s.chars().any(|c| c.is_uppercase()) {
            ctx.add_issue(
                ZodIssueCode::Custom { params: Default::default() },
                Some("Must contain at least one uppercase letter"),
            );
        }
    }
});
```

---

## 3. Pipelines (`.pipe()`) & Transforms (`.transform()`)

```rust
let trimmed_int = z::string()
    .trim()
    .pipe(z::coerce::int());
```

---

## 4. Bidirectional Codecs (`z::codec`)

Codecs allow defining two-way reversible pipelines:
- `.decode(&val)`: validates input format and decodes into output format (equivalent to `.parse()`, returns the typed output).
- `.encode(&val)`: validates output format and encodes in reverse back into input format (returns the raw `Value`).

A codec is a regular `Schema<Out>`: it nests inside objects, arrays, optionals and unions, and `.encode()` walks the
whole tree in reverse. In `a.pipe(b)` the encode order is `b` first, then `a`; checks and refinements run in both
directions. `.transform()` / `.preprocess()` are one-way: reaching them through `.encode()` returns a
`ZodError` whose `failure` is `Failure::Encode(..)` (it is not a `ZodIssue`, and it never panics).

```rust
use rust_zod::{z, Value};

let epoch_to_iso = z::codec(
    z::int64(),           // input schema (transport)
    z::iso::date(),       // output schema (domain)
    |val, ctx| {
        // decode: epoch ms -> ISO Date string
        val.as_i64().map(|_ts| Value::String("2026-10-27".into()))
    },
    |val, ctx| {
        // encode: ISO Date string -> epoch ms
        val.as_str().map(|_s| Value::Int(1700000000000))
    },
);

// Forward decode (typed output):
let date: String = epoch_to_iso.decode(&Value::from(1700000000000i64))?;

// Reverse encode:
let epoch = epoch_to_iso.encode(&Value::from(date))?;
```

### Async steps (`refine_async`, `transform_async`, `z::codec_async`)

Async work is runtime-agnostic (only `core::future`, no `tokio`/`futures` dependency). Async steps run under
`.parse_async()` / `.decode_async()` / `.encode_async()`; calling the synchronous `.parse()` on such a schema returns
`ZodError { failure: Some(Failure::Async), .. }` instead of panicking.

```rust
let unique_email = z::email().refine_async(
    |v| async move { !db_has_email(v.as_str().unwrap()).await },
    "Email already taken",
);
let ok = unique_email.parse_async(&Value::from("a@example.com")).await?;
```

Async callbacks must be deterministic for a given input: the engine may re-run the (synchronous) schema walk
while it resolves chained async steps, but each `(step, input)` pair is awaited only once.

---

## 5. Built-in Codec Presets (`z::codecs::*`)

`rust-zod` ships with ready-to-use codecs for common transportation patterns:

### StringBool (`z::stringbool()`)
Encodes and decodes `"true"/"false"`, `"yes"/"no"`, `"1"/"0"`:
```rust
let flag_codec = z::stringbool()
    .truthy(["yes", "on", "1"])
    .falsy(["no", "off", "0"])
    .build();

assert!(flag_codec.decode(&Value::from("yes"))?);
assert_eq!(flag_codec.encode(&Value::Bool(false))?, Value::from("no"));
```

### Standard Presets
- `z::codecs::string_int()`: Converts between string representation and `i64`.
- `z::codecs::string_number()`: Converts between string representation and `f64`.
- `z::codecs::epoch_date()`: Converts between epoch millisecond timestamp (`i64`) and `Value::Date`.
- `z::codecs::json_string()`: Converts between JSON text and a parsed `Value`.
- `z::codecs::base64_bytes()`: Converts between base64 text and `Value::Bytes`.
