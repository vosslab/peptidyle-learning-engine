//! Typed Object Storage Area and Object Address construction.

use question_model::generation::QuestionSeed;
use question_model::{
    CourseBannerId, CourseBannerRendition, CourseBannerUploadId, CourseInstanceId, ObjectId,
    ProfileImageId, QuestionImageAssetId, QuestionRevisionTuple, WorkspaceId, WorkspaceImportId,
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
    /// domain and therefore contains only [`ObjectAddress::QuestionImage`] bytes.
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
    /// One logical Question Image, whether public or restricted.
    QuestionImage,
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
    DraftQuestionImage {
        /// Private authoring workspace.
        workspace_id: WorkspaceId,
        /// Internal Draft identity, never a browser locator.
        draft_question_id: uuid::Uuid,
        /// Stable Question Image Asset identity.
        question_image_asset_id: QuestionImageAssetId,
        /// Fresh physical object identity.
        object_id: ObjectId,
    },
    /// Original bytes for a private workspace import.
    ///
    /// This intentionally uses the private-content Object Storage Area for immutable
    /// durable bytes and must never be exposed through CDN or Question Library image
    /// delivery.
    WorkspaceImportSource {
        /// Private authoring workspace.
        workspace_id: WorkspaceId,
        /// Staged import identity.
        workspace_import_id: WorkspaceImportId,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// Canonical private PLE Question JSON source for one workspace.
    ///
    /// This is an authored, private source payload distinct from staged import
    /// packages.
    WorkspaceQuestionSource {
        /// Private authoring workspace.
        workspace_id: WorkspaceId,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// A verified still image extracted from a private workspace import.
    ///
    /// Like [`Self::WorkspaceImportSource`], this is durable private-content storage
    /// but not eligible for CDN or Question Library delivery.
    WorkspaceImportExtractedImage {
        /// Private authoring workspace.
        workspace_id: WorkspaceId,
        /// Staged import identity.
        workspace_import_id: WorkspaceImportId,
        /// Question Image Asset referenced by imported draft content.
        question_image_asset_id: QuestionImageAssetId,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// An original source package for a published version.
    QuestionSource {
        /// Exact immutable Question Revision that owns the source.
        question_revision_tuple: QuestionRevisionTuple,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// The immutable original archive retained with a published imported version.
    ///
    /// This is private import evidence, not student-facing content: the archive checksum is
    /// part of the semantic key, and the object is never
    /// eligible for a signed delivery URL.
    PublishedImportArchive {
        /// Exact immutable Question Revision that owns the archive.
        question_revision_tuple: QuestionRevisionTuple,
        /// Import identity which produced this published version.
        workspace_import_id: WorkspaceImportId,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// A Question Image Asset and its physical object for a published version.
    QuestionImage {
        /// Exact immutable Question Revision that owns the image.
        question_revision_tuple: QuestionRevisionTuple,
        /// Question Image Asset referenced by content.
        question_image_asset_id: QuestionImageAssetId,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// A student-facing Question Image belonging to a Published Question Revision.
    ///
    /// Its identity is as immutable as [`Self::QuestionImage`], but its bytes
    /// live in private-content and are delivered only after Question Library
    /// authorization.  A CDN-readable key must never represent restricted
    /// published content.
    RestrictedQuestionImage {
        /// Exact immutable Question Revision that owns the image.
        question_revision_tuple: QuestionRevisionTuple,
        /// Question Image Asset referenced by content.
        question_image_asset_id: QuestionImageAssetId,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// A deterministic rendered Question cached by exact Question Revision and Question Seed.
    QuestionRender {
        /// Exact immutable Question Revision that owns the rendered result.
        question_revision_tuple: QuestionRevisionTuple,
        /// Question Seed that fully determines the render.
        question_seed: QuestionSeed,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// Validated banner bytes awaiting one authorized appearance save.
    ///
    /// Upload bytes are short-lived, non-signable, and scoped to one
    /// course before persistence adds Account and expiry ownership.
    CourseBannerUpload {
        /// Course whose authorized appearance flow created the upload.
        course_instance_id: CourseInstanceId,
        /// Opaque upload ID returned to the authorized browser.
        course_banner_upload_id: CourseBannerUploadId,
    },
    /// Immutable verified private source retained for one Course Banner.
    CourseBannerSource {
        /// Course whose appearance may reference the banner.
        course_instance_id: CourseInstanceId,
        /// Stable browser-safe banner delivery identity.
        course_banner_id: CourseBannerId,
    },
    /// Immutable normalized private delivery rendition for one Course Banner.
    CourseBannerRendition {
        /// Course whose appearance may reference the banner.
        course_instance_id: CourseInstanceId,
        /// Stable browser-safe banner delivery identity.
        course_banner_id: CourseBannerId,
        /// Closed, server-owned rendition identity.
        rendition: CourseBannerRendition,
    },
    /// One normalized private rendition for a self-owned Account Profile image.
    ProfileImage {
        /// Opaque role-neutral image ID minted only by the server.
        profile_image_id: ProfileImageId,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// A course-owned Student Record Object.
    StudentRecord {
        /// Exact course whose protected record owns this object.
        course_instance_id: CourseInstanceId,
        /// Physical object-record identity.
        object_id: ObjectId,
    },
    /// A short-lived processing Object that is never served.
    Temporary {
        /// Physical object-record identity.
        object_id: ObjectId,
    },
}

impl ObjectAddress {
    /// Object Storage Area selected by this semantic Object Address.
    pub fn storage_area(&self) -> ObjectStorageArea {
        match self {
            Self::WorkspaceImportSource { .. }
            | Self::DraftQuestionImage { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportExtractedImage { .. }
            | Self::QuestionSource { .. }
            | Self::PublishedImportArchive { .. }
            | Self::RestrictedQuestionImage { .. }
            | Self::QuestionRender { .. }
            | Self::CourseBannerSource { .. }
            | Self::CourseBannerRendition { .. }
            | Self::ProfileImage { .. } => ObjectStorageArea::PrivateContent,
            Self::QuestionImage { .. } => ObjectStorageArea::PublicAssets,
            Self::CourseBannerUpload { .. } => ObjectStorageArea::TempProcessing,
            Self::StudentRecord { .. } => ObjectStorageArea::StudentRecords,
            Self::Temporary { .. } => ObjectStorageArea::TempProcessing,
        }
    }

    /// Required Object Data Class inherited from the exact owning address.
    pub fn data_class(&self) -> ObjectDataClass {
        match self {
            Self::WorkspaceImportSource { .. }
            | Self::DraftQuestionImage { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportExtractedImage { .. } => ObjectDataClass::AuthoringContent,
            Self::QuestionSource { .. } | Self::PublishedImportArchive { .. } => {
                ObjectDataClass::QuestionSource
            }
            Self::QuestionImage { .. } | Self::RestrictedQuestionImage { .. } => {
                ObjectDataClass::QuestionImage
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
            Self::DraftQuestionImage {
                workspace_id,
                draft_question_id,
                question_image_asset_id,
                object_id,
            } => {
                // ASVS 5.3.2: only server-owned identities determine the key.
                format!(
                    "workspaces/{workspace_id}/questions/drafts/{draft_question_id}/images/{question_image_asset_id}/{object_id}"
                )
            }
            Self::WorkspaceImportSource {
                workspace_id,
                workspace_import_id,
                object_id,
            } => {
                format!(
                    "workspaces/{workspace_id}/imports/{workspace_import_id}/source/{object_id}"
                )
            }
            Self::WorkspaceQuestionSource {
                workspace_id,
                object_id,
            } => {
                format!("workspaces/{workspace_id}/questions/source/{object_id}")
            }
            Self::WorkspaceImportExtractedImage {
                workspace_id,
                workspace_import_id,
                question_image_asset_id,
                object_id,
            } => {
                format!(
                    "workspaces/{workspace_id}/imports/{workspace_import_id}/extracted-images/{question_image_asset_id}/{object_id}"
                )
            }
            Self::QuestionSource {
                question_revision_tuple,
                object_id,
            } => format!(
                "questions/{}/versions/{}/source/{object_id}",
                question_revision_tuple.question_id.as_str(),
                question_revision_tuple.revision_number
            ),
            Self::PublishedImportArchive {
                question_revision_tuple,
                workspace_import_id,
                object_id,
            } => format!(
                "questions/{}/versions/{}/imports/{workspace_import_id}/archive/{object_id}",
                question_revision_tuple.question_id.as_str(),
                question_revision_tuple.revision_number
            ),
            Self::QuestionImage {
                question_revision_tuple,
                question_image_asset_id,
                object_id,
            } => format!(
                "questions/{}/versions/{}/images/{question_image_asset_id}/{object_id}",
                question_revision_tuple.question_id.as_str(),
                question_revision_tuple.revision_number
            ),
            Self::RestrictedQuestionImage {
                question_revision_tuple,
                question_image_asset_id,
                object_id,
            } => {
                format!(
                    "questions/{}/versions/{}/restricted-images/{question_image_asset_id}/{object_id}",
                    question_revision_tuple.question_id.as_str(),
                    question_revision_tuple.revision_number
                )
            }
            Self::QuestionRender {
                question_revision_tuple,
                question_seed,
                object_id,
            } => format!(
                "questions/{}/versions/{}/renders/{}/{object_id}",
                question_revision_tuple.question_id.as_str(),
                question_revision_tuple.revision_number,
                question_seed.value()
            ),
            Self::CourseBannerUpload {
                course_instance_id,
                course_banner_upload_id,
            } => format!(
                "courses/{course_instance_id}/banners/uploads/{course_banner_upload_id}/{}",
                self.object_id()
            ),
            Self::CourseBannerSource {
                course_instance_id,
                course_banner_id,
            } => format!(
                "courses/{course_instance_id}/banners/{course_banner_id}/source/{}",
                self.object_id()
            ),
            Self::CourseBannerRendition {
                course_instance_id,
                course_banner_id,
                rendition,
            } => format!(
                "courses/{course_instance_id}/banners/{course_banner_id}/renditions/{}/{}",
                rendition.as_str(),
                self.object_id()
            ),
            Self::ProfileImage {
                profile_image_id,
                object_id,
            } => {
                // ASVS 5.3.2: this storage path is constructed exclusively
                // from server-owned typed identifiers, never a filename.
                format!("profiles/images/{profile_image_id}/{object_id}")
            }
            Self::StudentRecord {
                course_instance_id,
                object_id,
            } => {
                format!("courses/{course_instance_id}/records/{object_id}")
            }
            Self::Temporary { object_id } => format!("processing/{object_id}"),
        }
    }

    /// Object-record identity embedded in the key.
    pub fn object_id(&self) -> ObjectId {
        match self {
            Self::WorkspaceImportSource { object_id, .. }
            | Self::DraftQuestionImage { object_id, .. }
            | Self::WorkspaceQuestionSource { object_id, .. }
            | Self::WorkspaceImportExtractedImage { object_id, .. }
            | Self::QuestionSource { object_id, .. }
            | Self::PublishedImportArchive { object_id, .. }
            | Self::QuestionImage { object_id, .. }
            | Self::RestrictedQuestionImage { object_id, .. }
            | Self::QuestionRender { object_id, .. }
            | Self::StudentRecord { object_id, .. }
            | Self::Temporary { object_id } => *object_id,
            Self::CourseBannerUpload {
                course_instance_id,
                course_banner_upload_id,
            } => course_banner_upload_object_id(course_instance_id, *course_banner_upload_id),
            Self::CourseBannerSource {
                course_instance_id,
                course_banner_id,
            } => course_banner_source_object_id(course_instance_id, *course_banner_id),
            Self::CourseBannerRendition {
                course_instance_id,
                course_banner_id,
                rendition,
            } => {
                course_banner_rendition_object_id(course_instance_id, *course_banner_id, *rendition)
            }
            Self::ProfileImage { object_id, .. } => *object_id,
        }
    }

    /// Exact Question Revision associated with content, when one exists.
    pub fn question_revision_tuple(&self) -> Option<&QuestionRevisionTuple> {
        match self {
            Self::QuestionSource {
                question_revision_tuple,
                ..
            }
            | Self::PublishedImportArchive {
                question_revision_tuple,
                ..
            }
            | Self::QuestionImage {
                question_revision_tuple,
                ..
            }
            | Self::RestrictedQuestionImage {
                question_revision_tuple,
                ..
            }
            | Self::QuestionRender {
                question_revision_tuple,
                ..
            } => Some(question_revision_tuple),
            Self::WorkspaceImportSource { .. }
            | Self::DraftQuestionImage { .. }
            | Self::WorkspaceQuestionSource { .. }
            | Self::WorkspaceImportExtractedImage { .. }
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
            Self::QuestionImage { .. }
                | Self::RestrictedQuestionImage { .. }
                | Self::QuestionRender { .. }
                | Self::CourseBannerRendition { .. }
                | Self::ProfileImage { .. }
                | Self::StudentRecord { .. }
        )
    }

    /// Chooses the physical immutable Question image domain from the publication's
    /// immutable visibility.  Call this at publication time rather than
    /// reconstructing a key later from an untrusted route or browser value.
    pub fn published_question_image(
        question_revision_tuple: QuestionRevisionTuple,
        question_image_asset_id: QuestionImageAssetId,
        object_id: ObjectId,
    ) -> Self {
        Self::RestrictedQuestionImage {
            question_revision_tuple,
            question_image_asset_id,
            object_id,
        }
    }
}

/// Derives the immutable physical identity for one Course Banner Upload.
pub fn course_banner_upload_object_id(
    course_instance_id: &CourseInstanceId,
    course_banner_upload_id: CourseBannerUploadId,
) -> ObjectId {
    domain_separated_object_id_from_parts(
        b"ple:course-banner-upload:v3\0",
        course_instance_id.as_str().as_bytes(),
        course_banner_upload_id.as_uuid().as_bytes(),
        uuid::Uuid::nil().as_bytes(),
    )
}

/// Derives the immutable physical identity for one promoted course banner.
pub fn course_banner_source_object_id(
    course_instance_id: &CourseInstanceId,
    course_banner_id: CourseBannerId,
) -> ObjectId {
    domain_separated_object_id_from_parts(
        b"ple:course-banner-source:v3\0",
        course_instance_id.as_str().as_bytes(),
        course_banner_id.as_uuid().as_bytes(),
        uuid::Uuid::nil().as_bytes(),
    )
}

/// Derives the immutable physical identity for one normalized course-banner rendition.
pub fn course_banner_rendition_object_id(
    course_instance_id: &CourseInstanceId,
    course_banner_id: CourseBannerId,
    rendition: CourseBannerRendition,
) -> ObjectId {
    let rendition_uuid = match rendition {
        CourseBannerRendition::Banner => uuid::Uuid::from_u128(1),
    };
    domain_separated_object_id_from_parts(
        // `v3` hashes the Course public ID. Pre-production has no durable
        // object-store rows to preserve.
        b"ple:course-banner-rendition:v3\0",
        course_instance_id.as_str().as_bytes(),
        course_banner_id.as_uuid().as_bytes(),
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
    workspace_id: WorkspaceId,
    workspace_import_id: WorkspaceImportId,
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:workspace-qti-archive:v1\0");
    hasher.update(workspace_id.as_uuid().as_bytes());
    hasher.update(workspace_import_id.as_uuid().as_bytes());

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
    question_revision_tuple: &QuestionRevisionTuple,
    workspace_import_id: WorkspaceImportId,
    archive_sha256: Sha256Checksum,
) -> ObjectId {
    let mut hasher = Sha256::new();
    hasher.update(b"ple:published-import-archive:v2\0");
    hasher.update(question_revision_tuple.question_id.as_str().as_bytes());
    hasher.update(question_revision_tuple.revision_number.get().to_be_bytes());
    hasher.update(workspace_import_id.as_uuid().as_bytes());
    hasher.update(archive_sha256.as_bytes());

    let digest = hasher.finalize();
    let mut object_uuid = [0_u8; 16];
    object_uuid.copy_from_slice(&digest[..16]);
    ObjectId::from_uuid(uuid::Uuid::from_bytes(object_uuid))
}

#[cfg(test)]
#[path = "bucket_tests.rs"]
mod tests;
