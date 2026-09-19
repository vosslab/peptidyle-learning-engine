//! Typed Object Storage Area and Object Address construction.

use question_model::generation::QuestionSeed;
use question_model::{
    CourseBannerId, CourseBannerRendition, CourseBannerUploadId, CourseInstanceId, ObjectId,
    ProfileImageId, QuestionAssetId, QuestionRevisionTuple, WorkspaceId, WorkspaceImportId,
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
    /// A normalized, self-authorized Account Profile image.
    ProfileImage,
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
    /// Immutable raster staged for one real private Draft Question.
    DraftQuestionAsset {
        /// Private authoring workspace.
        workspace: WorkspaceId,
        /// Internal Draft identity, never its browser reference.
        draft_question_uuid: uuid::Uuid,
        /// Stable logical asset identity.
        asset: QuestionAssetId,
        /// Fresh physical object identity.
        object: ObjectId,
    },
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
        question_revision: QuestionRevisionTuple,
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
        question_revision: QuestionRevisionTuple,
        /// Import identity which produced this published version.
        import: WorkspaceImportId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A logical asset and its physical object for a published version.
    QuestionAsset {
        /// Exact immutable Question Revision that owns the asset.
        question_revision: QuestionRevisionTuple,
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
        question_revision: QuestionRevisionTuple,
        /// Logical asset referenced by content.
        asset: QuestionAssetId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A deterministic rendered Question cached by exact Question Revision and Question Seed.
    QuestionRender {
        /// Exact immutable Question Revision that owns the rendered result.
        question_revision: QuestionRevisionTuple,
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
        course: CourseInstanceId,
        /// Opaque upload reference returned to the authorized browser.
        upload: CourseBannerUploadId,
    },
    /// Immutable verified private source retained for one Course Banner.
    CourseBannerSource {
        /// Course whose appearance may reference the banner.
        course: CourseInstanceId,
        /// Stable browser-safe banner delivery identity.
        banner: CourseBannerId,
    },
    /// Immutable normalized private delivery rendition for one Course Banner.
    CourseBannerRendition {
        /// Course whose appearance may reference the banner.
        course: CourseInstanceId,
        /// Stable browser-safe banner delivery identity.
        banner: CourseBannerId,
        /// Closed, server-owned rendition identity.
        rendition: CourseBannerRendition,
    },
    /// One normalized private rendition for a self-owned Account Profile image.
    ProfileImage {
        /// Opaque role-neutral image reference minted only by the server.
        image: ProfileImageId,
        /// Physical object-record identity.
        object: ObjectId,
    },
    /// A course-owned Student Record Object.
    StudentRecord {
        /// Exact course whose protected record owns this object.
        course: CourseInstanceId,
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
            | Self::DraftQuestionAsset { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportAsset { .. }
            | Self::QuestionSource { .. }
            | Self::PublishedImportArchive { .. }
            | Self::RestrictedQuestionAsset { .. }
            | Self::QuestionRender { .. }
            | Self::CourseBannerSource { .. }
            | Self::CourseBannerRendition { .. }
            | Self::ProfileImage { .. } => ObjectStorageArea::PrivateContent,
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
            | Self::DraftQuestionAsset { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportAsset { .. } => ObjectDataClass::AuthoringContent,
            Self::QuestionSource { .. } | Self::PublishedImportArchive { .. } => {
                ObjectDataClass::QuestionSource
            }
            Self::QuestionAsset { .. } | Self::RestrictedQuestionAsset { .. } => {
                ObjectDataClass::QuestionAsset
            }
            Self::QuestionRender { .. } => ObjectDataClass::QuestionRender,
            Self::CourseBannerUpload { .. }
            | Self::CourseBannerSource { .. }
            | Self::CourseBannerRendition { .. } => ObjectDataClass::CourseAppearance,
            Self::ProfileImage { .. } => ObjectDataClass::ProfileImage,
            Self::StudentRecord { .. } => ObjectDataClass::StudentRecord,
            Self::Temporary { .. } => ObjectDataClass::TemporaryProcessing,
        }
    }

    /// Immutable path derived only from typed identity components.
    pub fn path(&self) -> String {
        match self {
            Self::DraftQuestionAsset {
                workspace,
                draft_question_uuid,
                asset,
                object,
            } => {
                // ASVS 5.3.2: only server-owned identities determine the key.
                format!(
                    "workspaces/{workspace}/questions/drafts/{draft_question_uuid}/assets/{asset}/{object}"
                )
            }
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
                question_revision.question_id.as_str(),
                question_revision.revision_number
            ),
            Self::PublishedImportArchive {
                question_revision,
                import,
                object,
            } => format!(
                "questions/{}/versions/{}/imports/{import}/archive/{object}",
                question_revision.question_id.as_str(),
                question_revision.revision_number
            ),
            Self::QuestionAsset {
                question_revision,
                asset,
                object,
            } => format!(
                "questions/{}/versions/{}/assets/{asset}/{object}",
                question_revision.question_id.as_str(),
                question_revision.revision_number
            ),
            Self::RestrictedQuestionAsset {
                question_revision,
                asset,
                object,
            } => {
                format!(
                    "questions/{}/versions/{}/restricted-assets/{asset}/{object}",
                    question_revision.question_id.as_str(),
                    question_revision.revision_number
                )
            }
            Self::QuestionRender {
                question_revision,
                question_seed,
                object,
            } => format!(
                "questions/{}/versions/{}/renders/{}/{object}",
                question_revision.question_id.as_str(),
                question_revision.revision_number,
                question_seed.value()
            ),
            Self::CourseBannerUpload { course, upload } => format!(
                "courses/{course}/banners/uploads/{upload}/{}",
                self.object_id()
            ),
            Self::CourseBannerSource { course, banner } => format!(
                "courses/{course}/banners/{banner}/source/{}",
                self.object_id()
            ),
            Self::CourseBannerRendition {
                course,
                banner,
                rendition,
            } => format!(
                "courses/{course}/banners/{banner}/renditions/{}/{}",
                rendition.as_str(),
                self.object_id()
            ),
            Self::ProfileImage { image, object } => {
                // ASVS 5.3.2: this storage path is constructed exclusively
                // from server-owned typed identifiers, never a filename.
                format!("profiles/images/{image}/{object}")
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
            | Self::DraftQuestionAsset { object, .. }
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
                course_banner_upload_object_id(course, *upload)
            }
            Self::CourseBannerSource { course, banner } => {
                course_banner_source_object_id(course, *banner)
            }
            Self::CourseBannerRendition {
                course,
                banner,
                rendition,
            } => course_banner_rendition_object_id(course, *banner, *rendition),
            Self::ProfileImage { object, .. } => *object,
        }
    }

    /// Exact Question Revision associated with content, when one exists.
    pub fn question_revision(&self) -> Option<&QuestionRevisionTuple> {
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
            | Self::DraftQuestionAsset { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportAsset { .. }
            | Self::CourseBannerUpload { .. }
            | Self::CourseBannerSource { .. }
            | Self::CourseBannerRendition { .. }
            | Self::ProfileImage { .. }
            | Self::StudentRecord { .. }
            | Self::Temporary { .. } => None,
        }
    }

    /// Whether this semantic object may receive a direct delivery URL.
    ///
    /// Workspace imports and published Source Object IDs remain private in the
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
                | Self::CourseBannerRendition { .. }
                | Self::ProfileImage { .. }
                | Self::StudentRecord { .. }
        )
    }

    /// Chooses the physical immutable asset domain from the publication's
    /// immutable visibility.  Call this at publication time rather than
    /// reconstructing a key later from an untrusted route or browser value.
    pub fn published_question_asset(
        question_revision: QuestionRevisionTuple,
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
    course: &CourseInstanceId,
    upload: CourseBannerUploadId,
) -> ObjectId {
    domain_separated_object_id_from_parts(
        b"ple:course-banner-upload:v3\0",
        course.as_str().as_bytes(),
        upload.as_uuid().as_bytes(),
        uuid::Uuid::nil().as_bytes(),
    )
}

/// Derives the immutable physical identity for one promoted course banner.
pub fn course_banner_source_object_id(
    course: &CourseInstanceId,
    banner: CourseBannerId,
) -> ObjectId {
    domain_separated_object_id_from_parts(
        b"ple:course-banner-source:v3\0",
        course.as_str().as_bytes(),
        banner.as_uuid().as_bytes(),
        uuid::Uuid::nil().as_bytes(),
    )
}

/// Derives the immutable physical identity for one normalized course-banner rendition.
pub fn course_banner_rendition_object_id(
    course: &CourseInstanceId,
    banner: CourseBannerId,
    rendition: CourseBannerRendition,
) -> ObjectId {
    let rendition_uuid = match rendition {
        CourseBannerRendition::Banner => uuid::Uuid::from_u128(1),
    };
    domain_separated_object_id_from_parts(
        // `v3` hashes the Course public ID. Pre-production has no durable
        // object-store rows to preserve.
        b"ple:course-banner-rendition:v3\0",
        course.as_str().as_bytes(),
        banner.as_uuid().as_bytes(),
        rendition_uuid.as_bytes(),
    )
}

fn domain_separated_object_id_from_parts(
    domain: &[u8],
    first: &[u8],
    second: &[u8],
    third: &[u8],
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(first);
    hasher.update(second);
    hasher.update(third);
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
    question_revision: &QuestionRevisionTuple,
    import: WorkspaceImportId,
    archive_sha256: Sha256Checksum,
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:published-import-archive:v2\0");
    hasher.update(question_revision.question_id.as_str().as_bytes());
    hasher.update(question_revision.revision_number.get().to_be_bytes());
    hasher.update(import.as_uuid().as_bytes());
    hasher.update(archive_sha256.as_bytes());

    let digest = hasher.finalize();
    let mut object_uuid = [0_u8; 16];
    object_uuid.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(uuid::Uuid::from_bytes(object_uuid))
}

#[cfg(test)]
#[path = "bucket_tests.rs"]
mod tests;
