//! [`QuestionDefinition`], the backend-neutral representation every engine
//! maps into (WP-C1).
//!
//! One shared shape is what lets a WeBWorK question, a QTI item, an H5P
//! activity, and a first-party algorithmic question flow through the same
//! attempt loop, gradebook, and export path. Each backend adapter translates
//! into this type, and everything downstream reads only this type.
//!
//! The definition describes a question. It carries no answer key and no
//! grading material: [`GradingDefinition`] states *how* a response is judged,
//! while the values it is judged against live in `crates/grading`, server-side.

use serde::{Deserialize, Serialize};

use crate::assignment_activity_rules::{AttemptPolicy, TimingPolicy};
use crate::envelope::ContentBlock;
use crate::generation::RandomizationDefinition;
use crate::identity::{ObjectId, WorkspaceId, WorkspaceImportId};
use crate::response::{QuestionResponseFormat, QuestionType};
use crate::taxonomy::{License, Tag, TaxonomyTerm};
use crate::{QuestionId, QuestionVersionNumber};

/// Maximum Unicode scalar values permitted in a student-facing question title.
///
/// Browser decoders use `Array.from(title).length`, which has the same count
/// for valid UTF-8 JSON strings. Titles are never silently trimmed or
/// normalized at either boundary.
pub const MAX_QUESTION_TITLE_UNICODE_SCALARS: usize = 512;

/// The authored or imported representation of a Question.
///
/// Question Format describes source representation and interchange. It is
/// independent of the educational Question Type, the server-side Question
/// Backend, and any Question Generator used to make a variation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestionFormat {
    /// Version 2 of PLE's canonical static flat-question JSON.
    PleFlatQuestionV2,
    /// A first-party generated Question authored as a native implementation.
    NativeAlgorithmic,
    /// A WeBWorK PG source.
    WebworkPg,
    /// An imported QTI item.
    Qti,
    /// An imported H5P activity.
    H5p,
    /// An archived iMathAS source snapshot.
    Imathas,
}

/// Why a student-facing question title cannot be safely delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionTitleError {
    /// The trimmed title contains no visible content.
    Blank,
    /// The title exceeds [`MAX_QUESTION_TITLE_UNICODE_SCALARS`].
    TooLong,
}

impl std::fmt::Display for QuestionTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blank => formatter.write_str("question title must not be blank"),
            Self::TooLong => write!(
                formatter,
                "question title must contain at most {MAX_QUESTION_TITLE_UNICODE_SCALARS} Unicode scalar values"
            ),
        }
    }
}

impl std::error::Error for QuestionTitleError {}

/// Validates the durable student-facing title without mutating it.
///
/// Whitespace-only titles are refused after trimming for the emptiness check;
/// leading and trailing whitespace are otherwise preserved exactly rather than
/// being silently changed before storage or delivery.
pub fn validate_question_title(title: &str) -> Result<(), QuestionTitleError> {
    if title.trim().is_empty() {
        return Err(QuestionTitleError::Blank);
    }
    if title.chars().count() > MAX_QUESTION_TITLE_UNICODE_SCALARS {
        return Err(QuestionTitleError::TooLong);
    }
    Ok(())
}

/// Which engine a question came from, and how to find it there.
///
/// The reference stays with the question so an import can be repeated and an
/// export can point back at the original. Each variant carries exactly what its
/// backend needs to locate the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "backend",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum QuestionSource {
    /// A first-party native question.
    Native,
    /// A WeBWorK PG problem, rendered by the renderer service.
    Webwork {
        /// Path within the problem library, for example an OPL path.
        pg_path: String,
    },
    /// An item from an imported QTI package.
    Qti {
        /// Item identifier within the package.
        item_id: String,
        /// Immutable object containing the original published package bytes.
        package_object: ObjectId,
        /// SHA-256 of the original published package bytes.
        package_sha256: String,
    },
    /// An imported H5P activity, which evaluates in the browser.
    H5p {
        /// H5P content type, for example `H5P.MultiChoice`.
        content_type: String,
    },
    /// An iMathAS item frozen from a server-fetched source snapshot.
    ///
    /// The provider name is an opaque deployment-configured key. The snapshot
    /// and integration profile make a published version replayable without
    /// serializing endpoints, credentials, launch material, or answer data.
    Imathas {
        /// Opaque configured provider key, never a URL or credential.
        provider: String,
        /// Provider-local item reference captured in the immutable snapshot.
        item_ref: String,
        /// Secure object containing the immutable, checksum-verified source.
        snapshot: ObjectId,
        /// SHA-256 of the snapshot bytes in lowercase hexadecimal.
        snapshot_sha256: String,
        /// Supported integration profile pinned at publication time.
        integration_profile: String,
    },
}

/// Why a published source locator cannot safely name immutable source bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionSourceValidationError {
    /// A source checksum was not lowercase, fixed-width hexadecimal SHA-256.
    NonCanonicalSha256,
}

impl std::fmt::Display for QuestionSourceValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonCanonicalSha256 => {
                formatter.write_str("source SHA-256 must be 64 lowercase hexadecimal characters")
            }
        }
    }
}

impl std::error::Error for QuestionSourceValidationError {}

impl QuestionSource {
    /// Validates source fields whose string representation is part of the
    /// immutable published contract.
    pub fn validate(&self) -> Result<(), QuestionSourceValidationError> {
        let checksum = match self {
            Self::Qti { package_sha256, .. } => package_sha256,
            Self::Imathas {
                snapshot_sha256, ..
            } => snapshot_sha256,
            Self::Native | Self::Webwork { .. } | Self::H5p { .. } => return Ok(()),
        };
        if checksum.len() == 64
            && checksum
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            Ok(())
        } else {
            Err(QuestionSourceValidationError::NonCanonicalSha256)
        }
    }
}

/// Source locator permitted while content is still a private workspace draft.
///
/// Unlike [`QuestionSource::Imathas`], its iMathAS variant intentionally has
/// no snapshot or profile: those are fetched and frozen by the server before
/// publication can mint a durable version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "backend",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DraftQuestionSource {
    /// A first-party native question.
    Native,
    /// A WeBWorK PG problem.
    Webwork { pg_path: String },
    /// An imported QTI item staged in this draft's private workspace.
    ///
    /// The import record, rather than the browser, resolves private archive,
    /// asset, and grading material. It cannot be used as a published locator.
    Qti {
        item_id: String,
        import_id: WorkspaceImportId,
    },
    /// An H5P activity.
    H5p { content_type: String },
    /// Private iMathAS sandbox locator, never an endpoint or credential.
    Imathas { provider: String, item_ref: String },
}

impl TryFrom<DraftQuestionSource> for QuestionSource {
    type Error = DraftSourcePublicationError;

    fn try_from(source: DraftQuestionSource) -> Result<Self, Self::Error> {
        match source {
            DraftQuestionSource::Native => Ok(Self::Native),
            DraftQuestionSource::Webwork { pg_path } => Ok(Self::Webwork { pg_path }),
            DraftQuestionSource::Qti { .. } => Err(DraftSourcePublicationError::QtiImportRequired),
            DraftQuestionSource::H5p { content_type } => Ok(Self::H5p { content_type }),
            DraftQuestionSource::Imathas { .. } => {
                Err(DraftSourcePublicationError::SnapshotRequired)
            }
        }
    }
}

/// Why a draft source cannot become a published immutable source automatically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftSourcePublicationError {
    /// iMathAS requires the server to archive a source snapshot and pin its profile.
    SnapshotRequired,
    /// QTI publication must resolve an authorized staged import into an exact
    /// immutable package object and checksum before identifiers are minted.
    QtiImportRequired,
}

/// How a response is judged, without stating what the answer is.
///
/// Safe to send to a browser: a student can see that partial credit applies and
/// how a question is weighted, and still learn nothing about the answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GradingDefinition {
    /// Correct or incorrect, with no middle ground.
    AllOrNothing {
        /// Points awarded for a correct response.
        points: f64,
    },
    /// Credit proportional to how much of the response is correct.
    ///
    /// Requires the `partialCredit` capability; capability validation refuses
    /// an assignment whose backend lacks it, before publication.
    PartialCredit {
        /// Points awarded for a fully correct response.
        points: f64,
    },
    /// Practice with no recorded score.
    ///
    /// The honest declaration for an H5P activity, which evaluates in the
    /// browser and therefore cannot carry a graded assignment.
    Ungraded,
}

/// Descriptive information used for search, attribution, and reuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionMetadata {
    /// Title shown in the catalog and in printed exams.
    pub title: String,
    /// Free-form labels for search.
    pub tags: Vec<Tag>,
    /// Controlled-vocabulary terms that survive export.
    pub taxonomy: Vec<TaxonomyTerm>,
    /// Terms under which the content may be reused.
    pub license: License,
    /// BCP 47 language tag for the prompt, for example `en-US`.
    pub language: String,
}

impl QuestionMetadata {
    /// Validates the one piece of metadata delivered in every student envelope.
    pub fn validate_title(&self) -> Result<(), QuestionTitleError> {
        validate_question_title(&self.title)
    }
}

/// Editable workspace content before publication.
///
/// A draft deliberately has neither a [`QuestionId`] nor a [`QuestionVersionNumber`].
/// Publication is the one boundary that mints both durable identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftQuestionDefinition {
    /// The workspace that authored it.
    pub workspace: WorkspaceId,
    /// Which engine it came from.
    pub source: DraftQuestionSource,
    /// The authored or imported representation of this Question.
    pub question_format: QuestionFormat,
    /// The prompt, in render order.
    pub prompt: Vec<ContentBlock>,
    /// The shape of response expected.
    pub response: QuestionResponseFormat,
    /// The educational interaction this Question assesses.
    pub question_type: QuestionType,
    /// How many attempts, and when feedback appears.
    pub attempt_policy: AttemptPolicy,
    /// Time limits, if any.
    pub timing_policy: TimingPolicy,
    /// How content varies between students and runs.
    pub randomization: RandomizationDefinition,
    /// How a response is judged.
    pub grading: GradingDefinition,
    /// Title, tags, taxonomy, license, language.
    pub metadata: QuestionMetadata,
}

/// Compact browser-safe identity for one private workspace draft.
///
/// The full source locator, prompt, Question Response Format, grading policy, and
/// asset references remain on the detail record.  In particular, this list
/// projection deliberately has no published Question ID or Question Version Number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDraftSummary {
    /// Private workspace identity used to retrieve the editable draft.
    pub workspace: WorkspaceId,
    /// Stable typed locator used in application navigation.
    pub reference: crate::AuthoringWorkspaceReference,
    /// Human-facing draft title.
    pub title: String,
    /// Question Backend without its private source locator.
    pub source_backend: crate::question_library::QuestionBackend,
}

impl DraftQuestionDefinition {
    /// Builds the intentionally compact list projection for this draft.
    pub fn workspace_summary(&self, reference: crate::AuthoringWorkspaceReference) -> WorkspaceDraftSummary {
        WorkspaceDraftSummary {
            workspace: self.workspace,
            reference,
            title: self.metadata.title.clone(),
            source_backend: crate::question_library::QuestionBackend::from(&self.source),
        }
    }
}

/// Immutable published question content.
///
/// Every assignment, attempt, envelope, cache key, and grading operation uses
/// this type, so a draft can never enter a published-only path by omission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionDefinition {
    /// Stable Question lineage established at publication.
    pub question_id: QuestionId,
    /// Exact immutable version within the Question lineage.
    pub version_number: QuestionVersionNumber,
    /// The workspace that authored it.
    pub workspace: WorkspaceId,
    /// Which engine it came from.
    pub source: QuestionSource,
    /// The authored or imported representation of this Question.
    pub question_format: QuestionFormat,
    /// The prompt, in render order.
    pub prompt: Vec<ContentBlock>,
    /// The shape of response expected.
    pub response: QuestionResponseFormat,
    /// The educational interaction this Question assesses.
    pub question_type: QuestionType,
    /// How many attempts, and when feedback appears.
    pub attempt_policy: AttemptPolicy,
    /// Time limits, if any.
    pub timing_policy: TimingPolicy,
    /// How content varies between students and runs.
    pub randomization: RandomizationDefinition,
    /// How a response is judged.
    pub grading: GradingDefinition,
    /// Title, tags, taxonomy, license, language.
    pub metadata: QuestionMetadata,
}

impl QuestionDefinition {
    /// Attaches the IDs minted at successful publication to draft content.
    pub fn from_draft(
        draft: DraftQuestionDefinition,
        question_id: QuestionId,
        version_number: QuestionVersionNumber,
        source: QuestionSource,
    ) -> Self {
        Self {
            question_id,
            version_number,
            workspace: draft.workspace,
            source,
            question_format: draft.question_format,
            prompt: draft.prompt,
            response: draft.response,
            question_type: draft.question_type,
            attempt_policy: draft.attempt_policy,
            timing_policy: draft.timing_policy,
            randomization: draft.randomization,
            grading: draft.grading,
            metadata: draft.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::NumericTolerance;
    use crate::generation::RandomizationDefinition;
    use uuid::Uuid;

    fn sample_draft() -> DraftQuestionDefinition {
        DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(3)),
            source: DraftQuestionSource::Native,
            question_format: QuestionFormat::NativeAlgorithmic,
            prompt: vec![ContentBlock::Text {
                markdown: "What is the molar mass?".to_string(),
            }],
            response: QuestionResponseFormat::Numeric {
                tolerance: NumericTolerance::Relative { fraction: 0.01 },
                unit: Some("g/mol".to_string()),
            },
            question_type: QuestionType::Numeric,
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Molar mass".to_string(),
                tags: vec![Tag::new("stoichiometry")],
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        }
    }

    #[test]
    fn a_draft_serializes_without_published_identifiers() {
        let json = serde_json::to_value(sample_draft()).expect("draft serializes");
        assert!(json.get("questionId").is_none());
        assert!(json.get("versionNumber").is_none());
    }

    #[test]
    fn a_published_question_carries_its_question_version_reference() {
        let published = QuestionDefinition::from_draft(
            sample_draft(),
            "123-4567".parse().expect("valid Question ID"),
            QuestionVersionNumber::new(1).expect("positive version"),
            QuestionSource::Native,
        );
        assert_eq!(published.question_id.to_string(), "123-4567");
    }

    #[test]
    fn a_question_survives_a_json_round_trip() {
        let question = sample_draft();
        let json = serde_json::to_string(&question).expect("serialization should succeed");
        let restored: DraftQuestionDefinition =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(restored, question);
    }

    #[test]
    fn imathas_sandbox_source_cannot_become_published_without_a_snapshot() {
        assert_eq!(
            QuestionSource::try_from(DraftQuestionSource::Imathas {
                provider: "myopenmath".to_string(),
                item_ref: "12345".to_string(),
            }),
            Err(DraftSourcePublicationError::SnapshotRequired)
        );
    }

    #[test]
    fn qti_draft_uses_only_an_opaque_workspace_import_and_requires_preparation() {
        let import_id = WorkspaceImportId::from_uuid(Uuid::from_u128(44));
        let draft = DraftQuestionSource::Qti {
            item_id: "choice-1".to_string(),
            import_id,
        };
        let json = serde_json::to_value(&draft).expect("draft serializes");
        assert_eq!(json["backend"], "qti");
        assert_eq!(json["importId"], import_id.to_string());
        assert!(json.get("packageObject").is_none());
        assert_eq!(
            QuestionSource::try_from(draft),
            Err(DraftSourcePublicationError::QtiImportRequired)
        );
    }

    #[test]
    fn published_qti_source_binds_exact_object_and_checksum() {
        let source = QuestionSource::Qti {
            item_id: "choice-1".to_string(),
            package_object: ObjectId::from_uuid(Uuid::from_u128(45)),
            package_sha256: "a".repeat(64),
        };
        let json = serde_json::to_string(&source).expect("source serializes");
        let restored: QuestionSource = serde_json::from_str(&json).expect("source round trips");
        assert_eq!(restored, source);
        assert!(serde_json::from_str::<QuestionSource>(
            r#"{\"backend\":\"qti\",\"itemId\":\"choice-1\",\"packageObject\":\"00000000-0000-0000-0000-00000000002d\",\"packageSha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"archive\":\"leak\"}"#
        )
        .is_err());
    }

    #[test]
    fn source_checksums_are_canonical_lowercase_sha256() {
        let valid = QuestionSource::Qti {
            item_id: "choice-1".to_string(),
            package_object: ObjectId::from_uuid(Uuid::from_u128(45)),
            package_sha256: "a".repeat(64),
        };
        assert_eq!(valid.validate(), Ok(()));

        for invalid_checksum in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
            let invalid = QuestionSource::Qti {
                item_id: "choice-1".to_string(),
                package_object: ObjectId::from_uuid(Uuid::from_u128(45)),
                package_sha256: invalid_checksum,
            };
            assert_eq!(
                invalid.validate(),
                Err(QuestionSourceValidationError::NonCanonicalSha256)
            );
        }
    }

    #[test]
    fn learner_title_policy_uses_trimmed_content_and_unicode_scalar_count() {
        assert_eq!(
            validate_question_title(" \t\n "),
            Err(QuestionTitleError::Blank)
        );
        assert!(validate_question_title("A title").is_ok());
        assert!(
            validate_question_title(&"\u{1F9EC}".repeat(MAX_QUESTION_TITLE_UNICODE_SCALARS))
                .is_ok()
        );
        assert_eq!(
            validate_question_title(&"\u{1F9EC}".repeat(MAX_QUESTION_TITLE_UNICODE_SCALARS + 1)),
            Err(QuestionTitleError::TooLong)
        );
    }
}
