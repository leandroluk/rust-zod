# Spec: 02 — Primitives, Top-Level String Formats, Numbers & Coercion

## Summary
Implements Milestone 2 based on [INSIGHT.md](../../../INSIGHT.md):
1. Primitives: `z::string()`, `z::boolean()`, `z::null()`, `z::any()`, `z::unknown()`, `z::never()`, `z::date()`, `z::json()`, `z::literal(...)`.
2. Text checks & transforms on `Schema<String>`: `.min()`, `.max()`, `.length()`, `.nonempty()`, `.regex()`, `.includes()`, `.starts_with()`, `.ends_with()`, `.lowercase()`, `.uppercase()`, `.trim()`, `.to_lower_case()`, `.to_upper_case()`, `.slugify()`.
3. Top-level string formats:
   - Identifiers: `z::uuid()`, `z::uuidv4()`, `z::uuidv6()`, `z::uuidv7()`, `z::guid()`, `z::nanoid()`, `z::cuid2()`, `z::ulid()`, `z::xid()`, `z::ksuid()`.
   - Web & Network: `z::email()`, `z::url()`, `z::http_url()`, `z::hostname()`, `z::ipv4()`, `z::ipv6()`, `z::mac()`, `z::cidrv4()`, `z::cidrv6()`.
   - Coding & Tokens: `z::base64()`, `z::base64url()`, `z::hex()`, `z::jwt()`, `z::emoji()`, `z::e164()`, `z::credit_card()`, `z::iban()`, `z::currency_code()`, `z::hash(...)`, `z::string_format(...)`.
   - ISO Date/Time (`z::iso::*`): `z::iso::date()`, `z::iso::time()`, `z::iso::datetime()`, `z::iso::duration()`.
4. Top-level numeric constructors: `z::number()`, `z::float64()`, `z::float32()`, `z::int()`, `z::int32()`, `z::uint32()`, `z::int64()`, `z::uint64()`, `z::bigint()`.
5. Numeric checks: `.gt()`, `.gte()`/`.min()`, `.lt()`, `.lte()`/`.max()`, `.positive()`, `.nonnegative()`, `.negative()`, `.nonpositive()`, `.multiple_of()`.
6. Coercion: `z::coerce::string()`, `z::coerce::number()`, `z::coerce::int()`, `z::coerce::boolean()`, `z::coerce::bigint()`, `z::coerce::date()`.

## Requirements
- REQ-PRIM-001: Implement primitive constructors `z::null()`, `z::unknown()`, `z::date()`, `z::literal(val)`.
- REQ-PRIM-002: Implement string checks (`.min(n)`, `.max(n)`, `.length(n)`, `.nonempty()`, `.regex(re)`, `.includes(s)`, `.starts_with(s)`, `.ends_with(s)`, `.lowercase()`, `.uppercase()`) and transforms (`.trim()`, `.to_lower_case()`, `.to_upper_case()`, `.slugify()`).
- REQ-PRIM-003: Implement top-level string formats for identifiers (`uuid`, `uuidv4`, `uuidv6`, `uuidv7`, `guid`, `nanoid`, `cuid2`, `ulid`, `xid`, `ksuid`).
- REQ-PRIM-004: Implement top-level string formats for network & web (`email`, `url`, `http_url`, `hostname`, `ipv4`, `ipv6`, `mac`, `cidrv4`, `cidrv6`).
- REQ-PRIM-005: Implement top-level string formats for encoding & tokens (`base64`, `base64url`, `hex`, `jwt`, `emoji`, `e164`, `credit_card`, `iban`, `currency_code`, `hash`, `string_format`).
- REQ-PRIM-006: Implement ISO formats under `z::iso::*` (`date`, `time`, `datetime`, `duration`).
- REQ-PRIM-007: Implement top-level numeric types (`z::number()`, `z::float64()`, `z::float32()`, `z::int()`, `z::int32()`, `z::uint32()`, `z::int64()`, `z::uint64()`, `z::bigint()`) and checks (`gt`, `gte`/`min`, `lt`, `lte`/`max`, `positive`, `nonnegative`, `negative`, `nonpositive`, `multiple_of`).
- REQ-PRIM-008: Implement coercion helpers under `z::coerce::*` (`string`, `number`, `int`, `boolean`, `bigint`, `date`).
