//! Typed bucket and key construction (WP-C4, MOD-OBJ).

use question_model::generation::Seed;
use question_model::{
    AssetId, ObjectId, ProblemId, TenantId, VersionId, WorkspaceId, WorkspaceImportId,
};
use serde::{Deserialize, Serialize};

use crate::ObjectCategory;

/// One of the three object stores with a distinct access and retention policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Bucket {
    /// Shared source, assets, and deterministic rendered content.
    Content,
    /// Student-specific exports, uploads, and annotations.
    StudentRecords,
    /// Never-served extraction and conversion workspaces.
    TempProcessing,
}

impl Bucket {
    /// Returns the deployment bucket name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::StudentRecords => "student-records",
            Self::TempProcessing => "temp-processing",
        }
    }
}

/// Stable identity components from which an immutable object key is built.
///
/// There is no raw-string variant. Callers choose a semantic destination and
/// supply typed IDs; MOD-OBJ alone decides the physical path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ObjectKey {
    /// Original bytes for a private workspace import.
    ///
    /// This intentionally uses the content bucket for immutable durable bytes,
    /// but it is not a catalog asset and must never be exposed through CDN or
    /// catalog asset delivery.
    WorkspaceSource {
        /// Tenant which owns the private workspace.
        tenant: TenantId,
        /// Private authoring workspace.
        workspace: WorkspaceId,
        /// Staged import identity.
        import: WorkspaceImportId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A verified logical asset extracted from a private workspace import.
    ///
    /// Like [`Self::WorkspaceSource`], this is durable content-bucket storage
    /// but not a CDN or catalog-delivery candidate.
    WorkspaceAsset {
        /// Tenant which owns the private workspace.
        tenant: TenantId,
        /// Private authoring workspace.
        workspace: WorkspaceId,
        /// Staged import identity.
        import: WorkspaceImportId,
        /// Logical asset referenced by imported draft content.
        asset: AssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// An original source package for a published version.
    ProblemSource {
        /// Published problem identity.
        problem: ProblemId,
        /// Immutable version identity.
        version: VersionId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A logical asset and its physical object for a published version.
    ProblemAsset {
        /// Published problem identity.
        problem: ProblemId,
        /// Immutable version identity.
        version: VersionId,
        /// Logical asset referenced by content.
        asset: AssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A deterministic rendered question cached by version and seed.
    ProblemRender {
        /// Published problem identity.
        problem: ProblemId,
        /// Immutable version identity.
        version: VersionId,
        /// Seed that fully determines the render.
        seed: Seed,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A tenant-owned student-record artifact.
    StudentRecord {
        /// Tenant whose RLS-protected record owns this object.
        tenant: TenantId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A short-lived processing artifact that is never served.
    Temporary {
        /// Physical object-record identity.
        object: ObjectId,
    },
}

impl ObjectKey {
    /// Bucket selected by this semantic key.
    pub fn bucket(&self) -> Bucket {
        match self {
            Self::WorkspaceSource { .. }
            | Self::WorkspaceAsset { .. }
            | Self::ProblemSource { .. }
            | Self::ProblemAsset { .. }
            | Self::ProblemRender { .. } => Bucket::Content,
            Self::StudentRecord { .. } => Bucket::StudentRecords,
            Self::Temporary { .. } => Bucket::TempProcessing,
        }
    }

    /// Immutable path derived only from typed identity components.
    pub fn path(&self) -> String {
        match self {
            Self::WorkspaceSource {
                tenant,
                workspace,
                import,
                object,
            } => format!("workspaces/{tenant}/{workspace}/imports/{import}/source/{object}"),
            Self::WorkspaceAsset {
                tenant,
                workspace,
                import,
                asset,
                object,
            } => {
                format!("workspaces/{tenant}/{workspace}/imports/{import}/assets/{asset}/{object}")
            }
            Self::ProblemSource {
                problem,
                version,
                object,
            } => format!("problems/{problem}/versions/{version}/source/{object}"),
            Self::ProblemAsset {
                problem,
                version,
                asset,
                object,
            } => format!("problems/{problem}/versions/{version}/assets/{asset}/{object}"),
            Self::ProblemRender {
                problem,
                version,
                seed,
                object,
            } => format!(
                "problems/{problem}/versions/{version}/renders/{}/{object}",
                seed.value()
            ),
            Self::StudentRecord { tenant, object } => {
                format!("records/{tenant}/{object}")
            }
            Self::Temporary { object } => format!("processing/{object}"),
        }
    }

    /// Object-record identity embedded in the key.
    pub fn object_id(&self) -> ObjectId {
        match self {
            Self::WorkspaceSource { object, .. }
            | Self::WorkspaceAsset { object, .. }
            | Self::ProblemSource { object, .. }
            | Self::ProblemAsset { object, .. }
            | Self::ProblemRender { object, .. }
            | Self::StudentRecord { object, .. }
            | Self::Temporary { object } => *object,
        }
    }

    /// Semantic category implied by the key shape.
    pub fn category(&self) -> ObjectCategory {
        match self {
            Self::WorkspaceSource { .. } => ObjectCategory::Source,
            Self::WorkspaceAsset { .. } => ObjectCategory::Asset,
            Self::ProblemSource { .. } => ObjectCategory::Source,
            Self::ProblemAsset { .. } => ObjectCategory::Asset,
            Self::ProblemRender { .. } => ObjectCategory::Render,
            Self::StudentRecord { .. } => ObjectCategory::Export,
            Self::Temporary { .. } => ObjectCategory::Temporary,
        }
    }

    /// Published version associated with content, when one exists.
    pub fn version_id(&self) -> Option<VersionId> {
        match self {
            Self::ProblemSource { version, .. }
            | Self::ProblemAsset { version, .. }
            | Self::ProblemRender { version, .. } => Some(*version),
            Self::WorkspaceSource { .. }
            | Self::WorkspaceAsset { .. }
            | Self::StudentRecord { .. }
            | Self::Temporary { .. } => None,
        }
    }

    /// Whether this semantic object may receive a direct delivery URL.
    ///
    /// Workspace imports remain private staging records even though their
    /// immutable bytes live in the content bucket. Only a separately
    /// authorized workspace-preview service may read them; generic catalog or
    /// CDN URL issuance must reject them.
    pub fn may_issue_signed_url(&self) -> bool {
        matches!(
            self,
            Self::ProblemSource { .. }
                | Self::ProblemAsset { .. }
                | Self::ProblemRender { .. }
                | Self::StudentRecord { .. }
        )
    }
}
