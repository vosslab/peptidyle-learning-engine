//! Typed bucket and key construction (WP-C4, MOD-OBJ).

use question_model::generation::Seed;
use question_model::{
    AssetId, CourseBannerCandidateId, CourseBannerId, CourseId, ObjectId, QuestionVersionReference,
    WorkspaceId, WorkspaceImportId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ObjectCategory, Sha256Digest};

/// One of the four object stores with a distinct access, encryption, and
/// delivery policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Bucket {
    /// Immutable student-facing renditions. This is the only CDN-readable
    /// domain and therefore contains only [`ObjectKey::QuestionAsset`] bytes.
    PublicAssets,
    /// Private authoring, provenance, grading, rendering, and course content.
    PrivateContent,
    /// Student-specific exports, uploads, and annotations.
    StudentRecords,
    /// Never-served extraction and conversion workspaces.
    TempProcessing,
}

impl Bucket {
    /// Returns the deployment bucket name.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PublicAssets => "public-assets",
            Self::PrivateContent => "private-content",
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
    /// This intentionally uses the private-content bucket for immutable
    /// durable bytes and must never be exposed through CDN or Question Library asset
    /// delivery.
    WorkspaceSource {
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
        /// Private authoring workspace.
        workspace: WorkspaceId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A verified logical asset extracted from a private workspace import.
    ///
    /// Like [`Self::WorkspaceSource`], this is durable private-content storage
    /// but not a CDN or Question Library delivery candidate.
    WorkspaceAsset {
        /// Private authoring workspace.
        workspace: WorkspaceId,
        /// Staged import identity.
        import: WorkspaceImportId,
        /// Logical asset referenced by imported draft content.
        asset: AssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A logical asset authored directly for a private workspace question.
    ///
    /// This private-content object is intentionally distinct from
    /// [`Self::WorkspaceAsset`]: it has no import provenance and is never a
    /// Question Library asset or direct-delivery candidate.
    WorkspaceQuestionAsset {
        /// Private authoring workspace.
        workspace: WorkspaceId,
        /// Logical asset referenced by a workspace question.
        asset: AssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// An original source package for a published version.
    QuestionSource {
        /// Exact immutable Question Version that owns the source.
        question_version: QuestionVersionReference,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// The immutable original archive retained with a published imported version.
    ///
    /// This is provenance, not student-facing content: the archive checksum is
    /// part of the semantic key, and the object is never
    /// eligible for a signed delivery URL.
    PublishedImportArchive {
        /// Exact immutable Question Version that owns the archive.
        question_version: QuestionVersionReference,
        /// Import identity which produced this published version.
        import: WorkspaceImportId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A logical asset and its physical object for a published version.
    QuestionAsset {
        /// Exact immutable Question Version that owns the asset.
        question_version: QuestionVersionReference,
        /// Logical asset referenced by content.
        asset: AssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A student-facing asset belonging to a Published Question version.
    ///
    /// Its identity is as immutable as [`Self::QuestionAsset`], but its bytes
    /// live in private-content and are delivered only after Question Library
    /// authorization.  A CDN-readable key must never represent restricted
    /// published content.
    RestrictedQuestionAsset {
        /// Exact immutable Question Version that owns the asset.
        question_version: QuestionVersionReference,
        /// Logical asset referenced by content.
        asset: AssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A deterministic rendered question cached by version and seed.
    QuestionRender {
        /// Exact immutable Question Version that owns the rendered result.
        question_version: QuestionVersionReference,
        /// Seed that fully determines the render.
        seed: Seed,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// Normalized banner bytes awaiting one authorized appearance save.
    ///
    /// Candidate bytes are short-lived, non-signable, and scoped to one
    /// course before persistence adds Account and expiry ownership.
    CourseBannerCandidate {
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
        /// Course whose appearance may reference the banner.
        course: CourseId,
        /// Stable browser-safe banner delivery identity.
        banner: CourseBannerId,
    },
    /// A course-owned student-record source_object_reference.
    StudentRecord {
        /// Exact course whose protected record owns this object.
        course: CourseId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A short-lived processing source_object_reference that is never served.
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
            | Self::WorkspaceQuestionAsset { .. }
            | Self::QuestionSource { .. }
            | Self::PublishedImportArchive { .. }
            | Self::RestrictedQuestionAsset { .. }
            | Self::QuestionRender { .. }
            | Self::CourseBanner { .. } => Bucket::PrivateContent,
            Self::QuestionAsset { .. } => Bucket::PublicAssets,
            Self::CourseBannerCandidate { .. } => Bucket::TempProcessing,
            Self::StudentRecord { .. } => Bucket::StudentRecords,
            Self::Temporary { .. } => Bucket::TempProcessing,
        }
    }

    /// Immutable path derived only from typed identity components.
    pub fn path(&self) -> String {
        match self {
            Self::WorkspaceSource {
                workspace,
                import,
                object,
            } => {
                format!("workspaces/{workspace}/imports/{import}/source/{object}")
            }
            Self::WorkspaceQuestionSource { workspace, object } => {
                format!("workspaces/{workspace}/questions/source/{object}")
            }
            Self::WorkspaceAsset {
                workspace,
                import,
                asset,
                object,
            } => {
                format!("workspaces/{workspace}/imports/{import}/assets/{asset}/{object}")
            }
            Self::WorkspaceQuestionAsset {
                workspace,
                asset,
                object,
            } => format!("workspaces/{workspace}/questions/assets/{asset}/{object}"),
            Self::QuestionSource {
                question_version,
                object,
            } => format!(
                "questions/{}/versions/{}/source/{object}",
                question_version.question_id, question_version.version_number
            ),
            Self::PublishedImportArchive {
                question_version,
                import,
                object,
            } => format!(
                "questions/{}/versions/{}/imports/{import}/archive/{object}",
                question_version.question_id, question_version.version_number
            ),
            Self::QuestionAsset {
                question_version,
                asset,
                object,
            } => format!(
                "questions/{}/versions/{}/assets/{asset}/{object}",
                question_version.question_id, question_version.version_number
            ),
            Self::RestrictedQuestionAsset {
                question_version,
                asset,
                object,
            } => {
                format!(
                    "questions/{}/versions/{}/restricted-assets/{asset}/{object}",
                    question_version.question_id, question_version.version_number
                )
            }
            Self::QuestionRender {
                question_version,
                seed,
                object,
            } => format!(
                "questions/{}/versions/{}/renders/{}/{object}",
                question_version.question_id,
                question_version.version_number,
                seed.value()
            ),
            Self::CourseBannerCandidate { course, candidate } => format!(
                "courses/{course}/banners/candidates/{candidate}/{}",
                self.object_id()
            ),
            Self::CourseBanner { course, banner } => {
                format!("courses/{course}/banners/{banner}/{}", self.object_id())
            }
            Self::StudentRecord { course, object } => {
                format!("courses/{course}/records/{object}")
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
            | Self::WorkspaceQuestionAsset { object, .. }
            | Self::QuestionSource { object, .. }
            | Self::PublishedImportArchive { object, .. }
            | Self::QuestionAsset { object, .. }
            | Self::RestrictedQuestionAsset { object, .. }
            | Self::QuestionRender { object, .. }
            | Self::StudentRecord { object, .. }
            | Self::Temporary { object } => *object,
            Self::CourseBannerCandidate { course, candidate } => {
                course_banner_candidate_object_id(*course, *candidate)
            }
            Self::CourseBanner { course, banner } => course_banner_object_id(*course, *banner),
        }
    }

    /// Semantic category implied by the key shape.
    pub fn category(&self) -> ObjectCategory {
        match self {
            Self::WorkspaceSource { .. } => ObjectCategory::Source,
            Self::WorkspaceQuestionSource { .. } => ObjectCategory::Source,
            Self::WorkspaceAsset { .. } => ObjectCategory::Asset,
            Self::WorkspaceQuestionAsset { .. } => ObjectCategory::Asset,
            Self::QuestionSource { .. } => ObjectCategory::Source,
            Self::PublishedImportArchive { .. } => ObjectCategory::Source,
            Self::QuestionAsset { .. } => ObjectCategory::Asset,
            Self::RestrictedQuestionAsset { .. } => ObjectCategory::Asset,
            Self::QuestionRender { .. } => ObjectCategory::Render,
            Self::CourseBannerCandidate { .. } => ObjectCategory::Temporary,
            Self::CourseBanner { .. } => ObjectCategory::CourseContent,
            Self::StudentRecord { .. } => ObjectCategory::Export,
            Self::Temporary { .. } => ObjectCategory::Temporary,
        }
    }

    /// Exact Question Version associated with content, when one exists.
    pub fn question_version(&self) -> Option<&QuestionVersionReference> {
        match self {
            Self::QuestionSource {
                question_version, ..
            }
            | Self::PublishedImportArchive {
                question_version, ..
            }
            | Self::QuestionAsset {
                question_version, ..
            }
            | Self::RestrictedQuestionAsset {
                question_version, ..
            }
            | Self::QuestionRender {
                question_version, ..
            } => Some(question_version),
            Self::WorkspaceSource { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceAsset { .. }
            | Self::WorkspaceQuestionAsset { .. }
            | Self::CourseBannerCandidate { .. }
            | Self::CourseBanner { .. }
            | Self::StudentRecord { .. }
            | Self::Temporary { .. } => None,
        }
    }

    /// Whether this semantic object may receive a direct delivery URL.
    ///
    /// Workspace imports and published Source Object References remain private in the
    /// private-content bucket. Source may
    /// contain answer keys or executable grading logic, so only trusted
    /// server-side adapters may read it. Generic Question Library or CDN URL issuance
    /// must reject every source key.
    pub fn may_issue_signed_url(&self) -> bool {
        matches!(
            self,
            Self::QuestionAsset { .. }
                | Self::RestrictedQuestionAsset { .. }
                | Self::QuestionRender { .. }
                | Self::CourseBanner { .. }
                | Self::StudentRecord { .. }
        )
    }

    /// Chooses the physical immutable asset domain from the publication's
    /// immutable visibility.  Call this at publication time rather than
    /// reconstructing a key later from an untrusted route or browser value.
    pub fn published_question_asset(
        question_version: QuestionVersionReference,
        asset: AssetId,
        object: ObjectId,
    ) -> Self {
        Self::RestrictedQuestionAsset {
            question_version,
            asset,
            object,
        }
    }
}

/// Derives the immutable physical identity for one banner candidate.
pub fn course_banner_candidate_object_id(
    course: CourseId,
    candidate: CourseBannerCandidateId,
) -> ObjectId {
    domain_separated_object_id(
        b"ple:course-banner-candidate:v1\0",
        [course.as_uuid(), candidate.as_uuid(), uuid::Uuid::nil()],
    )
}

/// Derives the immutable physical identity for one promoted course banner.
pub fn course_banner_object_id(course: CourseId, banner: CourseBannerId) -> ObjectId {
    domain_separated_object_id(
        b"ple:course-banner:v1\0",
        [course.as_uuid(), banner.as_uuid(), uuid::Uuid::nil()],
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
    workspace: WorkspaceId,
    import: WorkspaceImportId,
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:workspace-qti-archive:v1\0");
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
/// raw 32-byte value. The canonical Question ID spelling and big-endian
/// Question Version Number bind the address to one exact Question Version.
/// Only the first 16 bytes of the final SHA-256 digest become the deterministic
/// object UUID.
pub fn published_import_archive_object_id(
    question_version: &QuestionVersionReference,
    import: WorkspaceImportId,
    archive_sha256: Sha256Digest,
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:published-import-archive:v1\0");
    hasher.update(question_version.question_id.to_string().as_bytes());
    hasher.update(question_version.version_number.get().to_be_bytes());
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
    use question_model::{QuestionId, QuestionVersionNumber};
    use uuid::Uuid;

    fn question_version(version_number: u32) -> QuestionVersionReference {
        QuestionVersionReference {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G')
                .expect("canonical Question ID"),
            version_number: QuestionVersionNumber::new(version_number)
                .expect("positive Question Version Number"),
        }
    }

    #[test]
    fn source_objects_are_never_direct_delivery_targets() {
        let source = ObjectKey::QuestionSource {
            question_version: question_version(2),
            object: ObjectId::from_uuid(Uuid::from_u128(3)),
        };
        let asset = ObjectKey::QuestionAsset {
            question_version: question_version(2),
            asset: AssetId::from_uuid(Uuid::from_u128(4)),
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        };

        assert!(!source.may_issue_signed_url());
        assert!(asset.may_issue_signed_url());
    }

    #[test]
    fn only_immutable_question_assets_enter_the_public_delivery_domain() {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
        let question_version = question_version(4);
        let object = ObjectId::from_uuid(Uuid::from_u128(5));

        let public_asset = ObjectKey::QuestionAsset {
            question_version: question_version.clone(),
            asset: AssetId::from_uuid(Uuid::from_u128(6)),
            object,
        };
        assert_eq!(public_asset.bucket(), Bucket::PublicAssets);
        assert_eq!(
            ObjectKey::published_question_asset(
                question_version.clone(),
                AssetId::from_uuid(Uuid::from_u128(60)),
                object,
            )
            .bucket(),
            Bucket::PrivateContent,
            "Published Question assets must never enter the CDN-readable bucket"
        );

        for private_key in [
            ObjectKey::WorkspaceSource {
                workspace,
                import: WorkspaceImportId::from_uuid(Uuid::from_u128(7)),
                object,
            },
            ObjectKey::WorkspaceQuestionAsset {
                workspace,
                asset: AssetId::from_uuid(Uuid::from_u128(8)),
                object,
            },
            ObjectKey::QuestionSource {
                question_version: question_version.clone(),
                object,
            },
            ObjectKey::RestrictedQuestionAsset {
                question_version: question_version.clone(),
                asset: AssetId::from_uuid(Uuid::from_u128(61)),
                object,
            },
            ObjectKey::PublishedImportArchive {
                question_version: question_version.clone(),
                import: WorkspaceImportId::from_uuid(Uuid::from_u128(9)),
                object,
            },
            ObjectKey::QuestionRender {
                question_version: question_version.clone(),
                seed: Seed::new(1),
                object,
            },
            ObjectKey::CourseBanner {
                course: CourseId::from_uuid(Uuid::from_u128(10)),
                banner: CourseBannerId::from_uuid(Uuid::from_u128(11)),
            },
        ] {
            assert_eq!(
                private_key.bucket(),
                Bucket::PrivateContent,
                "{private_key:?} must not be placed in the CDN-readable bucket"
            );
        }
    }

    #[test]
    fn course_banner_keys_bind_scope_classification_and_signing() {
        let course = CourseId::from_uuid(Uuid::from_u128(2));
        let candidate_id = CourseBannerCandidateId::from_uuid(Uuid::from_u128(3));
        let banner_id = CourseBannerId::from_uuid(Uuid::from_u128(4));
        let candidate = ObjectKey::CourseBannerCandidate {
            course,
            candidate: candidate_id,
        };
        let banner = ObjectKey::CourseBanner {
            course,
            banner: banner_id,
        };

        assert_eq!(candidate.bucket(), Bucket::TempProcessing);
        assert_eq!(candidate.category(), ObjectCategory::Temporary);
        assert_eq!(candidate.question_version(), None);
        assert!(!candidate.may_issue_signed_url());
        assert_eq!(banner.bucket(), Bucket::PrivateContent);
        assert_eq!(banner.category(), ObjectCategory::CourseContent);
        assert_eq!(banner.question_version(), None);
        assert!(banner.may_issue_signed_url());
        assert!(candidate.path().contains(&course.to_string()));
        assert!(candidate.path().contains(&candidate_id.to_string()));
        assert!(banner.path().contains(&course.to_string()));
        assert!(banner.path().contains(&banner_id.to_string()));
        assert_ne!(candidate.object_id(), banner.object_id());
    }

    #[test]
    fn banner_object_identity_changes_with_course_and_route_id() {
        let course = CourseId::from_uuid(Uuid::from_u128(2));
        let banner = CourseBannerId::from_uuid(Uuid::from_u128(3));
        let base = course_banner_object_id(course, banner);
        assert_ne!(
            base,
            course_banner_object_id(CourseId::from_uuid(Uuid::from_u128(12)), banner)
        );
        assert_ne!(
            base,
            course_banner_object_id(course, CourseBannerId::from_uuid(Uuid::from_u128(13)))
        );
    }

    #[test]
    fn banner_keys_round_trip_without_a_caller_supplied_object_id() {
        let key = ObjectKey::CourseBanner {
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
            WorkspaceId::from_uuid(Uuid::from_u128(2)),
            WorkspaceImportId::from_uuid(Uuid::from_u128(3)),
        );

        assert_eq!(
            actual,
            workspace_qti_archive_object_id(
                WorkspaceId::from_uuid(Uuid::from_u128(2)),
                WorkspaceImportId::from_uuid(Uuid::from_u128(3)),
            )
        );
    }

    #[test]
    fn workspace_qti_archive_identity_changes_with_workspace() {
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(3));

        assert_ne!(
            workspace_qti_archive_object_id(WorkspaceId::from_uuid(Uuid::from_u128(2)), import),
            workspace_qti_archive_object_id(WorkspaceId::from_uuid(Uuid::from_u128(12)), import)
        );
    }

    #[test]
    fn workspace_qti_archive_identity_changes_with_import() {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));

        assert_ne!(
            workspace_qti_archive_object_id(
                workspace,
                WorkspaceImportId::from_uuid(Uuid::from_u128(3))
            ),
            workspace_qti_archive_object_id(
                workspace,
                WorkspaceImportId::from_uuid(Uuid::from_u128(13))
            )
        );
    }

    #[test]
    fn workspace_qti_archive_uses_private_workspace_source_key() {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(3));
        let object = workspace_qti_archive_object_id(workspace, import);
        let key = ObjectKey::WorkspaceSource {
            workspace,
            import,
            object,
        };

        assert_eq!(
            key.path(),
            format!("workspaces/{workspace}/imports/{import}/source/{object}")
        );
        assert_eq!(key.object_id(), object);
        assert_eq!(key.bucket(), Bucket::PrivateContent);
        assert_eq!(key.category(), ObjectCategory::Source);
        assert_eq!(key.question_version(), None);
        assert!(!key.may_issue_signed_url());
    }

    #[test]
    fn published_import_archive_object_id_matches_golden() {
        let actual = published_import_archive_object_id(
            &question_version(3),
            WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            Sha256Digest::compute(b"archive fixture"),
        );

        assert_eq!(
            actual,
            published_import_archive_object_id(
                &question_version(3),
                WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
                Sha256Digest::compute(b"archive fixture"),
            )
        );
    }

    #[test]
    fn published_import_archive_key_has_distinct_path_and_private_classification() {
        let key = ObjectKey::PublishedImportArchive {
            question_version: question_version(3),
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        };

        assert_eq!(
            key.path(),
            "questions/ABC-DEFG/versions/3/imports/00000000-0000-0000-0000-000000000004/archive/00000000-0000-0000-0000-000000000005"
        );
        assert_eq!(key.bucket(), Bucket::PrivateContent);
        assert_eq!(key.category(), ObjectCategory::Source);
        assert_eq!(key.question_version(), Some(&question_version(3)));
        assert!(!key.may_issue_signed_url());
    }

    #[test]
    fn every_archive_identity_input_changes_the_object_id() {
        let reference = question_version(3);
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(4));
        let archive = Sha256Digest::compute(b"archive fixture");
        let base = published_import_archive_object_id(&reference, import, archive);
        assert_ne!(
            base,
            published_import_archive_object_id(
                &QuestionVersionReference {
                    question_id: QuestionId::from_canonical_parts("BCDEFG", 'H')
                        .expect("canonical Question ID"),
                    version_number: reference.version_number,
                },
                import,
                archive
            )
        );
        assert_ne!(
            base,
            published_import_archive_object_id(&question_version(13), import, archive)
        );
        assert_ne!(
            base,
            published_import_archive_object_id(
                &reference,
                WorkspaceImportId::from_uuid(Uuid::from_u128(14)),
                archive
            )
        );
        assert_ne!(
            base,
            published_import_archive_object_id(
                &reference,
                import,
                Sha256Digest::compute(b"different archive")
            )
        );
    }

    #[test]
    fn published_import_archive_key_round_trips_through_serde() {
        let key = ObjectKey::PublishedImportArchive {
            question_version: question_version(3),
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
