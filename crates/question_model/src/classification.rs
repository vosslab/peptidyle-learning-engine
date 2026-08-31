//! Question Classifications, Tags, and licensing (WP-C1).
//!
//! Shared content carries no deployment partition: one published Question carries
//! one set of tags for every approved Instructor. That is what lets a single
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionClassification {
    /// The external or institutional system that owns the code.
    pub system: String,
    /// The classification code within that system.
    pub code: String,
    /// Human-readable classification name for display.
    pub name: String,
}

/// The terms under which content may be reused.
///
/// An enum rather than a free string so an export can decide, in code, whether
/// a redistribution is permitted. `Other` carries an SPDX identifier for terms
/// this list does not name, which keeps unusual licenses representable without
/// pretending they are one of the common ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum License {
    /// All rights reserved by the author; no redistribution.
    AllRightsReserved,
    /// Creative Commons Attribution.
    CcBy,
    /// Creative Commons Attribution-ShareAlike.
    CcBySa,
    /// Creative Commons Attribution-NonCommercial.
    CcByNc,
    /// Public domain dedication.
    Cc0,
    /// Anything else, named by its SPDX identifier.
    Other {
        /// SPDX license identifier, for example `MIT` or `GPL-3.0-or-later`.
        spdx: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_carries_its_label() {
        assert_eq!(Tag::new("stoichiometry").as_str(), "stoichiometry");
    }

    #[test]
    fn licenses_serialize_with_a_discriminant() {
        let json = serde_json::to_string(&License::CcBySa).expect("serialization should succeed");
        assert_eq!(json, r#"{"kind":"ccBySa"}"#);
    }

    #[test]
    fn an_unusual_license_keeps_its_spdx_identifier() {
        let license = License::Other {
            spdx: "GPL-3.0-or-later".to_string(),
        };
        let json = serde_json::to_string(&license).expect("serialization should succeed");
        assert!(json.contains("GPL-3.0-or-later"));
    }
}
