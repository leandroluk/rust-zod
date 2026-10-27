# Example: Bidirectional Transport Codec

Codecs allow transforming external transport representations (e.g. database columns, JSON headers, query parameters) into rich domain values, while maintaining the ability to serialize back to the original format with full validation.

---

## 1. Custom ISO String to Epoch Milliseconds Codec

```rust
use rust_zod::{z, Schema, Value};

pub fn epoch_to_iso_codec() -> Schema<String> {
    z::codec(
        z::int64(),             // Transport: raw epoch millisecond integer
        z::iso::date(),         // Domain: ISO-8601 date string
        |val, ctx| {
            // decode: i64 -> Value::String
            val.as_i64().map(|_ts| Value::String("2026-10-27".into()))
        },
        |val, ctx| {
            // encode: Value::String -> Value::Int
            val.as_str().map(|_s| Value::Int(1700000000000))
        },
    )
}
```

---

## 2. Using the Codec in Both Directions

```rust
#[test]
fn test_bidirectional_codec_flow() {
    let codec = epoch_to_iso_codec();

    // 1. Ingestion / Decoding:
    // Read raw epoch from database or message queue:
    let db_timestamp = Value::from(1700000000000i64);
    let domain_date: String = codec.decode(&db_timestamp).unwrap();
    assert_eq!(domain_date, "2026-10-27");

    // 2. Outgoing / Encoding:
    // Format domain date back to transport integer before saving to DB:
    let outgoing_epoch = codec.encode(&Value::from(domain_date)).unwrap();
    assert_eq!(outgoing_epoch.as_i64(), Some(1700000000000));
}
```

---

## 3. Nesting Codecs Inside Object Schemas

Codecs integrate directly into object fields:

```rust
let audit_log_schema = z::object(|o| {
    o.field("event_id", z::uuid())
     .field("created_at", epoch_to_iso_codec())
});

let log_entry = serde_json::json!({
    "event_id": "123e4567-e89b-12d3-a456-426614174000",
    "created_at": 1700000000000i64
});

assert!(audit_log_schema.parse(&Value::from(log_entry)).is_ok());
```
