// @spec REQ-CORE-007
use crate::value::Value;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Metadata associated with any Zod schema (replaces `.describe()` with rich attributes).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub examples: Vec<Value>,
    pub deprecated: bool,
    pub custom: BTreeMap<String, Value>,
}

/// Fluent builder for `.meta(|m| ...)` closure pattern.
#[derive(Debug, Default)]
pub struct MetaBuilder {
    meta: Meta,
}

impl MetaBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an explicit identifier for this schema (used in `$defs` and GraphQL type names).
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.meta.id = Some(id.into());
        self
    }

    /// Set a human-readable title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.meta.title = Some(title.into());
        self
    }

    /// Set a detailed description for documentation, OpenAPI, and GraphQL.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.meta.description = Some(description.into());
        self
    }

    /// Add an example value for documentation and OpenAPI.
    pub fn example(mut self, example: impl Into<Value>) -> Self {
        self.meta.examples.push(example.into());
        self
    }

    /// Mark this schema as deprecated.
    pub fn deprecated(mut self, deprecated: bool) -> Self {
        self.meta.deprecated = deprecated;
        self
    }

    /// Add arbitrary custom metadata (e.g., `x-` vendor extensions in OpenAPI).
    pub fn custom(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.meta.custom.insert(key.into(), value.into());
        self
    }

    /// Finalize building the metadata.
    pub fn build(self) -> Meta {
        self.meta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_builder() {
        let meta = MetaBuilder::new()
            .id("UserEmail")
            .title("User Email")
            .description("Primary email address")
            .example("user@example.com")
            .deprecated(false)
            .custom("x-pii", true)
            .build();

        assert_eq!(meta.id.as_deref(), Some("UserEmail"));
        assert_eq!(meta.title.as_deref(), Some("User Email"));
        assert_eq!(meta.description.as_deref(), Some("Primary email address"));
        assert_eq!(meta.examples.len(), 1);
        assert!(!meta.deprecated);
        assert_eq!(meta.custom.get("x-pii"), Some(&Value::Bool(true)));
    }
}
