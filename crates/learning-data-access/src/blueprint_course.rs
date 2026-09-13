//! Session-authorized persistence contracts for Blueprint Revisions and lineage metadata.

use std::collections::BTreeMap;

use async_trait::async_trait;
use question_model::{
    AssignmentEntryScoringRule, AssignmentInstructions, AssignmentPointValue, AssignmentTitle,
    BlueprintAssignmentContent, BlueprintAssignmentContentInput, BlueprintAssignmentEditChoice,
    BlueprintAssignmentEntryContent, BlueprintAssignmentReference, BlueprintAvailability,
    BlueprintCourseContent, BlueprintCourseModuleContent, BlueprintCourseReadAccess,
    BlueprintCourseReference, BlueprintCourseValidationError, BlueprintMetadataEtag,
    BlueprintMetadataState, BlueprintModuleEditChoice, BlueprintModuleReference,
    BlueprintQuestionPoolContent, BlueprintRevision, BlueprintRevisionContent,
    BlueprintRevisionReference, CreateBlueprintCourseInput, CreateBlueprintCourseReceipt,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId, QuestionPoolSelectionRule,
    QuestionRevisionReference, RelativeAssignmentSchedule, RenameBlueprintCourseInput,
    ReplaceBlueprintCourseContentInput, RequestChecksum, SaveBlueprintCourseReceipt,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// One current readable Blueprint lineage and the content selected by its read boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBlueprintCourse {
    pub reference: BlueprintCourseReference,
    pub short_name: String,
    pub long_name: String,
    pub availability: BlueprintAvailability,
    pub metadata_etag: BlueprintMetadataEtag,
    pub current_revision: BlueprintRevision,
    pub read_access: BlueprintCourseReadAccess,
    /// The exact current immutable Revision content.
    pub content: StoredBlueprintCourseContent,
}

/// Compact answer-free readable Blueprint lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBlueprintCourseSummary {
    pub reference: BlueprintCourseReference,
    pub short_name: String,
    pub long_name: String,
    pub availability: BlueprintAvailability,
    pub metadata_etag: BlueprintMetadataEtag,
    pub current_revision: BlueprintRevision,
    pub read_access: BlueprintCourseReadAccess,
}

/// Exact immutable Blueprint Revision content, including after lineage archive.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBlueprintRevision {
    pub reference: BlueprintRevisionReference,
    pub content: StoredBlueprintCourseContent,
}

/// Durable complete Blueprint content with server-resolved Question Revision pins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintCourseContent {
    pub modules: Vec<StoredBlueprintModule>,
}

/// One retained Blueprint Module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintModule {
    pub blueprint_module_reference: BlueprintModuleReference,
    pub label: String,
    pub assignments: Vec<StoredBlueprintAssignment>,
}

/// One retained Blueprint Assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintAssignment {
    pub blueprint_assignment_reference: BlueprintAssignmentReference,
    pub content: StoredBlueprintAssignmentContent,
}

/// Reusable assignment content after Question IDs become exact Revision pins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintAssignmentContent {
    pub title: String,
    pub instructions: AssignmentInstructions,
    pub entries: Vec<StoredBlueprintAssignmentEntry>,
    pub defaults: question_model::BlueprintAssignmentDefaults,
    pub schedule: RelativeAssignmentSchedule,
}

/// Stored entry retaining exact immutable Question Revision references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoredBlueprintAssignmentEntry {
    Fixed {
        question_revision: QuestionRevisionReference,
        points_possible: AssignmentPointValue,
        scoring_rule: AssignmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    Pool {
        question_revisions: Vec<QuestionRevisionReference>,
        selection_count: u32,
        points_per_item: AssignmentPointValue,
        scoring_rule: AssignmentEntryScoringRule,
        selection_rule: QuestionPoolSelectionRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
}

impl StoredBlueprintCourseContent {
    /// Resolves a newly accepted browser request into server-owned child identities and pins.
    pub fn from_create(
        input: CreateBlueprintCourseInput,
        pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
    ) -> Result<Self, StoreError> {
        input.validate().map_err(invalid_content)?;
        let modules = input
            .modules
            .into_iter()
            .map(|module| {
                let assignments = module
                    .assignments
                    .into_iter()
                    .map(|content| {
                        Ok(StoredBlueprintAssignment {
                            blueprint_assignment_reference: BlueprintAssignmentReference::from_uuid(
                                random_uuid()?,
                            ),
                            content: StoredBlueprintAssignmentContent::from_input(content, pins)?,
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?;
                Ok(StoredBlueprintModule {
                    blueprint_module_reference: BlueprintModuleReference::from_uuid(random_uuid()?),
                    label: module.label,
                    assignments,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let content = Self { modules };
        content.checksum()?;
        Ok(content)
    }

    /// Resolves a complete replacement against the exact expected head.
    pub fn from_replace(
        input: ReplaceBlueprintCourseContentInput,
        prior: &Self,
        pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
    ) -> Result<Self, StoreError> {
        input.validate().map_err(invalid_content)?;
        let modules = input
            .modules
            .into_iter()
            .map(|module| {
                let module_reference = match module.choice {
                    BlueprintModuleEditChoice::Retained {
                        blueprint_module_reference,
                    } => prior
                        .modules
                        .iter()
                        .find(|candidate| {
                            candidate.blueprint_module_reference == blueprint_module_reference
                        })
                        .map(|candidate| candidate.blueprint_module_reference)
                        .ok_or_else(|| invalid("retained Blueprint Module Reference"))?,
                    BlueprintModuleEditChoice::New => {
                        BlueprintModuleReference::from_uuid(random_uuid()?)
                    }
                };
                let assignments = module
                    .assignments
                    .into_iter()
                    .map(|assignment| Self::replacement_assignment(assignment, prior, pins))
                    .collect::<Result<Vec<_>, StoreError>>()?;
                Ok(StoredBlueprintModule {
                    blueprint_module_reference: module_reference,
                    label: module.label,
                    assignments,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let content = Self { modules };
        content.checksum()?;
        Ok(content)
    }

    pub fn requested_question_ids_from_create(
        input: &CreateBlueprintCourseInput,
    ) -> Vec<QuestionId> {
        input
            .modules
            .iter()
            .flat_map(|module| module.assignments.iter())
            .flat_map(requested_question_ids)
            .collect()
    }

    pub fn requested_question_ids_from_replace(
        input: &ReplaceBlueprintCourseContentInput,
    ) -> Vec<QuestionId> {
        input
            .modules
            .iter()
            .flat_map(|module| module.assignments.iter())
            .flat_map(|assignment| requested_question_ids(&assignment.content))
            .collect()
    }

    /// Rebuilds the one canonical content checksum used by the new base schema.
    pub fn checksum(&self) -> Result<question_model::BlueprintContentChecksum, StoreError> {
        let modules = self
            .modules
            .iter()
            .map(|module| {
                let assignments = module
                    .assignments
                    .iter()
                    .map(StoredBlueprintAssignment::to_domain)
                    .collect::<Result<Vec<_>, StoreError>>()?;
                BlueprintCourseModuleContent::new(
                    module.blueprint_module_reference,
                    module.label.clone(),
                    assignments,
                )
                .map_err(invalid_content)
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let course = BlueprintCourseContent::new(modules).map_err(invalid_content)?;
        Ok(BlueprintRevisionContent::course(course).checksum())
    }

    fn replacement_assignment(
        assignment: question_model::BlueprintAssignmentReplacementInput,
        prior: &StoredBlueprintCourseContent,
        pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
    ) -> Result<StoredBlueprintAssignment, StoreError> {
        let reference = match assignment.choice {
            BlueprintAssignmentEditChoice::Retained {
                blueprint_assignment_reference,
            } => prior
                .modules
                .iter()
                .flat_map(|module| module.assignments.iter())
                .find(|candidate| {
                    candidate.blueprint_assignment_reference == blueprint_assignment_reference
                })
                .map(|candidate| candidate.blueprint_assignment_reference)
                .ok_or_else(|| invalid("retained Blueprint Assignment Reference"))?,
            BlueprintAssignmentEditChoice::New => {
                BlueprintAssignmentReference::from_uuid(random_uuid()?)
            }
        };
        Ok(StoredBlueprintAssignment {
            blueprint_assignment_reference: reference,
            content: StoredBlueprintAssignmentContent::from_input(assignment.content, pins)?,
        })
    }
}

impl StoredBlueprintAssignmentContent {
    fn from_input(
        input: BlueprintAssignmentContentInput,
        pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
    ) -> Result<Self, StoreError> {
        input.validate().map_err(invalid_content)?;
        let entries = input
            .entries
            .into_iter()
            .map(|entry| match entry {
                question_model::BlueprintAssignmentEntryInput::Fixed(value) => {
                    Ok(StoredBlueprintAssignmentEntry::Fixed {
                        question_revision: pin(&value.question_id, pins)?,
                        points_possible: value.points_possible,
                        scoring_rule: value.scoring_rule,
                        question_attempt_limit: value.question_attempt_limit,
                        question_attempt_time_limit: value.question_attempt_time_limit,
                    })
                }
                question_model::BlueprintAssignmentEntryInput::Pool(value) => {
                    Ok(StoredBlueprintAssignmentEntry::Pool {
                        question_revisions: value
                            .items
                            .iter()
                            .map(|question_id| pin(question_id, pins))
                            .collect::<Result<Vec<_>, _>>()?,
                        selection_count: value.selection_count,
                        points_per_item: value.points_per_item,
                        scoring_rule: value.scoring_rule,
                        selection_rule: value.selection_rule,
                        question_attempt_limit: value.question_attempt_limit,
                        question_attempt_time_limit: value.question_attempt_time_limit,
                    })
                }
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        Ok(Self {
            title: input.title,
            instructions: input.instructions,
            entries,
            defaults: input.defaults,
            schedule: input.schedule,
        })
    }

    fn to_domain(
        &self,
        blueprint_assignment_reference: BlueprintAssignmentReference,
    ) -> Result<BlueprintAssignmentContent, StoreError> {
        let entries = self
            .entries
            .iter()
            .map(|entry| match entry {
                StoredBlueprintAssignmentEntry::Fixed {
                    question_revision,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => Ok(BlueprintAssignmentEntryContent::Fixed {
                    reference: question_revision.clone(),
                    points_possible: *points_possible,
                    scoring_rule: *scoring_rule,
                    question_attempt_limit: *question_attempt_limit,
                    question_attempt_time_limit: *question_attempt_time_limit,
                }),
                StoredBlueprintAssignmentEntry::Pool {
                    question_revisions,
                    selection_count,
                    points_per_item,
                    scoring_rule,
                    selection_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => Ok(BlueprintAssignmentEntryContent::Pool(
                    BlueprintQuestionPoolContent::new(
                        question_revisions.clone(),
                        *selection_count,
                        *points_per_item,
                        *scoring_rule,
                        *selection_rule,
                        *question_attempt_limit,
                        *question_attempt_time_limit,
                    )
                    .map_err(invalid_content)?,
                )),
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        BlueprintAssignmentContent::new(
            blueprint_assignment_reference,
            AssignmentTitle::try_new(self.title.clone())
                .map_err(|_| invalid("Blueprint Assignment title"))?,
            self.instructions.clone(),
            entries,
            self.defaults.clone(),
            self.schedule.clone(),
        )
        .map_err(invalid_content)
    }
}

impl StoredBlueprintAssignment {
    fn to_domain(&self) -> Result<BlueprintAssignmentContent, StoreError> {
        self.content.to_domain(self.blueprint_assignment_reference)
    }
}

/// Store boundary for immutable Blueprint Revisions and lineage metadata.
#[async_trait]
pub trait BlueprintCourseStore: Send + Sync {
    async fn list_blueprint_courses(
        &self,
        session: SessionTokenHash,
    ) -> Result<Vec<StoredBlueprintCourseSummary>, StoreError>;
    async fn load_blueprint_course(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
    ) -> Result<StoredBlueprintCourse, StoreError>;
    async fn load_blueprint_revision(
        &self,
        session: SessionTokenHash,
        reference: BlueprintRevisionReference,
    ) -> Result<StoredBlueprintRevision, StoreError>;
    async fn create_blueprint_course(
        &self,
        session: SessionTokenHash,
        request_checksum: RequestChecksum,
        input: CreateBlueprintCourseInput,
    ) -> Result<CreateBlueprintCourseReceipt, StoreError>;
    async fn save_blueprint_course(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_revision: BlueprintRevision,
        request_checksum: RequestChecksum,
        input: ReplaceBlueprintCourseContentInput,
    ) -> Result<SaveBlueprintCourseReceipt, StoreError>;
    async fn rename_blueprint_course(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
        input: RenameBlueprintCourseInput,
    ) -> Result<BlueprintMetadataState, StoreError>;
    async fn archive_blueprint(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
        confirmation_title: &str,
    ) -> Result<BlueprintMetadataState, StoreError>;
    async fn restore_blueprint(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
    ) -> Result<BlueprintMetadataState, StoreError>;
}

fn requested_question_ids(input: &BlueprintAssignmentContentInput) -> Vec<QuestionId> {
    input
        .entries
        .iter()
        .flat_map(|entry| match entry {
            question_model::BlueprintAssignmentEntryInput::Fixed(value) => {
                vec![value.question_id.clone()]
            }
            question_model::BlueprintAssignmentEntryInput::Pool(value) => value.items.clone(),
        })
        .collect()
}

fn pin(
    question_id: &QuestionId,
    pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
) -> Result<QuestionRevisionReference, StoreError> {
    pins.get(question_id)
        .cloned()
        .ok_or_else(|| invalid("currently available Published Question"))
}

fn invalid_content(error: BlueprintCourseValidationError) -> StoreError {
    StoreError::InvalidRecord(format!("Blueprint Course content is invalid: {error}"))
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("stored {label} is invalid"))
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Blueprint Course UUID randomness unavailable".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::{
        AssignmentActivityRules, BlueprintAssignmentDefaults, LateWorkRule,
        StudentFeedbackReleaseRule,
    };
    use uuid::Uuid;

    fn content(assignment_identity: u128) -> StoredBlueprintCourseContent {
        StoredBlueprintCourseContent {
            modules: vec![StoredBlueprintModule {
                blueprint_module_reference: BlueprintModuleReference::from_uuid(Uuid::from_u128(1)),
                label: "Module".to_string(),
                assignments: vec![StoredBlueprintAssignment {
                    blueprint_assignment_reference: BlueprintAssignmentReference::from_uuid(
                        Uuid::from_u128(assignment_identity),
                    ),
                    content: StoredBlueprintAssignmentContent {
                        title: "Assignment".to_string(),
                        instructions: AssignmentInstructions::default(),
                        entries: vec![StoredBlueprintAssignmentEntry::Fixed {
                            question_revision: QuestionRevisionReference {
                                question_id: "7K3-M9QX".parse().expect("Question ID"),
                                revision_number: question_model::QuestionRevisionNumber::new(1)
                                    .expect("revision"),
                            },
                            points_possible: AssignmentPointValue::from_whole(1),
                            scoring_rule: AssignmentEntryScoringRule::Normal,
                            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                        }],
                        defaults: BlueprintAssignmentDefaults {
                            assignment_attempt_time_limit_seconds: None,
                            attempt_limit: None,
                            late_work_rule: LateWorkRule::Accept,
                            activity_rules: AssignmentActivityRules::default(),
                            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                        },
                        schedule: RelativeAssignmentSchedule::default(),
                    },
                }],
            }],
        }
    }

    #[test]
    fn checksum_binds_stable_child_identities() {
        let expected = content(2).checksum().expect("checksum");
        let tampered = content(3).checksum().expect("checksum");
        assert_ne!(expected, tampered);
    }
}
