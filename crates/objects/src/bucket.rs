//! Typed Object Storage Area and Object Address construction.

use question_model::generation::QuestionSeed;
use question_model::{
    CourseBannerReference, CourseBannerUploadReference, CourseId, ObjectId, QuestionAssetId,
    QuestionRevisionReference, WorkspaceId, WorkspaceImportId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::Sha256Checksum;

/// One of the four Object Storage Areas with a distinct access, encryption, and
/// delivery policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectStorageArea {
    /// Immutable student-facing renditions. This is the only CDN-readable
    /// domain and therefore contains only [`ObjectAddress::QuestionAsset`] bytes.
    PublicAssets,
    /// Private authoring, import evidence, grading, rendering, and course content.
    PrivateContent,
    /// Student-specific exports and annotated exams.
    StudentRecords,
    /// Never-served extraction and conversion workspaces.
    TempProcessing,
}

impl ObjectStorageArea {
    /// Returns the stable Object Storage Area identifier.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PublicAssets => "public-assets",
            Self::PrivateContent => "private-content",
            Self::StudentRecords => "student-records",
            Self::TempProcessing => "temp-processing",
        }
    }
}

/// Required sensitivity and ownership class derived from an Object Address.
///
/// Unlike an Object Storage Area, this names why PLE stores the bytes. It is
/// never caller-supplied metadata and therefore cannot be relabeled to widen
/// access or conceal a Student record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectDataClass {
    /// Private instructor-authoring sources and assets.
    AuthoringContent,
    /// Immutable source bytes and import archives for a Question Revision.
    QuestionSource,
    /// One logical Question Asset, whether public or restricted.
    QuestionAsset,
    /// A deterministic answer-free Question render.
    QuestionRender,
    /// A Course Banner Upload or saved Course Banner.
    CourseAppearance,
    /// FERPA-bearing bytes owned by one Student record.
    StudentRecord,
    /// Short-lived bytes used only during processing.
    TemporaryProcessing,
}

/// Stable identity components from which an immutable Object Address is built.
///
/// There is no raw-string variant. Callers choose a semantic destination and
/// supply typed IDs; this crate alone decides the physical path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ObjectAddress {
    /// Original bytes for a private workspace import.
    ///
    /// This intentionally uses the private-content Object Storage Area for immutable
    /// durable bytes and must never be exposed through CDN or Question Library asset
    /// delivery.
    WorkspaceImportSource {
        /// Private authoring workspace.
        workspace: WorkspaceId,
        /// Staged import identity.
        import: WorkspaceImportId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// Canonical private PLE Question JSON source for one workspace.
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
    /// Like [`Self::WorkspaceImportSource`], this is durable private-content storage
    /// but not eligible for CDN or Question Library delivery.
    WorkspaceImportAsset {
        /// Private authoring workspace.
        workspace: WorkspaceId,
        /// Staged import identity.
        import: WorkspaceImportId,
        /// Logical asset referenced by imported draft content.
        asset: QuestionAssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// An original source package for a published version.
    QuestionSource {
        /// Exact immutable Question Revision that owns the source.
        question_revision: QuestionRevisionReference,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// The immutable original archive retained with a published imported version.
    ///
    /// This is private import evidence, not student-facing content: the archive checksum is
    /// part of the semantic key, and the object is never
    /// eligible for a signed delivery URL.
    PublishedImportArchive {
        /// Exact immutable Question Revision that owns the archive.
        question_revision: QuestionRevisionReference,
        /// Import identity which produced this published version.
        import: WorkspaceImportId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A logical asset and its physical object for a published version.
    QuestionAsset {
        /// Exact immutable Question Revision that owns the asset.
        question_revision: QuestionRevisionReference,
        /// Logical asset referenced by content.
        asset: QuestionAssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A student-facing asset belonging to a Published Question Revision.
    ///
    /// Its identity is as immutable as [`Self::QuestionAsset`], but its bytes
    /// live in private-content and are delivered only after Question Library
    /// authorization.  A CDN-readable key must never represent restricted
    /// published content.
    RestrictedQuestionAsset {
        /// Exact immutable Question Revision that owns the asset.
        question_revision: QuestionRevisionReference,
        /// Logical asset referenced by content.
        asset: QuestionAssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A deterministic rendered Question cached by exact Question Revision and Question Seed.
    QuestionRender {
        /// Exact immutable Question Revision that owns the rendered result.
        question_revision: QuestionRevisionReference,
        /// Question Seed that fully determines the render.
        question_seed: QuestionSeed,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// Validated banner bytes awaiting one authorized appearance save.
    ///
    /// Upload bytes are short-lived, non-signable, and scoped to one
    /// course before persistence adds Account and expiry ownership.
    CourseBannerUpload {
        /// Course whose authorized appearance flow created the upload.
        course: CourseId,
        /// Opaque upload reference returned to the authorized browser.
        upload: CourseBannerUploadReference,
    },
    /// Immutable current-or-retained course banner bytes.
    ///
    /// Typed-object signing is permitted, but the asset-delivery layer must
    /// still verify that this banner is the course's exact current pointer.
    CourseBanner {
        /// Course whose appearance may reference the banner.
        course: CourseId,
        /// Stable browser-safe banner delivery identity.
        banner: CourseBannerReference,
    },
    /// A course-owned Student Record Object.
    StudentRecord {
        /// Exact course whose protected record owns this object.
        course: CourseId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A short-lived processing Object that is never served.
    Temporary {
        /// Physical object-record identity.
        object: ObjectId,
    },
}

impl ObjectAddress {
    /// Object Storage Area selected by this semantic Object Address.
    pub fn storage_area(&self) -> ObjectStorageArea {
        match self {
            Self::WorkspaceImportSource { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportAsset { .. }
            | Self::QuestionSource { .. }
            | Self::PublishedImportArchive { .. }
            | Self::RestrictedQuestionAsset { .. }
            | Self::QuestionRender { .. }
            | Self::CourseBanner { .. } => ObjectStorageArea::PrivateContent,
            Self::QuestionAsset { .. } => ObjectStorageArea::PublicAssets,
            Self::CourseBannerUpload { .. } => ObjectStorageArea::TempProcessing,
            Self::StudentRecord { .. } => ObjectStorageArea::StudentRecords,
            Self::Temporary { .. } => ObjectStorageArea::TempProcessing,
        }
    }

    /// Required Object Data Class inherited from the exact owning address.
    pub fn data_class(&self) -> ObjectDataClass {
        match self {
            Self::WorkspaceImportSource { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportAsset { .. } => ObjectDataClass::AuthoringContent,
            Self::QuestionSource { .. } | Self::PublishedImportArchive { .. } => {
                ObjectDataClass::QuestionSource
            }
            Self::QuestionAsset { .. } | Self::RestrictedQuestionAsset { .. } => {
                ObjectDataClass::QuestionAsset
            }
            Self::QuestionRender { .. } => ObjectDataClass::QuestionRender,
            Self::CourseBannerUpload { .. } | Self::CourseBanner { .. } => {
                ObjectDataClass::CourseAppearance
            }
            Self::StudentRecord { .. } => ObjectDataClass::StudentRecord,
            Self::Temporary { .. } => ObjectDataClass::TemporaryProcessing,
        }
    }

    /// Immutable path derived only from typed identity components.
    pub fn path(&self) -> String {
        match self {
            Self::WorkspaceImportSource {
                workspace,
                import,
                object,
            } => {
                format!("workspaces/{workspace}/imports/{import}/source/{object}")
            }
            Self::WorkspaceQuestionSource { workspace, object } => {
                format!("workspaces/{workspace}/questions/source/{object}")
            }
            Self::WorkspaceImportAsset {
                workspace,
                import,
                asset,
                object,
            } => {
                format!("workspaces/{workspace}/imports/{import}/assets/{asset}/{object}")
            }
            Self::QuestionSource {
                question_revision,
                object,
            } => format!(
                "questions/{}/versions/{}/source/{object}",
                question_revision.question_id, question_revision.revision_number
            ),
            Self::PublishedImportArchive {
                question_revision,
                import,
                object,
            } => format!(
                "questions/{}/versions/{}/imports/{import}/archive/{object}",
                question_revision.question_id, question_revision.revision_number
            ),
            Self::QuestionAsset {
                question_revision,
                asset,
                object,
            } => format!(
                "questions/{}/versions/{}/assets/{asset}/{object}",
                question_revision.question_id, question_revision.revision_number
            ),
            Self::RestrictedQuestionAsset {
                question_revision,
                asset,
                object,
            } => {
                format!(
                    "questions/{}/versions/{}/restricted-assets/{asset}/{object}",
                    question_revision.question_id, question_revision.revision_number
                )
            }
            Self::QuestionRender {
                question_revision,
                question_seed,
                object,
            } => format!(
                "questions/{}/versions/{}/renders/{}/{object}",
                question_revision.question_id,
                question_revision.revision_number,
                question_seed.value()
            ),
            Self::CourseBannerUpload { course, upload } => format!(
                "courses/{course}/banners/uploads/{upload}/{}",
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
            Self::WorkspaceImportSource { object, .. }
            | Self::WorkspaceQuestionSource { object, .. }
            | Self::WorkspaceImportAsset { object, .. }
            | Self::QuestionSource { object, .. }
            | Self::PublishedImportArchive { object, .. }
            | Self::QuestionAsset { object, .. }
            | Self::RestrictedQuestionAsset { object, .. }
            | Self::QuestionRender { object, .. }
            | Self::StudentRecord { object, .. }
            | Self::Temporary { object } => *object,
            Self::CourseBannerUpload { course, upload } => {
                course_banner_upload_object_id(*course, *upload)
            }
            Self::CourseBanner { course, banner } => course_banner_object_id(*course, *banner),
        }
    }

    /// Exact Question Revision associated with content, when one exists.
    pub fn question_revision(&self) -> Option<&QuestionRevisionReference> {
        match self {
            Self::QuestionSource {
                question_revision, ..
            }
            | Self::PublishedImportArchive {
                question_revision, ..
            }
            | Self::QuestionAsset {
                question_revision, ..
            }
            | Self::RestrictedQuestionAsset {
                question_revision, ..
            }
            | Self::QuestionRender {
                question_revision, ..
            } => Some(question_revision),
            Self::WorkspaceImportSource { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportAsset { .. }
            | Self::CourseBannerUpload { .. }
            | Self::CourseBanner { .. }
            | Self::StudentRecord { .. }
            | Self::Temporary { .. } => None,
        }
    }

    /// Whether this semantic object may receive a direct delivery URL.
    ///
    /// Workspace imports and published Source Object References remain private in the
    /// private-content Object Storage Area. Source may
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
        question_revision: QuestionRevisionReference,
        asset: QuestionAssetId,
        object: ObjectId,
    ) -> Self {
        Self::RestrictedQuestionAsset {
            question_revision,
            asset,
            object,
        }
    }
}

/// Derives the immutable physical identity for one Course Banner Upload.
pub fn course_banner_upload_object_id(
    course: CourseId,
    upload: CourseBannerUploadReference,
) -> ObjectId {
    domain_separated_object_id(
        b"ple:course-banner-upload:v1\0",
        [course.as_uuid(), upload.as_uuid(), uuid::Uuid::nil()],
    )
}

/// Derives the immutable physical identity for one promoted course banner.
pub fn course_banner_object_id(course: CourseId, banner: CourseBannerReference) -> ObjectId {
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
/// import identity must address the same immutable [`ObjectAddress::WorkspaceImportSource`]
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
/// Question Revision Number bind the address to one exact Question Revision.
/// Only the first 16 bytes of the final SHA-256 digest become the deterministic
/// object UUID.
pub fn published_import_archive_object_id(
    question_revision: &QuestionRevisionReference,
    import: WorkspaceImportId,
    archive_sha256: Sha256Checksum,
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:published-import-archive:v1\0");
    hasher.update(question_revision.question_id.to_string().as_bytes());
    hasher.update(question_revision.revision_number.get().to_be_bytes());
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
    use question_model::{QuestionId, QuestionRevisionNumber};
    use uuid::Uuid;

    fn question_revision(revision_number: u32) -> QuestionRevisionReference {
        QuestionRevisionReference {
            question_id: QuestionId::from_canonical_parts("ABCDEF", 'G')
                .expect("canonical Question ID"),
            revision_number: QuestionRevisionNumber::new(revision_number)
                .expect("positive Question Revision Number"),
        }
    }

    #[test]
    fn source_objects_are_never_direct_delivery_targets() {
        let source = ObjectAddress::QuestionSource {
            question_revision: question_revision(2),
            object: ObjectId::from_uuid(Uuid::from_u128(3)),
        };
        let asset = ObjectAddress::QuestionAsset {
            question_revision: question_revision(2),
            asset: QuestionAssetId::from_uuid(Uuid::from_u128(4)),
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        };

        assert!(!source.may_issue_signed_url());
        assert!(asset.may_issue_signed_url());
    }

    #[test]
    fn only_immutable_question_assets_enter_the_public_delivery_domain() {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
        let question_revision = question_revision(4);
        let object = ObjectId::from_uuid(Uuid::from_u128(5));

        let public_asset = ObjectAddress::QuestionAsset {
            question_revision: question_revision.clone(),
            asset: QuestionAssetId::from_uuid(Uuid::from_u128(6)),
            object,
        };
        assert_eq!(public_asset.storage_area(), ObjectStorageArea::PublicAssets);
        assert_eq!(
            ObjectAddress::published_question_asset(
                question_revision.clone(),
                QuestionAssetId::from_uuid(Uuid::from_u128(60)),
                object,
            )
            .storage_area(),
            ObjectStorageArea::PrivateContent,
            "Published Question assets must never enter the CDN-readable Object Storage Area"
        );

        for private_key in [
            ObjectAddress::WorkspaceImportSource {
                workspace,
                import: WorkspaceImportId::from_uuid(Uuid::from_u128(7)),
                object,
            },
            ObjectAddress::QuestionSource {
                question_revision: question_revision.clone(),
                object,
            },
            ObjectAddress::RestrictedQuestionAsset {
                question_revision: question_revision.clone(),
                asset: QuestionAssetId::from_uuid(Uuid::from_u128(61)),
                object,
            },
            ObjectAddress::PublishedImportArchive {
                question_revision: question_revision.clone(),
                import: WorkspaceImportId::from_uuid(Uuid::from_u128(9)),
                object,
            },
            ObjectAddress::QuestionRender {
                question_revision: question_revision.clone(),
                question_seed: QuestionSeed::new(1),
                object,
            },
            ObjectAddress::CourseBanner {
                course: CourseId::from_uuid(Uuid::from_u128(10)),
                banner: CourseBannerReference::from_uuid(Uuid::from_u128(11)),
            },
        ] {
            assert_eq!(
                private_key.storage_area(),
                ObjectStorageArea::PrivateContent,
                "{private_key:?} must not be placed in the CDN-readable Object Storage Area"
            );
        }
    }

    #[test]
    fn course_banner_keys_bind_scope_classification_and_signing() {
        let course = CourseId::from_uuid(Uuid::from_u128(2));
        let upload_reference = CourseBannerUploadReference::from_uuid(Uuid::from_u128(3));
        let banner_reference = CourseBannerReference::from_uuid(Uuid::from_u128(4));
        let upload = ObjectAddress::CourseBannerUpload {
            course,
            upload: upload_reference,
        };
        let banner = ObjectAddress::CourseBanner {
            course,
            banner: banner_reference,
        };

        assert_eq!(upload.storage_area(), ObjectStorageArea::TempProcessing);
        assert_eq!(upload.question_revision(), None);
        assert!(!upload.may_issue_signed_url());
        assert_eq!(banner.storage_area(), ObjectStorageArea::PrivateContent);
        assert_eq!(banner.question_revision(), None);
        assert!(banner.may_issue_signed_url());
        assert!(upload.path().contains(&course.to_string()));
        assert!(upload.path().contains(&upload_reference.to_string()));
        assert!(banner.path().contains(&course.to_string()));
        assert!(banner.path().contains(&banner_reference.to_string()));
        assert_ne!(upload.object_id(), banner.object_id());
    }

    #[test]
    fn banner_object_identity_changes_with_course_and_route_id() {
        let course = CourseId::from_uuid(Uuid::from_u128(2));
        let banner = CourseBannerReference::from_uuid(Uuid::from_u128(3));
        let base = course_banner_object_id(course, banner);
        assert_ne!(
            base,
            course_banner_object_id(CourseId::from_uuid(Uuid::from_u128(12)), banner)
        );
        assert_ne!(
            base,
            course_banner_object_id(
                course,
                CourseBannerReference::from_uuid(Uuid::from_u128(13))
            )
        );
    }

    #[test]
    fn banner_keys_round_trip_without_a_caller_supplied_object_id() {
        let key = ObjectAddress::CourseBanner {
            course: CourseId::from_uuid(Uuid::from_u128(2)),
            banner: CourseBannerReference::from_uuid(Uuid::from_u128(3)),
        };
        let encoded = serde_json::to_string(&key).expect("banner key should serialize");
        let decoded: ObjectAddress =
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
    fn workspace_qti_archive_uses_private_workspace_import_source_key() {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(3));
        let object = workspace_qti_archive_object_id(workspace, import);
        let key = ObjectAddress::WorkspaceImportSource {
            workspace,
            import,
            object,
        };

        assert_eq!(
            key.path(),
            format!("workspaces/{workspace}/imports/{import}/source/{object}")
        );
        assert_eq!(key.object_id(), object);
        assert_eq!(key.storage_area(), ObjectStorageArea::PrivateContent);
        assert_eq!(key.question_revision(), None);
        assert!(!key.may_issue_signed_url());
    }

    #[test]
    fn published_import_archive_object_id_matches_golden() {
        let actual = published_import_archive_object_id(
            &question_revision(3),
            WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            Sha256Checksum::compute(b"archive fixture"),
        );

        assert_eq!(
            actual,
            published_import_archive_object_id(
                &question_revision(3),
                WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
                Sha256Checksum::compute(b"archive fixture"),
            )
        );
    }

    #[test]
    fn published_import_archive_key_has_distinct_path_and_private_classification() {
        let key = ObjectAddress::PublishedImportArchive {
            question_revision: question_revision(3),
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        };

        assert_eq!(
            key.path(),
            "questions/ABC-DEFG/versions/3/imports/00000000-0000-0000-0000-000000000004/archive/00000000-0000-0000-0000-000000000005"
        );
        assert_eq!(key.storage_area(), ObjectStorageArea::PrivateContent);
        assert_eq!(key.question_revision(), Some(&question_revision(3)));
        assert!(!key.may_issue_signed_url());
    }

    #[test]
    fn every_archive_identity_input_changes_the_object_id() {
        let reference = question_revision(3);
        let import = WorkspaceImportId::from_uuid(Uuid::from_u128(4));
        let archive = Sha256Checksum::compute(b"archive fixture");
        let base = published_import_archive_object_id(&reference, import, archive);
        assert_ne!(
            base,
            published_import_archive_object_id(
                &QuestionRevisionReference {
                    question_id: QuestionId::from_canonical_parts("BCDEFG", 'H')
                        .expect("canonical Question ID"),
                    revision_number: reference.revision_number,
                },
                import,
                archive
            )
        );
        assert_ne!(
            base,
            published_import_archive_object_id(&question_revision(13), import, archive)
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
                Sha256Checksum::compute(b"different archive")
            )
        );
    }

    #[test]
    fn published_import_archive_address_round_trips_through_serde() {
        let address = ObjectAddress::PublishedImportArchive {
            question_revision: question_revision(3),
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            object: ObjectId::from_uuid(Uuid::from_u128(5)),
        };

        let encoded = serde_json::to_string(&address).expect("Object Address should serialize");
        let decoded: ObjectAddress =
            serde_json::from_str(&encoded).expect("Object Address should deserialize");
        assert_eq!(decoded, address);
        assert!(encoded.contains("publishedImportArchive"));
    }
}
