//! Session-authorized persistence contracts for reusable Blueprint Courses.
//!
//! The stored content keeps exact immutable Question Revision pins. Browser
//! routes receive only answer-free views assembled from that server record.

use std::collections::BTreeMap;

use async_trait::async_trait;
use question_model::{
    AssignmentEntryScoringRule, AssignmentInstructions, AssignmentPointValue, AssignmentTitle,
    BlueprintAssignmentContent, BlueprintAssignmentContentInput, BlueprintAssignmentEditChoice,
    BlueprintAssignmentEntryContent, BlueprintAssignmentReference,
    BlueprintAssignmentReplacementInput, BlueprintContentChecksum, BlueprintCourseContent,
    BlueprintCourseModuleContent, BlueprintCourseReadAccess, BlueprintCourseReference,
    BlueprintCourseValidationError, BlueprintModuleEditChoice, BlueprintModuleReference,
    BlueprintQuestionPoolContent, BlueprintRevision, BlueprintRevisionContent,
    CreateBlueprintCourseContentInput, QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId,
    QuestionPoolSelectionRule, QuestionRevisionReference, RelativeAssignmentSchedule,
    ReplaceBlueprintCourseContentInput,
};
use serde::{Deserialize, Serialize};

use crate::{SessionTokenHash, StoreError};

/// One current Blueprint Course record retained by the server Store.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBlueprintCourse {
    /// Browser-safe public route reference.
    pub reference: BlueprintCourseReference,
    /// Current immutable Blueprint Revision number.
    pub revision: BlueprintRevision,
    /// Current caller's closed read classification.
    pub read_access: BlueprintCourseReadAccess,
    /// Complete answer-free content and exact Question Revision pins.
    pub content: StoredBlueprintCourseContent,
}

/// Compact answer-free result for a Blueprint Course workspace list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBlueprintCourseSummary {
    /// Browser-safe public route reference.
    pub reference: BlueprintCourseReference,
    /// Current instructor-visible title.
    pub title: String,
    /// Current immutable Blueprint Revision number.
    pub revision: BlueprintRevision,
    /// Current caller's closed read classification.
    pub read_access: BlueprintCourseReadAccess,
}

/// Durable complete Blueprint Revision Content with server-resolved Question pins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintCourseContent {
    /// Instructor-visible Blueprint Course title.
    pub title: String,
    /// Ordered labelled reusable modules.
    pub modules: Vec<StoredBlueprintModule>,
}

/// One retained Blueprint Module in complete stored Blueprint Revision Content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintModule {
    /// Server-allocated stable Blueprint Module lineage identity.
    pub blueprint_module_reference: BlueprintModuleReference,
    /// Instructor-visible module label.
    pub label: String,
    /// Ordered reusable Blueprint Assignments.
    pub assignments: Vec<StoredBlueprintAssignment>,
}

/// One retained Blueprint Assignment in complete stored Blueprint Revision Content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintAssignment {
    /// Server-allocated stable Blueprint Assignment lineage identity.
    pub blueprint_assignment_reference: BlueprintAssignmentReference,
    /// Complete answer-free reusable assignment content.
    pub content: StoredBlueprintAssignmentContent,
}

/// Reusable assignment content after Question IDs become exact Question Revision pins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintAssignmentContent {
    /// Instructor-facing assignment title.
    pub title: String,
    /// Student-facing reusable instructions.
    pub instructions: AssignmentInstructions,
    /// Ordered fixed Question and Question Pool entries.
    pub entries: Vec<StoredBlueprintAssignmentEntry>,
    /// Reusable assignment defaults.
    pub defaults: question_model::BlueprintAssignmentDefaults,
    /// Optional target-term-relative schedule defaults.
    pub schedule: RelativeAssignmentSchedule,
}

/// Stored entry retaining exact immutable Question Revision References.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoredBlueprintAssignmentEntry {
    /// One fixed Question Revision pin.
    Fixed {
        /// Exact Question Revision retained for future course creation.
        question_revision: QuestionRevisionReference,
        /// Points copied to the future Assignment entry.
        points_possible: AssignmentPointValue,
        /// Score treatment copied to the future Assignment entry.
        scoring_rule: AssignmentEntryScoringRule,
        /// Question Attempt retry boundary.
        question_attempt_limit: QuestionAttemptLimit,
        /// Question Attempt timing boundary.
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One ordered Question Pool of exact Question Revision pins.
    Pool {
        /// Exact Question Revision pins in authored order.
        question_revisions: Vec<QuestionRevisionReference>,
        /// Number of Question Pool Items selected for an Assignment Attempt.
        selection_count: u32,
        /// Points copied for each selected Question Pool Item.
        points_per_item: AssignmentPointValue,
        /// Score treatment copied to the future Assignment entry.
        scoring_rule: AssignmentEntryScoringRule,
        /// Complete reviewed Question Pool selection behavior.
        selection_rule: QuestionPoolSelectionRule,
        /// Question Attempt retry boundary.
        question_attempt_limit: QuestionAttemptLimit,
        /// Question Attempt timing boundary.
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
}

impl StoredBlueprintCourseContent {
    /// Resolves a browser creation request into server-owned child identities and Question pins.
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

    /// Resolves a complete replacement while retaining only server-owned child identities.
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

    /// Returns every requested public Question ID before the Store resolves current pins.
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

    /// Returns every requested public Question ID before the Store resolves current pins.
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

    /// Rebuilds the domain model and returns its canonical Blueprint Content Checksum.
    pub fn checksum(&self) -> Result<BlueprintContentChecksum, StoreError> {
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
        assignments: Vec<BlueprintAssignmentReplacementInput>,
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
        assignment: BlueprintAssignmentReplacementInput,
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

/// Store boundary for current published Blueprint Course lifecycle operations.
#[async_trait]
pub trait BlueprintCourseStore: Send + Sync {
    /// Lists Blueprint Courses readable by this active Instructor.
    async fn list_blueprint_courses(
        &self,
        session_token_hash: SessionTokenHash,
    ) -> Result<Vec<StoredBlueprintCourseSummary>, StoreError>;

    /// Loads one current readable Blueprint Course and its exact stored revision content.
    async fn load_blueprint_course(
        &self,
        session_token_hash: SessionTokenHash,
        reference: BlueprintCourseReference,
    ) -> Result<StoredBlueprintCourse, StoreError>;

    /// Creates and publishes an initial immutable Blueprint Revision.
    async fn create_blueprint_course(
        &self,
        session_token_hash: SessionTokenHash,
        input: CreateBlueprintCourseContentInput,
    ) -> Result<StoredBlueprintCourse, StoreError>;

    /// Replaces the current owner head with a successor published Blueprint Revision.
    async fn replace_blueprint_course(
        &self,
        session_token_hash: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_revision: BlueprintRevision,
        input: ReplaceBlueprintCourseContentInput,
    ) -> Result<StoredBlueprintCourse, StoreError>;
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
