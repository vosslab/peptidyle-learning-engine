//! Question Classifications, Tags, and licensing (WP-C1).
//!
//! Shared content carries no deployment partition: one published Question carries
//! one set of tags for every active Instructor. That is what lets a single
//! published Question serve thousands of instructors without copying.
//!
//! Licensing travels with the content because imported material (Open Problem
//! Library questions, QTI packages) arrives with terms attached that a later
//! export has to honor.

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
        Tag(label.into())
    }

    /// The label text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One exact mapping to a real external or institutional classification system.
///
/// Distinct from [`Tag`] because a Question Classification preserves its
/// external system and code through import and export. Bloom's revised
/// framework uses its dedicated two-axis Question Bloom Classification contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionClassification {
    /// The external or institutional system that owns the code.
    pub system: String,
    /// The classification code within that system.
    pub code: String,
    /// Human-readable classification name for display.
    pub name: String,
}

/// The exact versioned SPDX grant under which one Question Revision may be reused.
///
/// Question Library publication requires an adaptation-permitting Creative
/// Commons grant. The closed value set keeps draft validation, search, export,
/// and the database publication rule in agreement.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum QuestionLicense {
    /// Creative Commons public-domain dedication 1.0.
    #[serde(rename = "CC0-1.0")]
    Cc0_1_0,
    /// Creative Commons Attribution 4.0.
    #[serde(rename = "CC-BY-4.0")]
    CcBy4_0,
    /// Creative Commons Attribution-ShareAlike 4.0.
    #[serde(rename = "CC-BY-SA-4.0")]
    CcBySa4_0,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_carries_its_label() {
        assert_eq!(Tag::new("stoichiometry").as_str(), "stoichiometry");
    }

    #[test]
    fn question_license_serializes_as_an_exact_spdx_expression() {
        let json = serde_json::to_string(&QuestionLicense::CcBySa4_0)
            .expect("serialization should succeed");
        assert_eq!(json, r#""CC-BY-SA-4.0""#);
    }

    #[test]
    fn question_license_refuses_an_incompatible_or_unversioned_expression() {
        let decoded = serde_json::from_str::<QuestionLicense>(r#""CC-BY""#);
        assert!(decoded.is_err());
    }
}
