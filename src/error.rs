// @spec REQ-CORE-002
// @spec REQ-CORE-003
// @spec REQ-CORE-004
use crate::value::Value;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use compact_str::CompactString;
use core::fmt;

/// Sizing origin for size and bound constraints (TooSmall, TooBig).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeOrigin {
    String,
    Number,
    Array,
    Object,
    Set,
    Map,
    Date,
}

impl SizeOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            SizeOrigin::String => "string",
            SizeOrigin::Number => "number",
            SizeOrigin::Array => "array",
            SizeOrigin::Object => "object",
            SizeOrigin::Set => "set",
            SizeOrigin::Map => "map",
            SizeOrigin::Date => "date",
        }
    }
}

/// A segment in the property or index path to the invalid data.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathSegment {
    Key(CompactString),
    Index(usize),
}

impl fmt::Display for PathSegment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathSegment::Key(k) => write!(f, "{}", k),
            PathSegment::Index(idx) => write!(f, "[{}]", idx),
        }
    }
}

/// Helper to render a full path string (e.g. `users[0].email`).
pub fn format_path(path: &[PathSegment]) -> String {
    let mut out = String::new();
    for (i, seg) in path.iter().enumerate() {
        match seg {
            PathSegment::Key(k) => {
                if i > 0 {
                    out.push('.');
                }
                out.push_str(k);
            }
            PathSegment::Index(idx) => {
                out.push_str(&format!("[{}]", idx));
            }
        }
    }
    if out.is_empty() {
        "<root>".into()
    } else {
        out
    }
}

/// The 11 Zod v4 issue codes as specified in `colinhacks/zod/packages/zod/src/v4/core/errors.ts`.
#[derive(Debug, Clone, PartialEq)]
pub enum ZodIssueCode {
    InvalidType {
        expected: String,
        received: String,
    },
    TooBig {
        origin: SizeOrigin,
        maximum: f64,
        inclusive: bool,
        exact: bool,
    },
    TooSmall {
        origin: SizeOrigin,
        minimum: f64,
        inclusive: bool,
        exact: bool,
    },
    InvalidFormat {
        format: String,
        pattern: Option<String>,
        prefix: Option<String>,
        suffix: Option<String>,
        includes: Option<String>,
    },
    NotMultipleOf {
        divisor: f64,
    },
    UnrecognizedKeys {
        keys: Vec<String>,
    },
    InvalidUnion {
        errors: Vec<ZodError>,
        discriminator: Option<String>,
        options: Vec<String>,
        inclusive: bool,
        matches: usize,
    },
    InvalidKey {
        origin: String,
        issues: Vec<ZodIssue>,
    },
    InvalidElement {
        origin: String,
        key: String,
        issues: Vec<ZodIssue>,
    },
    InvalidValue {
        values: Vec<String>,
    },
    Custom {
        params: BTreeMap<String, Value>,
    },
}

impl ZodIssueCode {
    pub fn format(format_name: impl Into<String>) -> Self {
        Self::InvalidFormat {
            format: format_name.into(),
            pattern: None,
            prefix: None,
            suffix: None,
            includes: None,
        }
    }
}

/// A single validation issue collected during schema execution.
#[derive(Debug, Clone, PartialEq)]
pub struct ZodIssue {
    pub code: ZodIssueCode,
    pub path: Vec<PathSegment>,
    pub message: String,
    pub fatal: bool,
}

impl ZodIssue {
    pub fn new(code: ZodIssueCode, path: Vec<PathSegment>, message: impl Into<String>) -> Self {
        Self {
            code,
            path,
            message: message.into(),
            fatal: false,
        }
    }

    pub fn fatal(mut self) -> Self {
        self.fatal = true;
        self
    }
}

/// The error container holding all validation issues detected during parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ZodError {
    pub issues: Vec<ZodIssue>,
    /// Non-validation failure (async step hit by a sync parse, or a one-way step hit by `.encode()`).
    /// These are not `ZodIssue`s, mirroring `$ZodAsyncError` / `$ZodEncodeError` in Zod v4.
    pub failure: Option<Failure>,
}

/// Failures that are not data-validation issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// An async step (`refine_async`, `transform_async`, `codec_async`) was reached by a synchronous parse.
    Async,
    /// A unidirectional step (e.g. `.transform()`) was reached by `.encode()`.
    Encode(String),
}

impl ZodError {
    pub fn new(issues: Vec<ZodIssue>) -> Self {
        Self {
            issues,
            failure: None,
        }
    }

    pub fn from_issue(issue: ZodIssue) -> Self {
        Self {
            issues: alloc::vec![issue],
            failure: None,
        }
    }

    pub fn from_failure(failure: Failure) -> Self {
        Self {
            issues: Vec::new(),
            failure: Some(failure),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// Formatter 1: .flatten() -> { form_errors, field_errors }
    pub fn flatten(&self) -> FlattenedError {
        let mut form_errors = Vec::new();
        let mut field_errors: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for issue in &self.issues {
            if issue.path.is_empty() {
                form_errors.push(issue.message.clone());
            } else {
                let first_key = match &issue.path[0] {
                    PathSegment::Key(k) => k.to_string(),
                    PathSegment::Index(i) => format!("{}", i),
                };
                field_errors
                    .entry(first_key)
                    .or_default()
                    .push(issue.message.clone());
            }
        }

        FlattenedError {
            form_errors,
            field_errors,
        }
    }

    /// Formatter 2: .format() -> tree structure with `_errors` at each level.
    pub fn format(&self) -> FormattedError {
        let mut root = FormattedError::default();
        for issue in &self.issues {
            root.insert_issue(&issue.path, &issue.message);
        }
        root
    }

    /// Formatter 3: .treeify() -> { errors, properties, items }
    pub fn treeify(&self) -> TreeifiedError {
        let mut root = TreeifiedError::default();
        for issue in &self.issues {
            root.insert_issue(&issue.path, &issue.message);
        }
        root
    }

    /// Formatter 4: .prettify() -> human readable formatted string
    /// Example:
    /// ✖ Invalid email
    ///   → at user.email
    pub fn prettify(&self) -> String {
        let mut out = String::new();
        for (idx, issue) in self.issues.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            out.push_str(&format!("✖ {}\n", issue.message));
            out.push_str(&format!("  → at {}", format_path(&issue.path)));
        }
        out
    }
}

impl fmt::Display for ZodError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.failure {
            Some(Failure::Async) => {
                return write!(f, "Encountered an async step during a synchronous parse; use `parse_async` / `decode_async` / `encode_async`")
            }
            Some(Failure::Encode(msg)) => return write!(f, "Encode error: {}", msg),
            None => {}
        }
        write!(f, "{}", self.prettify())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ZodError {}

/// Result of `.flatten()`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlattenedError {
    pub form_errors: Vec<String>,
    pub field_errors: BTreeMap<String, Vec<String>>,
}

/// Result of `.format()`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormattedError {
    pub errors: Vec<String>,
    pub fields: BTreeMap<String, FormattedError>,
}

impl FormattedError {
    fn insert_issue(&mut self, path: &[PathSegment], message: &str) {
        if path.is_empty() {
            self.errors.push(message.to_string());
        } else {
            let key = match &path[0] {
                PathSegment::Key(k) => k.to_string(),
                PathSegment::Index(i) => format!("{}", i),
            };
            self.fields
                .entry(key)
                .or_default()
                .insert_issue(&path[1..], message);
        }
    }
}

/// Result of `.treeify()`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TreeifiedError {
    pub errors: Vec<String>,
    pub properties: BTreeMap<String, TreeifiedError>,
    pub items: BTreeMap<usize, TreeifiedError>,
}

impl TreeifiedError {
    fn insert_issue(&mut self, path: &[PathSegment], message: &str) {
        if path.is_empty() {
            self.errors.push(message.to_string());
        } else {
            match &path[0] {
                PathSegment::Key(k) => {
                    self.properties
                        .entry(k.to_string())
                        .or_default()
                        .insert_issue(&path[1..], message);
                }
                PathSegment::Index(i) => {
                    self.items
                        .entry(*i)
                        .or_default()
                        .insert_issue(&path[1..], message);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_formatters() {
        let issues = vec![
            ZodIssue::new(
                ZodIssueCode::InvalidType {
                    expected: "string".into(),
                    received: "number".into(),
                },
                vec![
                    PathSegment::Key("user".into()),
                    PathSegment::Key("name".into()),
                ],
                "Expected string, received number",
            ),
            ZodIssue::new(
                ZodIssueCode::TooSmall {
                    origin: SizeOrigin::Number,
                    minimum: 18.0,
                    inclusive: true,
                    exact: false,
                },
                vec![
                    PathSegment::Key("user".into()),
                    PathSegment::Key("age".into()),
                ],
                "Number must be greater than or equal to 18",
            ),
            ZodIssue::new(
                ZodIssueCode::Custom {
                    params: BTreeMap::new(),
                },
                vec![],
                "Form contains invalid data",
            ),
        ];

        let error = ZodError::new(issues);

        // 1. Flatten
        let flat = error.flatten();
        assert_eq!(flat.form_errors, vec!["Form contains invalid data"]);
        assert_eq!(flat.field_errors.get("user").unwrap().len(), 2);

        // 2. Prettify
        let pretty = error.prettify();
        assert!(pretty.contains("✖ Expected string, received number\n  → at user.name"));
        assert!(pretty.contains("✖ Number must be greater than or equal to 18\n  → at user.age"));
        assert!(pretty.contains("✖ Form contains invalid data\n  → at <root>"));

        // 3. Treeify
        let tree = error.treeify();
        assert_eq!(tree.errors, vec!["Form contains invalid data"]);
        let user_prop = tree.properties.get("user").unwrap();
        assert!(user_prop.properties.contains_key("name"));
        assert!(user_prop.properties.contains_key("age"));

        // 4. Format
        let fmt = error.format();
        assert_eq!(fmt.errors, vec!["Form contains invalid data"]);
        let user_field = fmt.fields.get("user").unwrap();
        assert_eq!(
            user_field.fields.get("name").unwrap().errors,
            vec!["Expected string, received number"]
        );
    }
}
