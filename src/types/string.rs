// @spec REQ-PRIM-002
// @spec REQ-PRIM-003
// @spec REQ-PRIM-004
// @spec REQ-PRIM-005
// @spec REQ-PRIM-006
use super::{too_big, too_small};
use crate::error::{SizeOrigin, ZodIssueCode};
use crate::schema::{Check, CheckOpts, Schema, SchemaDef};
use crate::value::Value;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

fn str_of(v: &Value) -> Option<&str> {
    match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn invalid_format(
    format: &str,
    prefix: Option<String>,
    suffix: Option<String>,
    includes: Option<String>,
    pattern: Option<String>,
) -> ZodIssueCode {
    ZodIssueCode::InvalidFormat {
        format: format.into(),
        pattern,
        prefix,
        suffix,
        includes,
    }
}

impl Schema<String> {
    /// Enforce minimum character length.
    pub fn min(self, minimum: usize) -> Self {
        self.min_with(minimum, |e| e)
    }

    pub fn min_with(self, minimum: usize, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.push_check(
            Check::Min { value: minimum as f64, inclusive: true },
            opts(CheckOpts::default()),
            move |v| match str_of(v) {
                Some(s) if s.chars().count() < minimum => {
                    Some((too_small(SizeOrigin::String, minimum as f64, true, false), None))
                }
                _ => None,
            },
        )
    }

    /// Enforce maximum character length.
    pub fn max(self, maximum: usize) -> Self {
        self.max_with(maximum, |e| e)
    }

    pub fn max_with(self, maximum: usize, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.push_check(
            Check::Max { value: maximum as f64, inclusive: true },
            opts(CheckOpts::default()),
            move |v| match str_of(v) {
                Some(s) if s.chars().count() > maximum => {
                    Some((too_big(SizeOrigin::String, maximum as f64, true, false), None))
                }
                _ => None,
            },
        )
    }

    /// Enforce exact character length.
    pub fn length(self, len: usize) -> Self {
        self.length_with(len, |e| e)
    }

    pub fn length_with(self, len: usize, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        self.push_check(Check::Length(len), opts(CheckOpts::default()), move |v| match str_of(v) {
            Some(s) if s.chars().count() != len => {
                Some((too_small(SizeOrigin::String, len as f64, true, true), None))
            }
            _ => None,
        })
    }

    /// Disallow empty string (shorthand for `.min(1)`).
    pub fn nonempty(self) -> Self {
        self.min(1)
    }

    /// Require the string to match a regular expression (compiled once, when the schema is built).
    #[cfg(feature = "regex")]
    pub fn regex(self, pattern: impl AsRef<str>) -> Self {
        self.regex_with(pattern, |e| e)
    }

    #[cfg(feature = "regex")]
    pub fn regex_with(self, pattern: impl AsRef<str>, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        let source = pattern.as_ref().to_string();
        let re = regex::Regex::new(&source).unwrap_or_else(|e| panic!("invalid regex `{}`: {}", source, e));
        let src = source.clone();
        self.push_check(Check::Pattern(source), opts(CheckOpts::default()), move |v| match str_of(v) {
            Some(s) if !re.is_match(s) => Some((invalid_format("regex", None, None, None, Some(src.clone())), None)),
            _ => None,
        })
    }

    /// Require substring inclusion.
    pub fn includes(self, expected: impl Into<String>) -> Self {
        self.includes_with(expected, |e| e)
    }

    pub fn includes_with(self, expected: impl Into<String>, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        let expected = expected.into();
        self.push_check(Check::Includes(expected.clone()), opts(CheckOpts::default()), move |v| match str_of(v) {
            Some(s) if !s.contains(&expected) => Some((
                invalid_format("string", None, None, Some(expected.clone()), None),
                Some(format!("String must include '{}'", expected)),
            )),
            _ => None,
        })
    }

    /// Require string to start with prefix.
    pub fn starts_with(self, prefix: impl Into<String>) -> Self {
        self.starts_with_with(prefix, |e| e)
    }

    pub fn starts_with_with(self, prefix: impl Into<String>, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        let prefix = prefix.into();
        self.push_check(Check::StartsWith(prefix.clone()), opts(CheckOpts::default()), move |v| match str_of(v) {
            Some(s) if !s.starts_with(&prefix) => Some((
                invalid_format("string", Some(prefix.clone()), None, None, None),
                Some(format!("String must start with '{}'", prefix)),
            )),
            _ => None,
        })
    }

    /// Require string to end with suffix.
    pub fn ends_with(self, suffix: impl Into<String>) -> Self {
        self.ends_with_with(suffix, |e| e)
    }

    pub fn ends_with_with(self, suffix: impl Into<String>, opts: impl FnOnce(CheckOpts) -> CheckOpts) -> Self {
        let suffix = suffix.into();
        self.push_check(Check::EndsWith(suffix.clone()), opts(CheckOpts::default()), move |v| match str_of(v) {
            Some(s) if !s.ends_with(&suffix) => Some((
                invalid_format("string", None, Some(suffix.clone()), None, None),
                Some(format!("String must end with '{}'", suffix)),
            )),
            _ => None,
        })
    }

    /// Enforce all lowercase characters.
    pub fn lowercase(self) -> Self {
        self.push_check(Check::Lowercase, CheckOpts::default(), |v| match str_of(v) {
            Some(s) if s.chars().any(|c| c.is_uppercase()) => Some((
                invalid_format("lowercase", None, None, None, None),
                Some("String must be lowercase".to_string()),
            )),
            _ => None,
        })
    }

    /// Enforce all uppercase characters.
    pub fn uppercase(self) -> Self {
        self.push_check(Check::Uppercase, CheckOpts::default(), |v| match str_of(v) {
            Some(s) if s.chars().any(|c| c.is_lowercase()) => Some((
                invalid_format("uppercase", None, None, None, None),
                Some("String must be uppercase".to_string()),
            )),
            _ => None,
        })
    }

    fn map_string(self, f: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        let inner = self.validator.clone();
        self.with_validator(Arc::new(move |val, ctx| {
            let res = inner(val, ctx)?;
            match res {
                Value::String(s) => Some(Value::String((f(&s)).into())),
                other => Some(other),
            }
        }))
    }

    /// Transform: trim leading and trailing whitespace.
    pub fn trim(self) -> Self {
        self.map_string(|s| s.trim().to_string())
    }

    /// Transform: collapse runs of whitespace into single spaces
    /// (full Unicode normalization forms are not available without extra dependencies).
    pub fn normalize(self) -> Self {
        self.map_string(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    /// Transform: convert to lower case.
    pub fn to_lower_case(self) -> Self {
        self.map_string(|s| s.to_lowercase())
    }

    /// Transform: convert to upper case.
    pub fn to_upper_case(self) -> Self {
        self.map_string(|s| s.to_uppercase())
    }

    /// Transform: convert text to URL slug.
    pub fn slugify(self) -> Self {
        self.map_string(|s| {
            let mut slug = String::new();
            let mut last_dash = false;
            for ch in s.chars() {
                if ch.is_alphanumeric() {
                    for lower in ch.to_lowercase() {
                        slug.push(lower);
                    }
                    last_dash = false;
                } else if (ch == ' ' || ch == '-' || ch == '_') && !last_dash && !slug.is_empty() {
                    slug.push('-');
                    last_dash = true;
                }
            }
            if slug.ends_with('-') {
                slug.pop();
            }
            slug
        })
    }
}

// Top-level string format helper
pub fn string_format(
    format_name: &'static str,
    check_fn: impl Fn(&str) -> bool + Send + Sync + 'static,
) -> Schema<String> {
    Schema::new(
        SchemaDef::String {
            format: Some(format_name.into()),
            checks: Vec::new(),
        },
        move |val, ctx| match val {
        Value::String(s) => {
            if check_fn(s) {
                Some(Value::String(s.clone()))
            } else {
                ctx.add_issue(
                    ZodIssueCode::InvalidFormat {
                        format: format_name.into(),
                        pattern: None,
                        prefix: None,
                        suffix: None,
                        includes: None,
                    },
                    None,
                );
                None
            }
        }
        _ => {
            ctx.add_issue(
                ZodIssueCode::InvalidType {
                    expected: "string".into(),
                    received: val.type_name().into(),
                },
                None,
            );
            None
        }
    })
}

// ----------------------------------------------------------------------------
// Identifiers Formats
// ----------------------------------------------------------------------------

fn is_hex_char(c: char) -> bool {
    c.is_ascii_hexdigit()
}

fn check_uuid(s: &str, expected_version: Option<char>) -> bool {
    if s.len() != 36 {
        return false;
    }
    let b = s.as_bytes();
    if b[8] != b'-' || b[13] != b'-' || b[18] != b'-' || b[23] != b'-' {
        return false;
    }
    for (i, &ch) in b.iter().enumerate() {
        if i == 8 || i == 13 || i == 18 || i == 23 {
            continue;
        }
        if !is_hex_char(ch as char) {
            return false;
        }
    }
    if let Some(ver) = expected_version {
        if (b[14] as char) != ver {
            return false;
        }
    }
    true
}

pub fn uuid() -> Schema<String> {
    string_format("uuid", |s| check_uuid(s, None))
}

pub fn uuidv4() -> Schema<String> {
    string_format("uuidv4", |s| check_uuid(s, Some('4')))
}

pub fn uuidv6() -> Schema<String> {
    string_format("uuidv6", |s| check_uuid(s, Some('6')))
}

pub fn uuidv7() -> Schema<String> {
    string_format("uuidv7", |s| check_uuid(s, Some('7')))
}

pub fn guid() -> Schema<String> {
    string_format("guid", |s| check_uuid(s, None))
}

pub fn nanoid() -> Schema<String> {
    string_format("nanoid", |s| {
        s.len() == 21 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

pub fn cuid2() -> Schema<String> {
    string_format("cuid2", |s| {
        !s.is_empty() && s.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

pub fn ulid() -> Schema<String> {
    string_format("ulid", |s| {
        s.len() == 26 && s.chars().all(|c| {
            matches!(c, '0'..='9' | 'A'..='H' | 'J'..='K' | 'M'..='N' | 'P'..='T' | 'V'..='Z' |
                        'a'..='h' | 'j'..='k' | 'm'..='n' | 'p'..='t' | 'v'..='z')
        })
    })
}

pub fn xid() -> Schema<String> {
    string_format("xid", |s| {
        s.len() == 20 && s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='v'))
    })
}

pub fn ksuid() -> Schema<String> {
    string_format("ksuid", |s| {
        s.len() == 27 && s.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

// ----------------------------------------------------------------------------
// Network & Web Formats
// ----------------------------------------------------------------------------

pub fn email() -> Schema<String> {
    string_format("email", |s| {
        if s.len() < 3 || s.len() > 254 {
            return false;
        }
        let parts: Vec<&str> = s.split('@').collect();
        if parts.len() != 2 {
            return false;
        }
        let (local, domain) = (parts[0], parts[1]);
        if local.is_empty() || local.len() > 64 || domain.is_empty() || !domain.contains('.') {
            return false;
        }
        if domain.starts_with('.') || domain.ends_with('.') {
            return false;
        }
        true
    })
}

pub fn url() -> Schema<String> {
    string_format("url", |s| {
        s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://")
    })
}

pub fn http_url() -> Schema<String> {
    string_format("http_url", |s| {
        s.starts_with("http://") || s.starts_with("https://")
    })
}

pub fn hostname() -> Schema<String> {
    string_format("hostname", |s| {
        !s.is_empty() && s.len() <= 253 && !s.starts_with('-') && !s.ends_with('-') && !s.contains("..")
    })
}

pub fn ipv4() -> Schema<String> {
    string_format("ipv4", |s| {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        for part in parts {
            if part.is_empty() || (part.len() > 1 && part.starts_with('0')) {
                return false;
            }
            match part.parse::<u8>() {
                Ok(_) => (),
                Err(_) => return false,
            }
        }
        true
    })
}

pub fn ipv6() -> Schema<String> {
    string_format("ipv6", |s| {
        s.contains(':') && s.chars().all(|c| is_hex_char(c) || c == ':')
    })
}

pub fn mac() -> Schema<String> {
    string_format("mac", |s| {
        let parts: Vec<&str> = if s.contains(':') {
            s.split(':').collect()
        } else if s.contains('-') {
            s.split('-').collect()
        } else {
            return false;
        };
        parts.len() == 6 && parts.iter().all(|p| p.len() == 2 && p.chars().all(is_hex_char))
    })
}

pub fn cidrv4() -> Schema<String> {
    string_format("cidrv4", |s| {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return false;
        }
        let (ip, prefix) = (parts[0], parts[1]);
        let valid_ip = ipv4_check(ip);
        let valid_prefix = prefix.parse::<u8>().is_ok_and(|p| p <= 32);
        valid_ip && valid_prefix
    })
}

fn ipv4_check(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    for part in parts {
        if part.is_empty() || (part.len() > 1 && part.starts_with('0')) {
            return false;
        }
        if part.parse::<u8>().is_err() {
            return false;
        }
    }
    true
}

pub fn cidrv6() -> Schema<String> {
    string_format("cidrv6", |s| {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return false;
        }
        let (ip, prefix) = (parts[0], parts[1]);
        let valid_ip = ip.contains(':') && ip.chars().all(|c| is_hex_char(c) || c == ':');
        let valid_prefix = prefix.parse::<u8>().is_ok_and(|p| p <= 128);
        valid_ip && valid_prefix
    })
}

// ----------------------------------------------------------------------------
// Coding & Tokens
// ----------------------------------------------------------------------------

pub fn base64() -> Schema<String> {
    string_format("base64", |s| {
        let len = s.len();
        if len % 4 != 0 {
            return false;
        }
        s.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    })
}

pub fn base64url() -> Schema<String> {
    string_format("base64url", |s| {
        s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '=')
    })
}

pub fn hex() -> Schema<String> {
    string_format("hex", |s| {
        !s.is_empty() && s.chars().all(is_hex_char)
    })
}

pub fn jwt() -> Schema<String> {
    string_format("jwt", |s| {
        let parts: Vec<&str> = s.split('.').collect();
        parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
    })
}

pub fn emoji() -> Schema<String> {
    string_format("emoji", |s| {
        !s.is_empty() && s.chars().any(|c| c as u32 > 0x1F000)
    })
}

pub fn e164() -> Schema<String> {
    string_format("e164", |s| {
        s.starts_with('+') && s.len() >= 8 && s.len() <= 16 && s[1..].chars().all(|c| c.is_ascii_digit())
    })
}

pub fn credit_card() -> Schema<String> {
    string_format("credit_card", |s| {
        let digits: Vec<u32> = s.chars().filter(|c| c.is_ascii_digit()).filter_map(|c| c.to_digit(10)).collect();
        if digits.len() < 13 || digits.len() > 19 {
            return false;
        }
        // Luhn algorithm
        let mut sum = 0;
        let mut alt = false;
        for &d in digits.iter().rev() {
            let mut val = d;
            if alt {
                val *= 2;
                if val > 9 {
                    val -= 9;
                }
            }
            sum += val;
            alt = !alt;
        }
        sum % 10 == 0
    })
}

pub fn iban() -> Schema<String> {
    string_format("iban", |s| {
        let clean: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        clean.len() >= 15 && clean.len() <= 34 && clean[..2].chars().all(|c| c.is_ascii_uppercase()) && clean[2..4].chars().all(|c| c.is_ascii_digit())
    })
}

pub fn currency_code() -> Schema<String> {
    string_format("currency_code", |s| {
        s.len() == 3 && s.chars().all(|c| c.is_ascii_uppercase())
    })
}

pub fn hash(algo: &'static str) -> Schema<String> {
    let expected_len = match algo.to_lowercase().as_str() {
        "md5" => 32,
        "sha1" => 40,
        "sha256" => 64,
        "sha512" => 128,
        _ => 0,
    };
    string_format("hash", move |s| {
        (expected_len == 0 || s.len() == expected_len) && s.chars().all(is_hex_char)
    })
}

// ----------------------------------------------------------------------------
// ISO Formats (z::iso::*)
// ----------------------------------------------------------------------------

pub mod iso {
    use super::*;

    pub fn date() -> Schema<String> {
        string_format("iso_date", |s| {
            if s.len() != 10 {
                return false;
            }
            let b = s.as_bytes();
            b[4] == b'-' && b[7] == b'-' &&
            b[..4].iter().all(|&c| (c as char).is_ascii_digit()) &&
            b[5..7].iter().all(|&c| (c as char).is_ascii_digit()) &&
            b[8..10].iter().all(|&c| (c as char).is_ascii_digit())
        })
    }

    pub fn time() -> Schema<String> {
        string_format("iso_time", |s| {
            if s.len() < 8 {
                return false;
            }
            let b = s.as_bytes();
            b[2] == b':' && b[5] == b':'
        })
    }

    pub fn datetime() -> Schema<String> {
        string_format("iso_datetime", |s| {
            if s.len() < 19 {
                return false;
            }
            s.contains('T') || s.contains('t')
        })
    }

    pub fn duration() -> Schema<String> {
        string_format("iso_duration", |s| {
            s.starts_with('P') || s.starts_with('p')
        })
    }
}
