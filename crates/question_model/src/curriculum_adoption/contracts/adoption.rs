//! Closed BlueprintCourse adoption operations.

use serde::{Deserialize, Serialize};

use super::{
    BlueprintAssignmentRevisionReference, BlueprintOperationRetryToken, BlueprintRevisionReference,
    CourseInstanceCreationReservation, CourseInstanceScheduleCorrection, CourseInstanceSnapshot,
    CourseOrigin, QuestionRevisionSubstitutions, UnavailableQuestionRevisionRecovery,
};
use crate::{
    AccountId, ActivityTimestamp, AssignmentReference, BlueprintCourseReference, BlueprintRevision,
    CourseInstanceReference, CourseTerm,
};

/// One server-reserved BlueprintCourse creation bound to an authenticated Instructor operation.
///
/// This value intentionally has no Serde implementation. The application service creates and
/// retains it beside the server-held preview record; browser JSON remains intent-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintForkReservation {
    source: BlueprintRevisionReference,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
    reserved_blueprint: BlueprintCourseReference,
}

impl BlueprintForkReservation {
    /// Reserves a BlueprintCourse identity after the server has authorized the fork intent.
    pub fn new(
        source: BlueprintRevisionReference,
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: BlueprintOperationRetryToken,
        reserved_blueprint: BlueprintCourseReference,
    ) -> Self {
        Self {
            source,
            authorized_account,
            request_digest,
            idempotency_key,
            reserved_blueprint,
        }
    }

    /// Returns the exact readable source that authorized the fork reservation.
    pub fn source(&self) -> &BlueprintRevisionReference {
        &self.source
    }

    /// Returns the authenticated account whose current authority is revalidated at apply.
    pub fn authorized_account(&self) -> AccountId {
        self.authorized_account
    }

    /// Returns the canonical request binding used for idempotent receipt persistence.
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }

    /// Returns the browser retry key bound to this server-created reservation.
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
    }

    /// Returns the server-reserved identity that the successful transaction materializes.
    pub fn reserved_blueprint(&self) -> BlueprintCourseReference {
        self.reserved_blueprint
    }
}

/// Closed server decision that allows a Blueprint Course fork to proceed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ForkBlueprintCourseReadiness {
    Ready,
    Blocked { blocker: ForkBlueprintCourseBlocker },
}

/// Typed server blocker for a Blueprint Course fork preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ForkBlueprintCourseBlocker {
    UnavailableQuestionRevision {
        recovery: UnavailableQuestionRevisionRecovery,
    },
    SourceRevisionDrift {
        observed: BlueprintRevisionReference,
    },
}

/// Closed server decision that allows Blueprint Assignment adoption to proceed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdoptBlueprintAssignmentReadiness {
    Ready,
    Blocked {
        blocker: AdoptBlueprintAssignmentBlocker,
    },
}

/// Typed server blocker for one Blueprint Assignment adoption preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AdoptBlueprintAssignmentBlocker {
    ScheduleCorrectionsRequired {
        #[serde(deserialize_with = "deserialize_schedule_corrections")]
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailableQuestionRevision {
        recovery: UnavailableQuestionRevisionRecovery,
    },
    SourceRevisionDrift {
        observed: BlueprintRevisionReference,
    },
    DestinationWitnessDrift {
        expected: CourseInstanceSnapshot,
        observed: CourseInstanceSnapshot,
    },
}

/// Closed server decision that allows Blueprint Course instantiation to proceed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstantiateBlueprintCourseReadiness {
    Ready,
    Blocked {
        blocker: InstantiateBlueprintCourseBlocker,
    },
}

/// Typed server blocker for one Blueprint Course instantiation preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum InstantiateBlueprintCourseBlocker {
    ScheduleCorrectionsRequired {
        #[serde(deserialize_with = "deserialize_schedule_corrections")]
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailableQuestionRevision {
        recovery: UnavailableQuestionRevisionRecovery,
    },
    SourceRevisionDrift {
        observed: BlueprintRevisionReference,
    },
}

/// Browser preview request for an independent BlueprintCourse fork.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ForkBlueprintCoursePreviewRequest {
    /// The exact readable source revision selected by the browser.
    pub source: BlueprintRevisionReference,
    /// Explicit Question Revision substitutions selected during preview correction.
    pub replacements: QuestionRevisionSubstitutions,
}

/// Browser preview request for one bounded BlueprintCourse assignment adoption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AdoptBlueprintAssignmentPreviewRequest {
    /// One bounded assignment location in the selected source revision.
    pub source: BlueprintAssignmentRevisionReference,
    /// Existing CourseInstance destination.
    pub course: CourseInstanceReference,
    /// Explicit Question Revision substitutions selected during preview correction.
    pub replacements: QuestionRevisionSubstitutions,
}

/// Browser preview request for a whole BlueprintCourse instantiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateBlueprintCoursePreviewRequest {
    /// The exact readable source revision selected by the browser.
    pub source: BlueprintRevisionReference,
    /// Destination term whose local calendar resolves source schedule intent.
    pub target_term: CourseTerm,
    /// Explicit QuestionId substitutions selected during preview correction.
    pub replacements: QuestionRevisionSubstitutions,
}

/// Answer-free result used to create a fork command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ForkBlueprintCoursePreviewView {
    /// Exact source observed by the authorized server read.
    pub source: BlueprintRevisionReference,
    /// Server-validated substitutions.
    pub replacements: QuestionRevisionSubstitutions,
    /// Server-owned authorization to construct the fork command.
    pub readiness: ForkBlueprintCourseReadiness,
}

/// Answer-free result used to create an assignment-adoption command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AdoptBlueprintAssignmentPreviewView {
    /// Exact source location observed by the authorized server read.
    pub source: BlueprintAssignmentRevisionReference,
    /// Exact existing CourseInstance state observed by the authorized server read.
    pub destination: CourseInstanceSnapshot,
    /// Server-validated substitutions.
    pub replacements: QuestionRevisionSubstitutions,
    /// Server-owned authorization to construct the ordinary adoption command.
    pub readiness: AdoptBlueprintAssignmentReadiness,
}

/// Answer-free result used to create a course-instantiation command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateBlueprintCoursePreviewView {
    /// Exact source observed by the authorized server read.
    pub source: BlueprintRevisionReference,
    /// Destination term returned by preview.
    pub target_term: CourseTerm,
    /// Server-validated substitutions.
    pub replacements: QuestionRevisionSubstitutions,
    /// Server-owned authorization to construct the instantiation command.
    pub readiness: InstantiateBlueprintCourseReadiness,
}

/// Apply command derived only from a completed fork preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBlueprintCourseCommand {
    source: BlueprintRevisionReference,
    replacements: QuestionRevisionSubstitutions,
    creation: BlueprintForkReservation,
    idempotency_key: BlueprintOperationRetryToken,
}

/// Apply command derived only from a completed assignment-adoption preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptBlueprintAssignmentCommand {
    source: BlueprintAssignmentRevisionReference,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    replacements: QuestionRevisionSubstitutions,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

/// Apply command derived only from a completed course-instantiation preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstantiateBlueprintCourseCommand {
    source: BlueprintRevisionReference,
    target_term: CourseTerm,
    replacements: QuestionRevisionSubstitutions,
    creation: CourseInstanceCreationReservation,
    idempotency_key: BlueprintOperationRetryToken,
}

impl AdoptBlueprintAssignmentCommand {
    /// Consumes one server-held record; a browser preview cannot construct this command.
    pub fn from_server_record(record: super::AdoptBlueprintAssignmentApplyRecord) -> Self {
        Self {
            source: record.source(),
            destination: record.destination().clone(),
            course_origin: record.course_origin(),
            replacements: record.replacements().clone(),
            authorized_account: record.authorized_account(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
    }

    pub fn source(&self) -> &BlueprintAssignmentRevisionReference {
        &self.source
    }
    pub fn destination(&self) -> &CourseInstanceSnapshot {
        &self.destination
    }
    pub fn course_origin(&self) -> CourseOrigin {
        self.course_origin
    }
    pub fn replacements(&self) -> &QuestionRevisionSubstitutions {
        &self.replacements
    }
    pub fn authorized_account(&self) -> AccountId {
        self.authorized_account
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
    }

    /// Derives the sole destination course locator from its exact server witness.
    pub fn course(&self) -> CourseInstanceReference {
        self.destination.course
    }
}

impl ForkBlueprintCourseCommand {
    /// Consumes one server-held reservation; browser JSON is never apply authority.
    pub fn from_server_record(record: super::ForkBlueprintCourseApplyRecord) -> Self {
        let idempotency_key = record.creation().idempotency_key().clone();
        Self {
            source: *record.source(),
            replacements: record.replacements().clone(),
            creation: record.creation().clone(),
            idempotency_key,
        }
    }
    pub fn source(&self) -> &BlueprintRevisionReference {
        &self.source
    }
    pub fn replacements(&self) -> &QuestionRevisionSubstitutions {
        &self.replacements
    }
    pub fn creation(&self) -> &BlueprintForkReservation {
        &self.creation
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
    }
}

impl InstantiateBlueprintCourseCommand {
    /// Consumes one server-held creation record; browser JSON is never apply authority.
    pub fn from_server_record(record: super::InstantiateBlueprintCourseApplyRecord) -> Self {
        let idempotency_key = record.creation().idempotency_key().clone();
        Self {
            source: *record.source(),
            target_term: record.target_term().clone(),
            replacements: record.replacements().clone(),
            creation: record.creation().clone(),
            idempotency_key,
        }
    }
    pub fn source(&self) -> &BlueprintRevisionReference {
        &self.source
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn replacements(&self) -> &QuestionRevisionSubstitutions {
        &self.replacements
    }

    pub fn creation(&self) -> &CourseInstanceCreationReservation {
        &self.creation
    }

    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
    }
}

/// A Blueprint Course fork preview cannot construct an apply command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForkBlueprintCourseCommandError {
    Blocked(ForkBlueprintCourseBlocker),
    CreationReservationMismatch,
}

/// A Blueprint Assignment adoption preview cannot construct an apply command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptBlueprintAssignmentCommandError {
    Blocked(AdoptBlueprintAssignmentBlocker),
}

/// A Blueprint Course instantiation preview cannot construct an apply command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstantiateBlueprintCourseCommandError {
    Blocked(InstantiateBlueprintCourseBlocker),
    CreationReservationMismatch,
}

impl ForkBlueprintCourseReadiness {
    pub(super) fn require_ready(&self) -> Result<(), ForkBlueprintCourseCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { blocker } => {
                Err(ForkBlueprintCourseCommandError::Blocked(blocker.clone()))
            }
        }
    }
}

impl AdoptBlueprintAssignmentReadiness {
    pub(super) fn require_ready(&self) -> Result<(), AdoptBlueprintAssignmentCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { blocker } => Err(AdoptBlueprintAssignmentCommandError::Blocked(
                blocker.clone(),
            )),
        }
    }
}

impl InstantiateBlueprintCourseReadiness {
    pub(super) fn require_ready(&self) -> Result<(), InstantiateBlueprintCourseCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { blocker } => Err(InstantiateBlueprintCourseCommandError::Blocked(
                blocker.clone(),
            )),
        }
    }
}

/// Immutable receipt retained for one successful BlueprintCourse fork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBlueprintCourseReceipt {
    source: BlueprintRevisionReference,
    created: BlueprintRevisionReference,
    creation: BlueprintForkReservation,
    server_time: ActivityTimestamp,
}

/// A fork receipt did not bind the reserved identity or its exact source lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForkBlueprintCourseReceiptError {
    SourceMismatch,
    CreatedBlueprintMismatch,
}

impl ForkBlueprintCourseReceipt {
    /// Records one committed fork with its source and server-reserved creation evidence.
    pub fn new(
        source: BlueprintRevisionReference,
        created: BlueprintRevisionReference,
        creation: BlueprintForkReservation,
        server_time: ActivityTimestamp,
    ) -> Result<Self, ForkBlueprintCourseReceiptError> {
        if creation.source() != &source {
            return Err(ForkBlueprintCourseReceiptError::SourceMismatch);
        }
        if creation.reserved_blueprint() != created.reference {
            return Err(ForkBlueprintCourseReceiptError::CreatedBlueprintMismatch);
        }
        Ok(Self {
            source,
            created,
            creation,
            server_time,
        })
    }

    pub fn source(&self) -> &BlueprintRevisionReference {
        &self.source
    }
    pub fn created(&self) -> &BlueprintRevisionReference {
        &self.created
    }
    pub fn creation(&self) -> &BlueprintForkReservation {
        &self.creation
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        self.creation.idempotency_key()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.creation.request_digest()
    }
    pub fn server_time(&self) -> ActivityTimestamp {
        self.server_time
    }
}

impl std::fmt::Display for ForkBlueprintCourseReceiptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "fork receipt creation evidence does not match the committed BlueprintCourse",
        )
    }
}

impl std::error::Error for ForkBlueprintCourseReceiptError {}

fn deserialize_schedule_corrections<'de, D>(
    deserializer: D,
) -> Result<Vec<CourseInstanceScheduleCorrection>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    super::bounded::deserialize_bounded_vec::<D, _, { crate::MAX_ASSIGNMENT_ORDERED_ENTRIES }>(
        deserializer,
    )
}

impl std::fmt::Display for ForkBlueprintCourseCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Blueprint Course fork preview is not ready for apply")
    }
}
impl std::error::Error for ForkBlueprintCourseCommandError {}

impl std::fmt::Display for AdoptBlueprintAssignmentCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Blueprint Assignment adoption preview is not ready for apply")
    }
}
impl std::error::Error for AdoptBlueprintAssignmentCommandError {}

impl std::fmt::Display for InstantiateBlueprintCourseCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Blueprint Course instantiation preview is not ready for apply")
    }
}
impl std::error::Error for InstantiateBlueprintCourseCommandError {}

/// Browser-safe completed assignment-adoption result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AdoptBlueprintAssignmentCompleted {
    pub course: CourseInstanceReference,
    pub assignment: AssignmentReference,
}

/// Browser-safe completion for a committed fork; it projects the exact created revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ForkBlueprintCourseCompleted {
    pub blueprint: BlueprintCourseReference,
    pub revision: BlueprintRevision,
}

/// Browser-safe completed whole-course instantiation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateBlueprintCourseCompleted {
    pub course: CourseInstanceReference,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AccountId, BlueprintAssignmentId, BlueprintCourseReference, BlueprintRevision,
        CourseScheduleRevisionNumber, CourseScheduleRevisionReference,
        CurriculumAdoptionRequestBinding, QuestionId, QuestionRevisionNumber,
        QuestionRevisionReference,
    };
    use uuid::Uuid;

    fn source() -> BlueprintRevisionReference {
        BlueprintRevisionReference {
            reference: BlueprintCourseReference::new(7).expect("BP reference"),
            revision: BlueprintRevision::new(2).expect("revision"),
        }
    }

    fn assignment_id() -> BlueprintAssignmentId {
        BlueprintAssignmentId::from_uuid(Uuid::from_u128(9))
    }

    fn other_assignment_id() -> BlueprintAssignmentId {
        BlueprintAssignmentId::from_uuid(Uuid::from_u128(10))
    }

    fn course_origin() -> CourseOrigin {
        CourseOrigin::from_blueprint(source())
    }

    #[test]
    fn course_origin_distinguishes_direct_creation_from_rollover() {
        let source_course = CourseInstanceReference::new(9).expect("source course");

        assert_eq!(CourseOrigin::from_blueprint(source()).source_course, None);
        assert_eq!(
            CourseOrigin::from_rollover(source(), source_course).source_course,
            Some(source_course)
        );
    }

    fn request_binding(
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: BlueprintOperationRetryToken,
    ) -> CurriculumAdoptionRequestBinding {
        CurriculumAdoptionRequestBinding::new(authorized_account, request_digest, idempotency_key)
    }

    fn authorized_account() -> AccountId {
        AccountId::from_uuid(Uuid::from_u128(1))
    }

    fn schedule_revision(
        course: CourseInstanceReference,
        revision_number: u64,
    ) -> CourseScheduleRevisionReference {
        CourseScheduleRevisionReference::new(
            course,
            CourseScheduleRevisionNumber::new(revision_number).expect("positive revision"),
        )
    }

    fn destination() -> CourseInstanceSnapshot {
        let course = CourseInstanceReference::new(3).expect("course");
        CourseInstanceSnapshot::new(course, schedule_revision(course, 1), vec![])
            .expect("bounded witness")
    }

    fn unavailable_recovery() -> UnavailableQuestionRevisionRecovery {
        UnavailableQuestionRevisionRecovery {
            source: BlueprintAssignmentRevisionReference::new(source(), assignment_id()),
            position: super::super::BlueprintQuestionPosition::new(None, 0, 0, None)
                .expect("position"),
            unavailable: QuestionRevisionReference {
                question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("question"),
                revision_number: QuestionRevisionNumber::new(1).expect("positive version"),
            },
            choices: super::super::ReplacementQuestionRevisionChoices::new(vec![
                QuestionRevisionReference {
                    question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("question"),
                    revision_number: QuestionRevisionNumber::new(2).expect("positive version"),
                },
            ])
            .expect("choices"),
        }
    }

    #[test]
    fn operations_are_closed_snake_case_and_preview_bound() {
        let key = BlueprintOperationRetryToken::parse("blueprint-apply").expect("key");
        let source = source();
        let fork = ForkBlueprintCoursePreviewView {
            source,
            replacements: QuestionRevisionSubstitutions::default(),
            readiness: ForkBlueprintCourseReadiness::Ready,
        };
        let command = ForkBlueprintCourseCommand::from_server_record(
            super::super::ForkBlueprintCourseApplyRecord::new(
                source,
                fork.replacements.clone(),
                BlueprintForkReservation::new(
                    source,
                    authorized_account(),
                    [4; 32],
                    key.clone(),
                    BlueprintCourseReference::new(8).expect("reserved blueprint"),
                ),
                fork.readiness.clone(),
            )
            .expect("server-held record"),
        );
        assert_eq!(command.source(), &source);
        assert_eq!(command.idempotency_key(), &key);
        let wire = serde_json::to_value(&fork).expect("preview serializes");
        assert!(wire.get("readiness").is_some());
        assert!(serde_json::from_value::<ForkBlueprintCoursePreviewView>(wire).is_ok());

        let location = BlueprintAssignmentRevisionReference::new(source, assignment_id());
        let adopt = AdoptBlueprintAssignmentPreviewView {
            source: location,
            destination: destination(),
            replacements: QuestionRevisionSubstitutions::default(),
            readiness: AdoptBlueprintAssignmentReadiness::Blocked {
                blocker: AdoptBlueprintAssignmentBlocker::ScheduleCorrectionsRequired {
                    corrections: vec![],
                },
            },
        };
        assert_eq!(
            super::super::AdoptBlueprintAssignmentApplyRecord::new(
                adopt.source,
                adopt.destination,
                course_origin(),
                adopt.replacements,
                request_binding(authorized_account(), [3; 32], key),
                adopt.readiness,
            ),
            Err(AdoptBlueprintAssignmentCommandError::Blocked(
                AdoptBlueprintAssignmentBlocker::ScheduleCorrectionsRequired {
                    corrections: vec![]
                }
            ))
        );
    }

    #[test]
    fn adoption_and_instantiation_bind_exact_source_location_and_idempotency() {
        let source = source();
        let location = BlueprintAssignmentRevisionReference::new(source, assignment_id());
        let key = BlueprintOperationRetryToken::parse("adopt-blueprint-1").expect("key");
        let adoption = AdoptBlueprintAssignmentPreviewView {
            source: location,
            destination: destination(),
            replacements: QuestionRevisionSubstitutions::default(),
            readiness: AdoptBlueprintAssignmentReadiness::Ready,
        };
        let command = AdoptBlueprintAssignmentCommand::from_server_record(
            super::super::AdoptBlueprintAssignmentApplyRecord::new(
                adoption.source,
                adoption.destination,
                course_origin(),
                adoption.replacements,
                request_binding(authorized_account(), [4; 32], key.clone()),
                adoption.readiness,
            )
            .expect("server-held record"),
        );
        assert_eq!(command.source().source(), source);
        assert_eq!(command.source().assignment_id(), assignment_id());
        assert_eq!(command.idempotency_key(), &key);
        assert_eq!(command.destination(), &destination());
        assert_eq!(
            command.course(),
            CourseInstanceReference::new(3).expect("course")
        );

        let term =
            CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
        let instantiation = InstantiateBlueprintCoursePreviewView {
            source,
            target_term: term.clone(),
            replacements: QuestionRevisionSubstitutions::default(),
            readiness: InstantiateBlueprintCourseReadiness::Ready,
        };
        let command = InstantiateBlueprintCourseCommand::from_server_record(
            super::super::InstantiateBlueprintCourseApplyRecord::new(
                source,
                term.clone(),
                instantiation.replacements,
                CourseInstanceCreationReservation::for_blueprint(
                    source,
                    term.clone(),
                    authorized_account(),
                    [5; 32],
                    key.clone(),
                    CourseInstanceReference::new(4).expect("reserved course"),
                ),
                instantiation.readiness,
            )
            .expect("server-held record"),
        );
        assert_eq!(command.source(), &source);
        assert_eq!(command.target_term(), &term);
        assert_eq!(
            command.creation().reserved_course(),
            CourseInstanceReference::new(4).expect("reserved course")
        );
    }

    #[test]
    fn previews_are_snake_case_and_refuse_unknown_source_fields() {
        let source = source();
        let preview = ForkBlueprintCoursePreviewView {
            source,
            replacements: QuestionRevisionSubstitutions::default(),
            readiness: ForkBlueprintCourseReadiness::Ready,
        };
        let wire = serde_json::to_value(preview).expect("preview serializes");
        assert!(wire.get("readiness").is_some());
        assert!(wire.get("corrections_required").is_none());
        let mut forged = wire;
        forged["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<ForkBlueprintCoursePreviewView>(forged).is_err());
        let source = BlueprintAssignmentRevisionReference::new(source, assignment_id());
        let mut source_wire = serde_json::to_value(source).expect("source serializes");
        source_wire["module_index"] = serde_json::json!(0);
        assert!(
            serde_json::from_value::<BlueprintAssignmentRevisionReference>(source_wire).is_err()
        );
        assert!(
            !BlueprintAssignmentRevisionReference::new(source.source(), other_assignment_id())
                .same_assignment_lineage(source)
        );

        let blocker =
            serde_json::to_value(ForkBlueprintCourseBlocker::UnavailableQuestionRevision {
                recovery: unavailable_recovery(),
            })
            .expect("blocker serializes");
        assert!(serde_json::from_value::<ForkBlueprintCourseBlocker>(blocker.clone()).is_ok());
        let schedule_blocker = serde_json::to_value(
            AdoptBlueprintAssignmentBlocker::ScheduleCorrectionsRequired {
                corrections: vec![],
            },
        )
        .expect("schedule blocker serializes");
        assert!(serde_json::from_value::<ForkBlueprintCourseBlocker>(schedule_blocker).is_err());
        let destination_blocker =
            serde_json::to_value(AdoptBlueprintAssignmentBlocker::DestinationWitnessDrift {
                expected: destination(),
                observed: destination(),
            })
            .expect("destination blocker serializes");
        assert!(
            serde_json::from_value::<InstantiateBlueprintCourseBlocker>(destination_blocker)
                .is_err()
        );
        let mut nested = blocker;
        nested["unavailable_question_revision"]["recovery"]["untrusted"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ForkBlueprintCourseBlocker>(nested).is_err());
    }

    #[test]
    fn operation_specific_blockers_prevent_their_own_command_construction() {
        let key = BlueprintOperationRetryToken::parse("refused-blueprint").expect("key");
        let fork_blocker = ForkBlueprintCourseBlocker::UnavailableQuestionRevision {
            recovery: unavailable_recovery(),
        };
        assert_eq!(
            super::super::ForkBlueprintCourseApplyRecord::new(
                source(),
                QuestionRevisionSubstitutions::default(),
                BlueprintForkReservation::new(
                    source(),
                    authorized_account(),
                    [9; 32],
                    key.clone(),
                    BlueprintCourseReference::new(9).expect("reserved blueprint"),
                ),
                ForkBlueprintCourseReadiness::Blocked {
                    blocker: fork_blocker.clone()
                },
            ),
            Err(ForkBlueprintCourseCommandError::Blocked(fork_blocker))
        );

        let adoption_blocker = AdoptBlueprintAssignmentBlocker::DestinationWitnessDrift {
            expected: destination(),
            observed: CourseInstanceSnapshot::new(
                CourseInstanceReference::new(3).expect("course"),
                schedule_revision(CourseInstanceReference::new(3).expect("course"), 2),
                vec![],
            )
            .expect("bounded witness"),
        };
        assert_eq!(
            super::super::AdoptBlueprintAssignmentApplyRecord::new(
                BlueprintAssignmentRevisionReference::new(source(), assignment_id()),
                destination(),
                course_origin(),
                QuestionRevisionSubstitutions::default(),
                request_binding(authorized_account(), [9; 32], key.clone()),
                AdoptBlueprintAssignmentReadiness::Blocked {
                    blocker: adoption_blocker.clone()
                },
            ),
            Err(AdoptBlueprintAssignmentCommandError::Blocked(
                adoption_blocker
            ))
        );

        let instantiation_blocker =
            InstantiateBlueprintCourseBlocker::ScheduleCorrectionsRequired {
                corrections: vec![CourseInstanceScheduleCorrection {
                    field: super::super::CourseInstanceScheduleField::DueAt,
                    reason: super::super::CourseInstanceScheduleReason::AmbiguousLocalTime,
                }],
            };
        let term =
            CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
        assert_eq!(
            super::super::InstantiateBlueprintCourseApplyRecord::new(
                source(),
                term.clone(),
                QuestionRevisionSubstitutions::default(),
                CourseInstanceCreationReservation::for_blueprint(
                    source(),
                    term,
                    authorized_account(),
                    [9; 32],
                    key,
                    CourseInstanceReference::new(9).expect("reserved course"),
                ),
                InstantiateBlueprintCourseReadiness::Blocked {
                    blocker: instantiation_blocker.clone()
                },
            ),
            Err(InstantiateBlueprintCourseCommandError::Blocked(
                instantiation_blocker
            ))
        );
    }

    #[test]
    fn creation_reservations_bind_commands_without_a_browser_serde_path() {
        let key = BlueprintOperationRetryToken::parse("creation-binding").expect("key");
        let fork = ForkBlueprintCoursePreviewView {
            source: source(),
            replacements: QuestionRevisionSubstitutions::default(),
            readiness: ForkBlueprintCourseReadiness::Ready,
        };
        let creation = BlueprintForkReservation::new(
            source(),
            authorized_account(),
            [6; 32],
            key.clone(),
            BlueprintCourseReference::new(10).expect("reserved blueprint"),
        );
        let command = ForkBlueprintCourseCommand::from_server_record(
            super::super::ForkBlueprintCourseApplyRecord::new(
                source(),
                fork.replacements.clone(),
                creation.clone(),
                fork.readiness.clone(),
            )
            .expect("server creation binding"),
        );
        assert_eq!(command.creation(), &creation);
        assert!(serde_json::to_value(&fork).is_ok());

        let mismatched = BlueprintForkReservation::new(
            BlueprintRevisionReference {
                reference: BlueprintCourseReference::new(11).expect("other source"),
                revision: BlueprintRevision::new(1).expect("revision"),
            },
            authorized_account(),
            [7; 32],
            key.clone(),
            BlueprintCourseReference::new(12).expect("reserved blueprint"),
        );
        assert_eq!(
            super::super::ForkBlueprintCourseApplyRecord::new(
                source(),
                fork.replacements.clone(),
                mismatched,
                fork.readiness.clone(),
            ),
            Err(ForkBlueprintCourseCommandError::CreationReservationMismatch)
        );

        let term =
            CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
        let instantiate = InstantiateBlueprintCoursePreviewView {
            source: source(),
            target_term: term.clone(),
            replacements: QuestionRevisionSubstitutions::default(),
            readiness: InstantiateBlueprintCourseReadiness::Ready,
        };
        let instance_key = BlueprintOperationRetryToken::parse("instance-binding").expect("key");
        let instance_creation = CourseInstanceCreationReservation::for_blueprint(
            source(),
            term.clone(),
            authorized_account(),
            [8; 32],
            instance_key.clone(),
            CourseInstanceReference::new(13).expect("reserved course"),
        );
        let instance_command = InstantiateBlueprintCourseCommand::from_server_record(
            super::super::InstantiateBlueprintCourseApplyRecord::new(
                source(),
                term,
                instantiate.replacements,
                instance_creation.clone(),
                instantiate.readiness,
            )
            .expect("server CourseInstance creation binding"),
        );
        assert_eq!(instance_command.creation(), &instance_creation);
    }

    #[test]
    fn server_record_authority_survives_preview_mutation_and_fork_receipt_binds_creation() {
        let key = BlueprintOperationRetryToken::parse("server-record").expect("key");
        let creation = BlueprintForkReservation::new(
            source(),
            authorized_account(),
            [3; 32],
            key,
            BlueprintCourseReference::new(14).expect("reserved BlueprintCourse"),
        );
        let record = super::super::ForkBlueprintCourseApplyRecord::new(
            source(),
            QuestionRevisionSubstitutions::default(),
            creation.clone(),
            ForkBlueprintCourseReadiness::Ready,
        )
        .expect("server record");
        let mut browser_preview = ForkBlueprintCoursePreviewView {
            source: source(),
            replacements: QuestionRevisionSubstitutions::default(),
            readiness: ForkBlueprintCourseReadiness::Ready,
        };
        browser_preview.source = BlueprintRevisionReference {
            reference: BlueprintCourseReference::new(15).expect("forged source"),
            revision: BlueprintRevision::new(1).expect("forged revision"),
        };
        let command = ForkBlueprintCourseCommand::from_server_record(record);
        assert_ne!(command.source(), &browser_preview.source);

        let created = BlueprintRevisionReference {
            reference: creation.reserved_blueprint(),
            revision: BlueprintRevision::new(1).expect("created revision"),
        };
        let receipt = ForkBlueprintCourseReceipt::new(
            source(),
            created,
            creation,
            ActivityTimestamp::from_unix_millis(1),
        )
        .expect("matching receipt");
        let completion = ForkBlueprintCourseCompleted {
            blueprint: receipt.created().reference,
            revision: receipt.created().revision,
        };
        assert_eq!(
            completion.blueprint,
            receipt.creation().reserved_blueprint()
        );
        let completion_wire = serde_json::to_value(completion).expect("completion serializes");
        assert!(completion_wire.get("replay").is_none());
    }
}
