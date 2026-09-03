//! Free-form Question tags.

use serde::{Deserialize, Serialize};

/// A free-form label an instructor can search by.
///
/// A newtype rather than a bare `String` so a tag cannot be passed where a
/// title or an identifier is expected.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Tag(String);

impl Tag {
    /// Wraps a label as a tag.
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    /// The label text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_carries_its_label() {
        assert_eq!(Tag::new("stoichiometry").as_str(), "stoichiometry");
    }
}
