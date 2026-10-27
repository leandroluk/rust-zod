# Primitives & Top-Level Formats

Following pure Zod v4 specifications, formats are declared as top-level functions directly on the `z` module, leaving `z::string()` focused strictly on general-purpose text checks and transformations.

---

## 1. Primitives

```rust
use rust_zod::z;

z::null();       // Validates Value::Null
z::boolean();    // Validates Value::Bool
z::any();        // Passes any value untouched
z::unknown();    // Safely validates unknown values
z::never();      // Always fails validation
z::date();       // Validates Unix timestamps / Value::Date
z::json();       // Validates valid JSON strings
z::literal(42);  // Validates exact value
z::literal(["read", "write"]); // Multi-value literal (Zod v4 feature)
```

---

## 2. Top-Level String Formats

### Identifiers
```rust
z::uuid();     // Any valid UUID string
z::uuidv4();   // UUID version 4
z::uuidv6();   // UUID version 6
z::uuidv7();   // UUID version 7
z::guid();     // GUID format
z::nanoid();   // NanoID (21 alphanumeric chars)
z::cuid2();    // CUID v2
z::ulid();     // Crockford Base32 26-char ULID
z::xid();      // 20-char Hex-based XID
z::ksuid();    // 27-char KSUID
```

### Network & Web
```rust
z::email();     // Standard RFC email addresses
z::url();       // Valid URLs (http, https, ftp, etc.)
z::http_url();  // Strictly HTTP and HTTPS URLs
z::hostname();  // RFC 1123 hostnames
z::ipv4();      // IPv4 addresses (e.g. "192.168.1.1")
z::ipv6();      // IPv6 addresses (e.g. "::1")
z::mac();       // MAC addresses (colon or hyphen formatted)
z::cidrv4();    // IPv4 CIDR blocks (e.g. "10.0.0.0/24")
z::cidrv6();    // IPv6 CIDR blocks (e.g. "2001:db8::/32")
```

### Coding & Tokens
```rust
z::base64();         // Standard Base64 strings
z::base64url();      // URL-safe Base64 strings
z::hex();            // Hexadecimal strings
z::jwt();            // Three-segment JSON Web Tokens
z::emoji();          // Valid Emoji sequences
z::e164();           // E.164 phone numbers (e.g. "+14155552671")
z::credit_card();    // Valid credit card numbers passing Luhn check
z::iban();           // International Bank Account Numbers
z::currency_code();  // 3-letter ISO 4217 codes (e.g. "USD", "BRL")
```

### ISO Date & Time Formats (`z::iso::*`)
```rust
z::iso::date();      // YYYY-MM-DD
z::iso::time();      // HH:MM:SS or HH:MM:SS.mmm
z::iso::datetime();  // Full ISO-8601 UTC timestamp
z::iso::duration();  // ISO-8601 duration string (e.g. "P3Y6M4DT12H30M5S")
```

---

## 3. Generic String Checks & Transforms

Format schemas (`z::email()`, `z::uuid()`) inherit all methods from `z::string()`:

```rust
let clean_title = z::string()
    .trim()
    .min(3)
    .max(50)
    .starts_with("prefix:")
    .ends_with(".md")
    .includes("guide")
    .slugify(); // converts "Hello World!" -> "hello-world"
```

---

## 4. Top-Level Numeric Types & Checks

```rust
// Direct numeric types
z::float64(); // or z::number()
z::float32();
z::int();     // or z::int64()
z::int32();
z::uint32();
z::uint64();
z::bigint();

// Numeric bound checks
let age_schema = z::int32()
    .gte(18)           // or .min(18)
    .lt(120)           // or .max(119)
    .positive()        // > 0
    .multiple_of(1);   // integer step
```

---

## 5. Type Coercion (`z::coerce::*`)

Automatically coerce raw values before schema checks:

```rust
z::coerce::string().parse(&Value::from(42))?;        // -> "42"
z::coerce::number().parse(&Value::from("3.1415"))?;  // -> 3.1415
z::coerce::boolean().parse(&Value::from("true"))?;   // -> true
z::coerce::int().parse(&Value::from("100"))?;        // -> 100
```
