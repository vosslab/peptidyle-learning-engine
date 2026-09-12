//! Session-authorized persistence contracts for Blueprint Drafts and publications.
//!
//! A Blueprint Course is a stable lineage. Its owner edits one private Draft;
//! a deliberate publication copies that Draft into immutable revision evidence.

use std::collections::BTreeMap;

use async_trait::async_trait;
use question_model::{
    AssignmentEntryScoringRule, AssignmentInstructions, AssignmentPointValue, AssignmentTitle,
    BlueprintAssignmentContent, BlueprintAssignmentContentInput, BlueprintAssignmentEditChoice,
    BlueprintAssignmentEntryContent, BlueprintAssignmentReference, BlueprintAvailability,
    BlueprintAvailabilityEditNumber, BlueprintCourseContent, BlueprintCourseModuleContent,
    BlueprintCourseReadAccess, BlueprintCourseReference, BlueprintCourseValidationError,
    BlueprintDraftEditNumber, BlueprintModuleEditChoice, BlueprintModuleReference,
    BlueprintQuestionPoolContent, BlueprintRevision, BlueprintRevisionContent,
    BlueprintRevisionReference, CreateBlueprintCourseContentInput, CreateBlueprintDraftReceipt,
    PublishBlueprintDraftReceipt, QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId,
    QuestionPoolSelectionRule, QuestionRevisionReference, RelativeAssignmentSchedule,
    ReplaceBlueprintCourseContentInput, RequestChecksum, SaveBlueprintDraftReceipt,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// One current readable Blueprint lineage and the content selected by its read boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBlueprintCourse {
    pub reference: BlueprintCourseReference,
    pub title: String,
    pub availability: BlueprintAvailability,
    pub availability_edit_number: BlueprintAvailabilityEditNumber,
    pub latest_published_revision: Option<BlueprintRevision>,
    pub read_access: BlueprintCourseReadAccess,
    /// The owner's Draft or the authorized reader's latest immutable Revision.
    pub content: StoredBlueprintCourseContent,
    /// Present only for the private owner Draft.
    pub draft_edit_number: Option<BlueprintDraftEditNumber>,
}

/// Compact answer-free readable Blueprint lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBlueprintCourseSummary {
    pub reference: BlueprintCourseReference,
    pub title: String,
    pub availability: BlueprintAvailability,
    pub availability_edit_number: BlueprintAvailabilityEditNumber,
    pub latest_published_revision: Option<BlueprintRevision>,
    pub read_access: BlueprintCourseReadAccess,
}

/// Exact immutable Blueprint Revision content, including after lineage archive.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBlueprintRevision {
    pub reference: BlueprintRevisionReference,
    pub content: StoredBlueprintCourseContent,
}

/// Current availability result after a qualified lineage transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredBlueprintAvailability {
    pub availability: BlueprintAvailability,
    pub edit_number: BlueprintAvailabilityEditNumber,
}

/// Durable complete Blueprint content with server-resolved Question Revision pins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintCourseContent {
    pub title: String,
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
        input: CreateBlueprintCourseContentInput,
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
        let content = Self {
            title: input.title,
            modules,
        };
        content.checksum()?;
        Ok(content)
    }

    /// Resolves a complete Draft replacement while retaining only owned child identities.
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
                let prior_module = match module.choice {
                    BlueprintModuleEditChoice::Retained {
                        blueprint_module_reference,
                    } => prior
                        .modules
                        .iter()
                        .find(|candidate| {
                            candidate.blueprint_module_reference == blueprint_module_reference
                        })
                        .ok_or_else(|| invalid("retained Blueprint Module Reference"))?,
                    BlueprintModuleEditChoice::New => {
                        return Self::new_module(module.label, module.assignments, pins);
                    }
                };
                let assignments = module
                    .assignments
                    .into_iter()
                    .map(|assignment| Self::replacement_assignment(assignment, prior_module, pins))
                    .collect::<Result<Vec<_>, StoreError>>()?;
                Ok(StoredBlueprintModule {
                    blueprint_module_reference: prior_module.blueprint_module_reference,
                    label: module.label,
                    assignments,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let content = Self {
            title: input.title,
            modules,
        };
        content.checksum()?;
        Ok(content)
    }

    pub fn requested_question_ids_from_create(
        input: &CreateBlueprintCourseContentInput,
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
                    .map(|assignment| assignment.content.to_domain())
                    .collect::<Result<Vec<_>, StoreError>>()?;
                BlueprintCourseModuleContent::new(module.label.clone(), assignments)
                    .map_err(invalid_content)
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let course =
            BlueprintCourseContent::new(self.title.clone(), modules).map_err(invalid_content)?;
        Ok(BlueprintRevisionContent::course(course).checksum())
    }

    fn new_module(
        label: String,
        assignments: Vec<question_model::BlueprintAssignmentReplacementInput>,
        pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
    ) -> Result<StoredBlueprintModule, StoreError> {
        let assignments = assignments
            .into_iter()
            .map(|assignment| match assignment.choice {
                BlueprintAssignmentEditChoice::New => Ok(StoredBlueprintAssignment {
                    blueprint_assignment_reference: BlueprintAssignmentReference::from_uuid(
                        random_uuid()?,
                    ),
                    content: StoredBlueprintAssignmentContent::from_input(
                        assignment.content,
                        pins,
                    )?,
                }),
                BlueprintAssignmentEditChoice::Retained { .. } => Err(invalid(
                    "retained Blueprint Assignment in a new Blueprint Module",
                )),
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        Ok(StoredBlueprintModule {
            blueprint_module_reference: BlueprintModuleReference::from_uuid(random_uuid()?),
            label,
            assignments,
        })
    }

    fn replacement_assignment(
        assignment: question_model::BlueprintAssignmentReplacementInput,
        prior_module: &StoredBlueprintModule,
        pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
    ) -> Result<StoredBlueprintAssignment, StoreError> {
        let reference = match assignment.choice {
            BlueprintAssignmentEditChoice::Retained {
                blueprint_assignment_reference,
            } => prior_module
                .assignments
                .iter()
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

    fn to_domain(&self) -> Result<BlueprintAssignmentContent, StoreError> {
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

/// Store boundary for the direct Blueprint Draft, publication, and availability lifecycle.
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
    async fn create_blueprint_draft(
        &self,
        session: SessionTokenHash,
        request_checksum: RequestChecksum,
        input: CreateBlueprintCourseContentInput,
    ) -> Result<CreateBlueprintDraftReceipt, StoreError>;
    async fn save_blueprint_draft(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_edit_number: BlueprintDraftEditNumber,
        request_checksum: RequestChecksum,
        input: ReplaceBlueprintCourseContentInput,
    ) -> Result<SaveBlueprintDraftReceipt, StoreError>;
    async fn publish_blueprint_draft(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_edit_number: BlueprintDraftEditNumber,
        request_checksum: RequestChecksum,
    ) -> Result<PublishBlueprintDraftReceipt, StoreError>;
    async fn archive_blueprint(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_edit_number: BlueprintAvailabilityEditNumber,
        confirmation_title: &str,
    ) -> Result<StoredBlueprintAvailability, StoreError>;
    async fn restore_blueprint(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_edit_number: BlueprintAvailabilityEditNumber,
    ) -> Result<StoredBlueprintAvailability, StoreError>;
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
