//! Typed bucket and key construction (WP-C4, MOD-OBJ).

use question_model::generation::Seed;
use question_model::{
    AssetId, CourseBannerCandidateId, CourseBannerId, CourseId, ObjectId, ProblemId, TenantId,
    VersionId, WorkspaceId, WorkspaceImportId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ObjectCategory, Sha256Digest};

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
    /// Canonical private flat-question source for one workspace.
    ///
    /// This is an authored, private source payload distinct from staged import
    /// packages.
    WorkspaceQuestionSource {
        /// Tenant which owns the private workspace.
        tenant: TenantId,
        /// Private authoring workspace.
        workspace: WorkspaceId,
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
    /// The immutable original archive retained with a published imported version.
    ///
    /// This is provenance, not learner-facing content: the tenant binding and
    /// archive checksum are part of the semantic key, and the object is never
    /// eligible for a signed delivery URL.
    PublishedImportArchive {
        /// Tenant which owns the imported provenance.
        tenant: TenantId,
        /// Published problem identity.
        problem: ProblemId,
        /// Immutable version identity.
        version: VersionId,
        /// Import identity which produced this published version.
        import: WorkspaceImportId,
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
    /// Normalized banner bytes awaiting one authorized appearance save.
    ///
    /// Candidate bytes are short-lived, non-signable, and scoped to one
    /// tenant/course before persistence adds actor and expiry ownership.
    CourseBannerCandidate {
        /// Tenant which owns the course.
        tenant: TenantId,
        /// Course whose authorized appearance flow created the candidate.
        course: CourseId,
        /// Opaque candidate identity returned to the authorized browser.
        candidate: CourseBannerCandidateId,
    },
    /// Immutable current-or-retained course banner bytes.
    ///
    /// Typed-object signing is permitted, but the asset-delivery layer must
    /// still verify that this banner is the course's exact current pointer.
    CourseBanner {
        /// Tenant which owns the course.
        tenant: TenantId,
        /// Course whose appearance may reference the banner.
        course: CourseId,
        /// Stable browser-safe banner delivery identity.
        banner: CourseBannerId,
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
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceAsset { .. }
            | Self::ProblemSource { .. }
            | Self::PublishedImportArchive { .. }
            | Self::ProblemAsset { .. }
            | Self::ProblemRender { .. }
            | Self::CourseBanner { .. } => Bucket::Content,
            Self::CourseBannerCandidate { .. } => Bucket::TempProcessing,
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
            Self::WorkspaceQuestionSource {
                tenant,
                workspace,
                object,
            } => format!("workspaces/{tenant}/{workspace}/questions/source/{object}"),
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
            Self::PublishedImportArchive {
                tenant,
                problem,
                version,
                import,
                object,
            } => format!(
                "tenants/{tenant}/problems/{problem}/versions/{version}/imports/{import}/archive/{object}"
            ),
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
            Self::CourseBannerCandidate {
                tenant,
                course,
                candidate,
            } => format!(
                "tenants/{tenant}/courses/{course}/banners/candidates/{candidate}/{}",
                self.object_id()
            ),
            Self::CourseBanner {
                tenant,
                course,
                banner,
            } => format!(
                "tenants/{tenant}/courses/{course}/banners/{banner}/{}",
                self.object_id()
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
            | Self::WorkspaceQuestionSource { object, .. }
            | Self::WorkspaceAsset { object, .. }
            | Self::ProblemSource { object, .. }
            | Self::PublishedImportArchive { object, .. }
            | Self::ProblemAsset { object, .. }
            | Self::ProblemRender { object, .. }
            | Self::StudentRecord { object, .. }
            | Self::Temporary { object } => *object,
            Self::CourseBannerCandidate {
                tenant,
                course,
                candidate,
            } => course_banner_candidate_object_id(*tenant, *course, *candidate),
            Self::CourseBanner {
                tenant,
                course,
                banner,
            } => course_banner_object_id(*tenant, *course, *banner),
        }
    }

    /// Semantic category implied by the key shape.
    pub fn category(&self) -> ObjectCategory {
        match self {
            Self::WorkspaceSource { .. } => ObjectCategory::Source,
            Self::WorkspaceQuestionSource { .. } => ObjectCategory::Source,
            Self::WorkspaceAsset { .. } => ObjectCategory::Asset,
            Self::ProblemSource { .. } => ObjectCategory::Source,
            Self::PublishedImportArchive { .. } => ObjectCategory::Source,
            Self::ProblemAsset { .. } => ObjectCategory::Asset,
            Self::ProblemRender { .. } => ObjectCategory::Render,
            Self::CourseBannerCandidate { .. } => ObjectCategory::Temporary,
            Self::CourseBanner { .. } => ObjectCategory::CourseContent,
            Self::StudentRecord { .. } => ObjectCategory::Export,
            Self::Temporary { .. } => ObjectCategory::Temporary,
        }
    }

    /// Published version associated with content, when one exists.
    pub fn version_id(&self) -> Option<VersionId> {
        match self {
            Self::ProblemSource { version, .. }
            | Self::PublishedImportArchive { version, .. }
            | Self::ProblemAsset { version, .. }
            | Self::ProblemRender { version, .. } => Some(*version),
            Self::WorkspaceSource { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceAsset { .. }
            | Self::CourseBannerCandidate { .. }
            | Self::CourseBanner { .. }
            | Self::StudentRecord { .. }
            | Self::Temporary { .. } => None,
        }
    }

    /// Whether this semantic object may receive a direct delivery URL.
    ///
    /// Workspace imports and published source artifacts remain private even
    /// though their immutable bytes live in the content bucket. Source may
    /// contain answer keys or executable grading logic, so only trusted
    /// server-side adapters may read it. Generic catalog or CDN URL issuance
    /// must reject every source key.
    pub fn may_issue_signed_url(&self) -> bool {
        matches!(
            self,
            Self::ProblemAsset { .. }
                | Self::ProblemRender { .. }
                | Self::CourseBanner { .. }
                | Self::StudentRecord { .. }
        )
    }
}

/// Derives the immutable physical identity for one banner candidate.
pub fn course_banner_candidate_object_id(
    tenant: TenantId,
    course: CourseId,
    candidate: CourseBannerCandidateId,
) -> ObjectId {
    domain_separated_object_id(
        b"ple:course-banner-candidate:v1\0",
        [tenant.as_uuid(), course.as_uuid(), candidate.as_uuid()],
    )
}

/// Derives the immutable physical identity for one promoted course banner.
pub fn course_banner_object_id(
    tenant: TenantId,
    course: CourseId,
    banner: CourseBannerId,
) -> ObjectId {
    domain_separated_object_id(
        b"ple:course-banner:v1\0",
        [tenant.as_uuid(), course.as_uuid(), banner.as_uuid()],
    )
}

fn domain_separated_object_id(domain: &[u8], components: [uuid::Uuid; 3]) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    for component in components {
        hasher.update(component.as_bytes());
    }
    let digest = hasher.finalize();
    let mut object_uuid = [0_u8; 16];
    object_uuid.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(uuid::Uuid::from_bytes(object_uuid))
}

/// Derives the stable object identity for a private workspace QTI archive.
///
/// UUID wrappers are encoded as their raw 16-byte values. The archive digest
/// is deliberately excluded: an exact replay and divergent bytes for the same
/// import identity must address the same immutable [`ObjectKey::WorkspaceSource`]
/// key so the owning upload path can distinguish replay from conflict.
/// Only the first 16 bytes of the domain-separated SHA-256 digest become the
/// deterministic object UUID.
pub fn workspace_qti_archive_object_id(
    tenant: TenantId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:workspace-qti-archive:v1\0");
    hasher.update(tenant.as_uuid().as_bytes());
    hasher.update(workspace.as_uuid().as_bytes());
    hasher.update(import.as_uuid().as_bytes());

    let digest = hasher.finalize();
    let mut object_uuid = [0_u8; 16];
    object_uuid.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(uuid::Uuid::from_bytes(object_uuid))
}

/// Derives the stable object identity for a published import archive.
///
/// The archive checksum is already a SHA-256 digest and is appended as its
/// raw 32-byte value. UUID wrappers are likewise encoded as their raw 16-byte
/// values; no textual formatting, UUID normalization, or checksum rehashing
/// is involved in the input encoding. Only the first 16 bytes of the final
/// SHA-256 digest become the deterministic object UUID.
pub fn published_import_archive_object_id(
    tenant: TenantId,
    problem: ProblemId,
    version: VersionId,
    import: WorkspaceImportId,
    archive_sha256: Sha256Digest,
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:published-import-archive:v1\0");
    hasher.update(tenant.as_uuid().as_bytes());
    hasher.update(problem.as_uuid().as_bytes());
    hasher.update(version.as_uuid().as_bytes());
    hasher.update(import.as_uuid().as_bytes());
    hasher.update(archive_sha256.as_bytes());

    let digest = hasher.finalize();
    let mut object_uuid = [0_u8; 16];
    object_uuid.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(uuid::Uuid::from_bytes(object_uuid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn source_objects_are_never_direct_delivery_targets() {
        let source = ObjectKey::ProblemSource {
            problem: ProblemId::from_uuid(Uuid::from_u128(1)),
            version: VersionId::from_uuid(Uuid::from_u128(2)),
            object: ObjectId::from_uuid(Uuid::from_u128(3)),
        };
        let asset = ObjectKey::ProblemAsset {
            problem: ProblemId::from_uuid(Uuid::from_u128(1)),
            version: VersionId::from_uuid(Uuid::from_u128(2)),
            asset: AssetId::from_uuid(Uuid::from_u128(4)),
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        };

        assert!(!source.may_issue_signed_url());
        assert!(asset.may_issue_signed_url());
    }

    #[test]
    fn course_banner_keys_bind_scope_classification_and_signing() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let course = CourseId::from_uuid(Uuid::from_u128(2));
        let candidate_id = CourseBannerCandidateId::from_uuid(Uuid::from_u128(3));
        let banner_id = CourseBannerId::from_uuid(Uuid::from_u128(4));
        let candidate = ObjectKey::CourseBannerCandidate {
            tenant,
            course,
            candidate: candidate_id,
        };
        let banner = ObjectKey::CourseBanner {
            tenant,
            course,
            banner: banner_id,
        };

        assert_eq!(candidate.bucket(), Bucket::TempProcessing);
        assert_eq!(candidate.category(), ObjectCategory::Temporary);
        assert_eq!(candidate.version_id(), None);
        assert!(!candidate.may_issue_signed_url());
        assert_eq!(banner.bucket(), Bucket::Content);
        assert_eq!(banner.category(), ObjectCategory::CourseContent);
        assert_eq!(banner.version_id(), None);
        assert!(banner.may_issue_signed_url());
        assert!(candidate.path().contains(&tenant.to_string()));
        assert!(candidate.path().contains(&course.to_string()));
        assert!(candidate.path().contains(&candidate_id.to_string()));
        assert!(banner.path().contains(&tenant.to_string()));
        assert!(banner.path().contains(&course.to_string()));
        assert!(banner.path().contains(&banner_id.to_string()));
        assert_ne!(candidate.object_id(), banner.object_id());
    }

    #[test]
    fn banner_object_identity_changes_with_tenant_course_and_route_id() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let course = CourseId::from_uuid(Uuid::from_u128(2));
        let banner = CourseBannerId::from_uuid(Uuid::from_u128(3));
        let base = course_banner_object_id(tenant, course, banner);

        assert_ne!(
            base,
            course_banner_object_id(TenantId::from_uuid(Uuid::from_u128(11)), course, banner)
        );
        assert_ne!(
            base,
            course_banner_object_id(tenant, CourseId::from_uuid(Uuid::from_u128(12)), banner)
        );
        assert_ne!(
            base,
            course_banner_object_id(
                tenant,
                course,
                CourseBannerId::from_uuid(Uuid::from_u128(13))
            )
        );
    }

    #[test]
    fn banner_keys_round_trip_without_a_caller_supplied_object_id() {
        let key = ObjectKey::CourseBanner {
            tenant: TenantId::from_uuid(Uuid::from_u128(1)),
            course: CourseId::from_uuid(Uuid::from_u128(2)),
            banner: CourseBannerId::from_uuid(Uuid::from_u128(3)),
        };
        let encoded = serde_json::to_string(&key).expect("banner key should serialize");
        let decoded: ObjectKey =
            serde_json::from_str(&encoded).expect("banner key should deserialize");

        assert_eq!(decoded, key);
        assert!(!encoded.contains("\"object\""));
    }

    #[test]
    fn workspace_qti_archive_object_id_matches_golden() {
        let actual = workspace_qti_archive_object_id(
            TenantId::from_uuid(Uuid::from_u128(1)),
            WorkspaceId::from_uuid(Uuid::from_u128(2)),
            WorkspaceImportId::from_uuid(Uuid::from_u128(3)),
        );

        assert_eq!(
            actual,
            ObjectId::from_uuid(
                Uuid::parse_str("6c313ff1-1a1f-d2ff-882e-ef18819b9f95")
                    .expect("golden object UUID should be valid")
            )
        );
    }

    #[test]
    fn workspace_qti_archive_identity_changes_with_tenant() {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(3));

        assert_ne!(
            workspace_qti_archive_object_id(
                TenantId::from_uuid(Uuid::from_u128(1)),
                workspace,
                import
            ),
            workspace_qti_archive_object_id(
                TenantId::from_uuid(Uuid::from_u128(11)),
                workspace,
                import
            )
        );
    }

    #[test]
    fn workspace_qti_archive_identity_changes_with_workspace() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(3));

        assert_ne!(
            workspace_qti_archive_object_id(
                tenant,
                WorkspaceId::from_uuid(Uuid::from_u128(2)),
                import
            ),
            workspace_qti_archive_object_id(
                tenant,
                WorkspaceId::from_uuid(Uuid::from_u128(12)),
                import
            )
        );
    }

    #[test]
    fn workspace_qti_archive_identity_changes_with_import() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));

        assert_ne!(
            workspace_qti_archive_object_id(
                tenant,
                workspace,
                WorkspaceImportId::from_uuid(Uuid::from_u128(3))
            ),
            workspace_qti_archive_object_id(
                tenant,
                workspace,
                WorkspaceImportId::from_uuid(Uuid::from_u128(13))
            )
        );
    }

    #[test]
    fn workspace_qti_archive_uses_private_workspace_source_key() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(3));
        let object = workspace_qti_archive_object_id(tenant, workspace, import);
        let key = ObjectKey::WorkspaceSource {
            tenant,
            workspace,
            import,
            object,
        };

        assert_eq!(
            key.path(),
            format!("workspaces/{tenant}/{workspace}/imports/{import}/source/{object}")
        );
        assert_eq!(key.object_id(), object);
        assert_eq!(key.bucket(), Bucket::Content);
        assert_eq!(key.category(), ObjectCategory::Source);
        assert_eq!(key.version_id(), None);
        assert!(!key.may_issue_signed_url());
    }

    #[test]
    fn published_import_archive_object_id_matches_golden() {
        let actual = published_import_archive_object_id(
            TenantId::from_uuid(Uuid::from_u128(1)),
            ProblemId::from_uuid(Uuid::from_u128(2)),
            VersionId::from_uuid(Uuid::from_u128(3)),
            WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            Sha256Digest::compute(b"archive fixture"),
        );

        assert_eq!(
            actual,
            ObjectId::from_uuid(
                Uuid::parse_str("e6ca5943-2fb2-c3b2-bf14-5c9cc3813aa1")
                    .expect("golden object UUID should be valid")
            )
        );
    }

    #[test]
    fn published_import_archive_key_has_distinct_path_and_private_classification() {
        let key = ObjectKey::PublishedImportArchive {
            tenant: TenantId::from_uuid(Uuid::from_u128(1)),
            problem: ProblemId::from_uuid(Uuid::from_u128(2)),
            version: VersionId::from_uuid(Uuid::from_u128(3)),
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        };

        assert_eq!(
            key.path(),
            "tenants/00000000-0000-0000-0000-000000000001/problems/00000000-0000-0000-0000-000000000002/versions/00000000-0000-0000-0000-000000000003/imports/00000000-0000-0000-0000-000000000004/archive/00000000-0000-0000-0000-000000000005"
        );
        assert_eq!(key.bucket(), Bucket::Content);
        assert_eq!(key.category(), ObjectCategory::Source);
        assert_eq!(
            key.version_id(),
            Some(VersionId::from_uuid(Uuid::from_u128(3)))
        );
        assert!(!key.may_issue_signed_url());
    }

    #[test]
    fn every_archive_identity_input_changes_the_object_id() {
        let tenant = TenantId::from_uuid(Uuid::from_u128(1));
        let problem = ProblemId::from_uuid(Uuid::from_u128(2));
        let version = VersionId::from_uuid(Uuid::from_u128(3));
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(4));
        let archive = Sha256Digest::compute(b"archive fixture");
        let base = published_import_archive_object_id(tenant, problem, version, import, archive);

        assert_ne!(
            base,
            published_import_archive_object_id(
                TenantId::from_uuid(Uuid::from_u128(11)),
                problem,
                version,
                import,
                archive
            )
        );
        assert_ne!(
            base,
            published_import_archive_object_id(
                tenant,
                ProblemId::from_uuid(Uuid::from_u128(12)),
                version,
                import,
                archive
            )
        );
        assert_ne!(
            base,
            published_import_archive_object_id(
                tenant,
                problem,
                VersionId::from_uuid(Uuid::from_u128(13)),
                import,
                archive
            )
        );
        assert_ne!(
            base,
            published_import_archive_object_id(
                tenant,
                problem,
                version,
                WorkspaceImportId::from_uuid(Uuid::from_u128(14)),
                archive
            )
        );
        assert_ne!(
            base,
            published_import_archive_object_id(
                tenant,
                problem,
                version,
                import,
                Sha256Digest::compute(b"different archive")
            )
        );
    }

    #[test]
    fn published_import_archive_key_round_trips_through_serde() {
        let key = ObjectKey::PublishedImportArchive {
            tenant: TenantId::from_uuid(Uuid::from_u128(1)),
            problem: ProblemId::from_uuid(Uuid::from_u128(2)),
            version: VersionId::from_uuid(Uuid::from_u128(3)),
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        };

        let encoded = serde_json::to_string(&key).expect("object key should serialize");
        let decoded: ObjectKey =
            serde_json::from_str(&encoded).expect("object key should deserialize");
        assert_eq!(decoded, key);
        assert!(encoded.contains("publishedImportArchive"));
    }
}
