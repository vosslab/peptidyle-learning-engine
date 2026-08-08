//! Browser-safe shared-catalog metadata (MOD-API-CAT).

use serde::{Deserialize, Serialize};

use crate::taxonomy::{License, Tag, TaxonomyTerm};
use crate::{
    ActivityTimestamp, BackendCapabilities, ProblemId, QuestionMetadata, QuestionSource, UserId,
    VersionId,
};

/// Exact immutable problem version used by lineage and assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProblemVersionRef {
    /// Stable published problem.
    pub problem: ProblemId,
    /// Exact immutable version.
    pub version: VersionId,
}

/// Visibility of immutable published content.
///
/// Private content remains a tenant-owned draft and therefore has no variant
/// here and no `ProblemId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublicationScope {
    /// Discoverable only by the publishing institution.
    Institution,
    /// Discoverable across every tenant.
    Public,
}

/// Catalog state after publication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CatalogLifecycle {
    /// Discoverable and eligible for new assignments.
    Published,
    /// Hidden from discovery but still eligible by exact reference.
    Deprecated {
        /// Why instructors should stop newly assigning this version.
        reason: String,
    },
    /// Historical content retained for exact resolution only.
    Archived {
        /// Original deprecation explanation retained for the record.
        reason: String,
    },
}

impl CatalogLifecycle {
    /// Whether catalog browsing should include the version.
    pub fn is_discoverable(&self) -> bool {
        matches!(self, Self::Published)
    }

    /// Whether a new assignment may reference the version.
    pub fn is_assignable(&self) -> bool {
        matches!(self, Self::Published | Self::Deprecated { .. })
    }
}

/// Adapter family without source paths or package identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionBackend {
    /// First-party Rust/WASM question.
    Native,
    /// WeBWorK PG question.
    Webwork,
    /// IMS QTI item.
    Qti,
    /// Ungraded H5P activity.
    H5p,
}

impl From<&QuestionSource> for QuestionBackend {
    fn from(source: &QuestionSource) -> Self {
        match source {
            QuestionSource::Native { .. } => Self::Native,
            QuestionSource::Webwork { .. } => Self::Webwork,
            QuestionSource::Qti { .. } => Self::Qti,
            QuestionSource::H5p { .. } => Self::H5p,
        }
    }
}

/// Hot catalog metadata returned by browse endpoints without loading payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProblemSummary {
    /// Stable published problem.
    pub problem: ProblemId,
    /// Exact immutable version represented by this row.
    pub version: VersionId,
    /// Adapter family, without private source-locator fields.
    pub backend: QuestionBackend,
    /// Capabilities declared by the owning adapter at publication time.
    pub capabilities: BackendCapabilities,
    /// Shared metadata used for title, taxonomy, license, and language facets.
    pub metadata: QuestionMetadata,
    /// Institution-only or public visibility.
    pub scope: PublicationScope,
    /// Published, deprecated, or archived state.
    pub lifecycle: CatalogLifecycle,
    /// Ordered, nonempty author identifiers controlling the linear chain.
    pub authors: Vec<UserId>,
    /// Earlier version in the same single-writer chain, when this is a revision.
    pub previous_version: Option<VersionId>,
    /// Source version when this problem began as a third-party fork.
    pub derived_from: Option<ProblemVersionRef>,
    /// Database-authoritative publication time.
    pub published_at: ActivityTimestamp,
}

impl CatalogProblemSummary {
    /// Free-form tags for filtering without loading the question payload.
    pub fn tags(&self) -> &[Tag] {
        &self.metadata.tags
    }

    /// Controlled terms for taxonomy aggregation and filtering.
    pub fn taxonomy(&self) -> &[TaxonomyTerm] {
        &self.metadata.taxonomy
    }

    /// License facet for reuse decisions.
    pub fn license(&self) -> &License {
        &self.metadata.license
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_summary_never_carries_private_source_locators() {
        assert_eq!(
            QuestionBackend::from(&QuestionSource::Webwork {
                pg_path: "OpenProblemLibrary/private/path.pg".to_string(),
            }),
            QuestionBackend::Webwork
        );
        assert_eq!(
            serde_json::to_string(&QuestionBackend::Webwork).expect("backend serializes"),
            "\"webwork\""
        );
    }

    #[test]
    fn deprecation_hides_discovery_while_archival_blocks_assignment() {
        assert!(CatalogLifecycle::Published.is_discoverable());
        assert!(CatalogLifecycle::Published.is_assignable());
        let deprecated = CatalogLifecycle::Deprecated {
            reason: "Correction available".to_string(),
        };
        assert!(!deprecated.is_discoverable());
        assert!(deprecated.is_assignable());
        assert!(
            !CatalogLifecycle::Archived {
                reason: "Historical".to_string(),
            }
            .is_assignable()
        );
    }
}
