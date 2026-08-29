//! Closed BlueprintCourse adoption operations.

use serde::{Deserialize, Serialize};

use super::{
    AssignmentDefinitionSourceView, CourseInstanceCreationWitness,
    CourseInstanceScheduleCorrection, CourseInstanceWitness, CurriculumAdoptionIdempotencyKey,
    CurriculumPinReplacements, ObservedBlueprintSource, UnavailableCurriculumPinRecovery,
};
use crate::{
    ActivityTimestamp, AssignmentReference, BlueprintReference, BlueprintRevision, CourseReference,
    CourseTerm, UserId,
};

/// One server-reserved BlueprintCourse creation bound to an authenticated Instructor operation.
///
/// This value intentionally has no Serde implementation. The application service creates and
/// retains it beside the server-held preview record; browser JSON remains intent-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintCourseCreationWitness {
    source: ObservedBlueprintSource,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
    reserved_blueprint: BlueprintReference,
}

impl BlueprintCourseCreationWitness {
    /// Reserves a BlueprintCourse identity after the server has authorized the fork intent.
    pub fn new(
        source: ObservedBlueprintSource,
        authorized_actor: UserId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
        reserved_blueprint: BlueprintReference,
    ) -> Self {
        Self {
            source,
            authorized_actor,
            request_digest,
            idempotency_key,
            reserved_blueprint,
        }
    }

    /// Returns the exact readable source that authorized the fork reservation.
    pub fn source(&self) -> &ObservedBlueprintSource {
        &self.source
    }

    /// Returns the authenticated actor whose current authority must be revalidated at apply.
    pub fn authorized_actor(&self) -> UserId {
        self.authorized_actor
    }

    /// Returns the canonical request binding used for idempotent receipt persistence.
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }

    /// Returns the browser retry key bound to this server-created reservation.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }

    /// Returns the server-reserved identity that the successful transaction materializes.
    pub fn reserved_blueprint(&self) -> BlueprintReference {
        self.reserved_blueprint
    }
}

/// Closed server decision that allows a Blueprint operation to proceed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAdoptionEligibility {
    Eligible,
    Refused { refusal: BlueprintAdoptionRefusal },
}

/// Typed server refusal for a Blueprint operation preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BlueprintAdoptionRefusal {
    ScheduleCorrectionsRequired {
        #[serde(deserialize_with = "deserialize_schedule_corrections")]
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailablePin {
        recovery: UnavailableCurriculumPinRecovery,
    },
    SourceRevisionDrift {
        observed: ObservedBlueprintSource,
    },
    DestinationWitnessDrift {
        expected: CourseInstanceWitness,
        observed: CourseInstanceWitness,
    },
}

/// Browser preview request for an independent BlueprintCourse fork.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ForkBlueprintCoursePreviewRequest {
    /// The exact readable source revision selected by the browser.
    pub source: ObservedBlueprintSource,
    /// Explicit QuestionId substitutions selected during preview correction.
    pub replacements: CurriculumPinReplacements,
}

/// Browser preview request for one bounded BlueprintCourse assignment adoption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AdoptBlueprintAssignmentPreviewRequest {
    /// One bounded assignment location in the selected source revision.
    pub source: AssignmentDefinitionSourceView,
    /// Existing CourseInstance destination.
    pub course: CourseReference,
    /// Explicit QuestionId substitutions selected during preview correction.
    pub replacements: CurriculumPinReplacements,
}

/// Browser preview request for a whole BlueprintCourse instantiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateBlueprintCoursePreviewRequest {
    /// The exact readable source revision selected by the browser.
    pub source: ObservedBlueprintSource,
    /// Destination term whose local calendar resolves source schedule intent.
    pub target_term: CourseTerm,
    /// Explicit QuestionId substitutions selected during preview correction.
    pub replacements: CurriculumPinReplacements,
}

/// Answer-free result used to create a fork command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ForkBlueprintCoursePreviewView {
    /// Exact source observed by the authorized server read.
    pub source: ObservedBlueprintSource,
    /// Server-validated substitutions.
    pub replacements: CurriculumPinReplacements,
    /// Server-owned authorization to construct the fork command.
    pub eligibility: BlueprintAdoptionEligibility,
}

/// Answer-free result used to create an assignment-adoption command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AdoptBlueprintAssignmentPreviewView {
    /// Exact source location observed by the authorized server read.
    pub source: AssignmentDefinitionSourceView,
    /// Exact existing CourseInstance state observed by the authorized server read.
    pub destination: CourseInstanceWitness,
    /// Server-validated substitutions.
    pub replacements: CurriculumPinReplacements,
    /// Server-owned authorization to construct the ordinary adoption command.
    pub eligibility: BlueprintAdoptionEligibility,
}

/// Answer-free result used to create a course-instantiation command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateBlueprintCoursePreviewView {
    /// Exact source observed by the authorized server read.
    pub source: ObservedBlueprintSource,
    /// Destination term returned by preview.
    pub target_term: CourseTerm,
    /// Server-validated substitutions.
    pub replacements: CurriculumPinReplacements,
    /// Server-owned authorization to construct the instantiation command.
    pub eligibility: BlueprintAdoptionEligibility,
}

/// Apply command derived only from a completed fork preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBlueprintCourseCommand {
    source: ObservedBlueprintSource,
    replacements: CurriculumPinReplacements,
    creation: BlueprintCourseCreationWitness,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Apply command derived only from a completed assignment-adoption preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptBlueprintAssignmentCommand {
    source: AssignmentDefinitionSourceView,
    destination: CourseInstanceWitness,
    replacements: CurriculumPinReplacements,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Apply command derived only from a completed course-instantiation preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstantiateBlueprintCourseCommand {
    source: ObservedBlueprintSource,
    target_term: CourseTerm,
    replacements: CurriculumPinReplacements,
    creation: CourseInstanceCreationWitness,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl AdoptBlueprintAssignmentCommand {
    /// Consumes one server-held record; a browser preview cannot construct this command.
    pub fn from_server_record(record: super::AdoptBlueprintAssignmentApplyRecord) -> Self {
        Self {
            source: record.source(),
            destination: record.destination().clone(),
            replacements: record.replacements().clone(),
            authorized_actor: record.authorized_actor(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
    }

    pub fn source(&self) -> &AssignmentDefinitionSourceView {
        &self.source
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }
    pub fn authorized_actor(&self) -> UserId {
        self.authorized_actor
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }

    /// Derives the sole destination course locator from its exact server witness.
    pub fn course(&self) -> CourseReference {
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

    pub fn source(&self) -> &ObservedBlueprintSource {
        &self.source
    }

    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }

    pub fn creation(&self) -> &BlueprintCourseCreationWitness {
        &self.creation
    }

    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
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

    pub fn source(&self) -> &ObservedBlueprintSource {
        &self.source
    }

    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }

    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }

    pub fn creation(&self) -> &CourseInstanceCreationWitness {
        &self.creation
    }

    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

/// A preview has unresolved schedule or exact-pin correction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurriculumAdoptionCommandError {
    Refused(BlueprintAdoptionRefusal),
    CreationWitnessMismatch,
}

pub(super) fn require_blueprint_eligible(
    eligibility: &BlueprintAdoptionEligibility,
) -> Result<(), CurriculumAdoptionCommandError> {
    match eligibility {
        BlueprintAdoptionEligibility::Eligible => Ok(()),
        BlueprintAdoptionEligibility::Refused { refusal } => {
            Err(CurriculumAdoptionCommandError::Refused(refusal.clone()))
        }
    }
}

/// Immutable receipt retained for one successful BlueprintCourse fork.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBlueprintCourseReceipt {
    source: ObservedBlueprintSource,
    created: ObservedBlueprintSource,
    creation: BlueprintCourseCreationWitness,
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
        source: ObservedBlueprintSource,
        created: ObservedBlueprintSource,
        creation: BlueprintCourseCreationWitness,
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

    pub fn source(&self) -> &ObservedBlueprintSource {
        &self.source
    }
    pub fn created(&self) -> &ObservedBlueprintSource {
        &self.created
    }
    pub fn creation(&self) -> &BlueprintCourseCreationWitness {
        &self.creation
    }
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
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

impl std::fmt::Display for CurriculumAdoptionCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum adoption preview is not eligible for apply")
    }
}
impl std::error::Error for CurriculumAdoptionCommandError {}

/// Whether a matching completed write was newly applied or replayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurriculumReplayStatus {
    Applied,
    Replayed,
}

/// Browser-safe completed assignment-adoption result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AdoptBlueprintAssignmentCompleted {
    pub course: CourseReference,
    pub assignment: AssignmentReference,
    pub replay: CurriculumReplayStatus,
}

/// Browser-safe completion for a committed fork; it projects the exact created revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ForkBlueprintCourseCompleted {
    pub blueprint: BlueprintReference,
    pub revision: BlueprintRevision,
    pub replay: CurriculumReplayStatus,
}

/// Browser-safe completed whole-course instantiation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InstantiateBlueprintCourseCompleted {
    pub course: CourseReference,
    pub replay: CurriculumReplayStatus,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BlueprintReference, BlueprintRevision, CourseScheduleRevision, ProblemId,
        ProblemVersionRef, QuestionId, UserId, VersionId,
    };
    use uuid::Uuid;

    fn source() -> ObservedBlueprintSource {
        ObservedBlueprintSource {
            reference: BlueprintReference::new(7).expect("BP reference"),
            revision: BlueprintRevision::new(2).expect("revision"),
        }
    }

    fn actor() -> UserId {
        UserId::from_uuid(Uuid::from_u128(1))
    }

    fn destination() -> CourseInstanceWitness {
        CourseInstanceWitness::new(
            CourseReference::new(3).expect("course"),
            CourseScheduleRevision::new(1).expect("revision"),
            vec![],
        )
        .expect("bounded witness")
    }

    fn unavailable_recovery() -> UnavailableCurriculumPinRecovery {
        UnavailableCurriculumPinRecovery {
            source: AssignmentDefinitionSourceView::new(source(), 0, 0).expect("source"),
            position: super::super::CurriculumPinPosition::new(None, 0, 0, None).expect("position"),
            unavailable: ProblemVersionRef {
                problem: ProblemId::from_uuid(Uuid::from_u128(2)),
                version: VersionId::from_uuid(Uuid::from_u128(3)),
            },
            choices: super::super::ReplacementQuestionChoices::new(vec![
                QuestionId::from_canonical_parts("ABCDEF", 'G').expect("question"),
            ])
            .expect("choices"),
        }
    }

    #[test]
    fn operations_are_closed_snake_case_and_preview_bound() {
        let key = CurriculumAdoptionIdempotencyKey::parse("blueprint-apply").expect("key");
        let source = source();
        let fork = ForkBlueprintCoursePreviewView {
            source,
            replacements: CurriculumPinReplacements::default(),
            eligibility: BlueprintAdoptionEligibility::Eligible,
        };
        let command = ForkBlueprintCourseCommand::from_server_record(
            super::super::ForkBlueprintCourseApplyRecord::new(
                source,
                fork.replacements.clone(),
                BlueprintCourseCreationWitness::new(
                    source,
                    actor(),
                    [4; 32],
                    key.clone(),
                    BlueprintReference::new(8).expect("reserved blueprint"),
                ),
                fork.eligibility.clone(),
            )
            .expect("server-held record"),
        );
        assert_eq!(command.source(), &source);
        assert_eq!(command.idempotency_key(), &key);
        let wire = serde_json::to_value(&fork).expect("preview serializes");
        assert!(wire.get("eligibility").is_some());
        assert!(serde_json::from_value::<ForkBlueprintCoursePreviewView>(wire).is_ok());

        let location = AssignmentDefinitionSourceView::new(source, 1, 2).expect("location");
        let adopt = AdoptBlueprintAssignmentPreviewView {
            source: location,
            destination: destination(),
            replacements: CurriculumPinReplacements::default(),
            eligibility: BlueprintAdoptionEligibility::Refused {
                refusal: BlueprintAdoptionRefusal::ScheduleCorrectionsRequired {
                    corrections: vec![],
                },
            },
        };
        assert_eq!(
            super::super::AdoptBlueprintAssignmentApplyRecord::new(
                adopt.source,
                adopt.destination,
                adopt.replacements,
                actor(),
                [3; 32],
                key,
                adopt.eligibility,
            ),
            Err(CurriculumAdoptionCommandError::Refused(
                BlueprintAdoptionRefusal::ScheduleCorrectionsRequired {
                    corrections: vec![]
                }
            ))
        );
    }

    #[test]
    fn adoption_and_instantiation_bind_exact_source_location_and_idempotency() {
        let source = source();
        let location = AssignmentDefinitionSourceView::new(source, 1, 2).expect("location");
        let key = CurriculumAdoptionIdempotencyKey::parse("adopt-blueprint-1").expect("key");
        let adoption = AdoptBlueprintAssignmentPreviewView {
            source: location,
            destination: destination(),
            replacements: CurriculumPinReplacements::default(),
            eligibility: BlueprintAdoptionEligibility::Eligible,
        };
        let command = AdoptBlueprintAssignmentCommand::from_server_record(
            super::super::AdoptBlueprintAssignmentApplyRecord::new(
                adoption.source,
                adoption.destination,
                adoption.replacements,
                actor(),
                [4; 32],
                key.clone(),
                adoption.eligibility,
            )
            .expect("server-held record"),
        );
        assert_eq!(command.source().source(), source);
        assert_eq!(command.source().module_index(), 1);
        assert_eq!(command.source().assignment_index(), 2);
        assert_eq!(command.idempotency_key(), &key);
        assert_eq!(command.destination(), &destination());
        assert_eq!(command.course(), CourseReference::new(3).expect("course"));

        let term =
            CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
        let instantiation = InstantiateBlueprintCoursePreviewView {
            source,
            target_term: term.clone(),
            replacements: CurriculumPinReplacements::default(),
            eligibility: BlueprintAdoptionEligibility::Eligible,
        };
        let command = InstantiateBlueprintCourseCommand::from_server_record(
            super::super::InstantiateBlueprintCourseApplyRecord::new(
                source,
                term.clone(),
                instantiation.replacements,
                CourseInstanceCreationWitness::for_blueprint(
                    source,
                    term.clone(),
                    actor(),
                    [5; 32],
                    key.clone(),
                    CourseReference::new(4).expect("reserved course"),
                ),
                instantiation.eligibility,
            )
            .expect("server-held record"),
        );
        assert_eq!(command.source(), &source);
        assert_eq!(command.target_term(), &term);
        assert_eq!(
            command.creation().reserved_course(),
            CourseReference::new(4).expect("reserved course")
        );
    }

    #[test]
    fn previews_are_snake_case_and_refuse_unknown_or_unbounded_locations() {
        let source = source();
        let preview = ForkBlueprintCoursePreviewView {
            source,
            replacements: CurriculumPinReplacements::default(),
            eligibility: BlueprintAdoptionEligibility::Eligible,
        };
        let wire = serde_json::to_value(preview).expect("preview serializes");
        assert!(wire.get("eligibility").is_some());
        assert!(wire.get("corrections_required").is_none());
        let mut forged = wire;
        forged["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<ForkBlueprintCoursePreviewView>(forged).is_err());
        let bound = u16::try_from(crate::MAX_ASSIGNMENT_ORDERED_ENTRIES).expect("bound");
        assert!(AssignmentDefinitionSourceView::new(source, bound, 0).is_err());
        assert!(AssignmentDefinitionSourceView::new(source, 0, bound).is_err());

        let refusal = serde_json::json!({
            "kind": "unavailable_pin",
            "recovery": serde_json::to_value(unavailable_recovery()).expect("recovery"),
        });
        let mut nested = refusal;
        nested["recovery"]["untrusted"] = serde_json::json!(true);
        assert!(serde_json::from_value::<BlueprintAdoptionRefusal>(nested).is_err());
    }

    #[test]
    fn every_blueprint_refusal_prevents_command_construction() {
        let key = CurriculumAdoptionIdempotencyKey::parse("refused-blueprint").expect("key");
        let refusals = vec![
            BlueprintAdoptionRefusal::ScheduleCorrectionsRequired {
                corrections: vec![CourseInstanceScheduleCorrection {
                    field: super::super::CourseInstanceScheduleField::DueAt,
                    reason: super::super::CourseInstanceScheduleReason::AmbiguousLocalTime,
                }],
            },
            BlueprintAdoptionRefusal::UnavailablePin {
                recovery: unavailable_recovery(),
            },
            BlueprintAdoptionRefusal::SourceRevisionDrift { observed: source() },
            BlueprintAdoptionRefusal::DestinationWitnessDrift {
                expected: destination(),
                observed: CourseInstanceWitness::new(
                    CourseReference::new(3).expect("course"),
                    CourseScheduleRevision::new(2).expect("revision"),
                    vec![],
                )
                .expect("bounded witness"),
            },
        ];
        for refusal in refusals {
            let creation = BlueprintCourseCreationWitness::new(
                source(),
                actor(),
                [9; 32],
                key.clone(),
                BlueprintReference::new(9).expect("reserved blueprint"),
            );
            assert_eq!(
                super::super::ForkBlueprintCourseApplyRecord::new(
                    source(),
                    CurriculumPinReplacements::default(),
                    creation,
                    BlueprintAdoptionEligibility::Refused {
                        refusal: refusal.clone(),
                    },
                ),
                Err(CurriculumAdoptionCommandError::Refused(refusal))
            );
        }
    }

    #[test]
    fn creation_witnesses_bind_commands_without_a_browser_serde_path() {
        let key = CurriculumAdoptionIdempotencyKey::parse("creation-binding").expect("key");
        let fork = ForkBlueprintCoursePreviewView {
            source: source(),
            replacements: CurriculumPinReplacements::default(),
            eligibility: BlueprintAdoptionEligibility::Eligible,
        };
        let creation = BlueprintCourseCreationWitness::new(
            source(),
            actor(),
            [6; 32],
            key.clone(),
            BlueprintReference::new(10).expect("reserved blueprint"),
        );
        let command = ForkBlueprintCourseCommand::from_server_record(
            super::super::ForkBlueprintCourseApplyRecord::new(
                source(),
                fork.replacements.clone(),
                creation.clone(),
                fork.eligibility.clone(),
            )
            .expect("server creation binding"),
        );
        assert_eq!(command.creation(), &creation);
        assert!(serde_json::to_value(&fork).is_ok());

        let mismatched = BlueprintCourseCreationWitness::new(
            ObservedBlueprintSource {
                reference: BlueprintReference::new(11).expect("other source"),
                revision: BlueprintRevision::new(1).expect("revision"),
            },
            actor(),
            [7; 32],
            key.clone(),
            BlueprintReference::new(12).expect("reserved blueprint"),
        );
        assert_eq!(
            super::super::ForkBlueprintCourseApplyRecord::new(
                source(),
                fork.replacements.clone(),
                mismatched,
                fork.eligibility.clone(),
            ),
            Err(CurriculumAdoptionCommandError::CreationWitnessMismatch)
        );

        let term =
            CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
        let instantiate = InstantiateBlueprintCoursePreviewView {
            source: source(),
            target_term: term.clone(),
            replacements: CurriculumPinReplacements::default(),
            eligibility: BlueprintAdoptionEligibility::Eligible,
        };
        let instance_key =
            CurriculumAdoptionIdempotencyKey::parse("instance-binding").expect("key");
        let instance_creation = CourseInstanceCreationWitness::for_blueprint(
            source(),
            term.clone(),
            actor(),
            [8; 32],
            instance_key.clone(),
            CourseReference::new(13).expect("reserved course"),
        );
        let instance_command = InstantiateBlueprintCourseCommand::from_server_record(
            super::super::InstantiateBlueprintCourseApplyRecord::new(
                source(),
                term,
                instantiate.replacements,
                instance_creation.clone(),
                instantiate.eligibility,
            )
            .expect("server CourseInstance creation binding"),
        );
        assert_eq!(instance_command.creation(), &instance_creation);
    }

    #[test]
    fn server_record_authority_survives_preview_mutation_and_fork_receipt_binds_creation() {
        let key = CurriculumAdoptionIdempotencyKey::parse("server-record").expect("key");
        let creation = BlueprintCourseCreationWitness::new(
            source(),
            actor(),
            [3; 32],
            key,
            BlueprintReference::new(14).expect("reserved BlueprintCourse"),
        );
        let record = super::super::ForkBlueprintCourseApplyRecord::new(
            source(),
            CurriculumPinReplacements::default(),
            creation.clone(),
            BlueprintAdoptionEligibility::Eligible,
        )
        .expect("server record");
        let mut browser_preview = ForkBlueprintCoursePreviewView {
            source: source(),
            replacements: CurriculumPinReplacements::default(),
            eligibility: BlueprintAdoptionEligibility::Eligible,
        };
        browser_preview.source = ObservedBlueprintSource {
            reference: BlueprintReference::new(15).expect("forged source"),
            revision: BlueprintRevision::new(1).expect("forged revision"),
        };
        let command = ForkBlueprintCourseCommand::from_server_record(record);
        assert_ne!(command.source(), &browser_preview.source);

        let created = ObservedBlueprintSource {
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
            replay: CurriculumReplayStatus::Applied,
        };
        assert_eq!(
            completion.blueprint,
            receipt.creation().reserved_blueprint()
        );
    }
}
