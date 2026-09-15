//! Session-authorized persistence contracts for Blueprint Revisions and lineage metadata.

use std::collections::BTreeMap;

use async_trait::async_trait;
use question_model::{
    AssessmentEntryScoringRule, AssessmentInstructions, AssessmentPointValue, AssessmentTitle,
    BlueprintAssessmentContent, BlueprintAssessmentContentInput, BlueprintAssessmentEditChoice,
    BlueprintAssessmentEntryContent, BlueprintAssessmentReference, BlueprintAvailability,
    BlueprintCourseContent, BlueprintCourseModuleContent, BlueprintCourseReadAccess,
    BlueprintCourseReference, BlueprintCourseValidationError, BlueprintMetadataEtag,
    BlueprintMetadataState, BlueprintModuleEditChoice, BlueprintModuleReference,
    BlueprintQuestionPoolContent, BlueprintRevision, BlueprintRevisionContent,
    BlueprintRevisionReference, CreateBlueprintCourseInput, CreateBlueprintCourseReceipt,
    QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId, QuestionPoolSelectionRule,
    QuestionRevisionReference, RenameBlueprintCourseInput, ReplaceBlueprintCourseContentInput,
    RequestChecksum, SaveBlueprintCourseReceipt,
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
    /// Lifetime Course Instances adopted from this Blueprint lineage, across Revisions.
    pub total_adoptions: u64,
    /// Students counted once per adopted Course Instance, including ended memberships.
    pub total_students_ever_enrolled: u64,
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
    pub assessments: Vec<StoredBlueprintAssessment>,
}

/// One retained Blueprint Assessment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintAssessment {
    pub blueprint_assessment_reference: BlueprintAssessmentReference,
    pub content: StoredBlueprintAssessmentContent,
}

/// Reusable assessment content after Question IDs become exact Revision pins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintAssessmentContent {
    pub title: String,
    pub instructions: AssessmentInstructions,
    pub entries: Vec<StoredBlueprintAssessmentEntry>,
    pub defaults: question_model::BlueprintAssessmentDefaults,
}

/// Stored entry retaining exact immutable Question Revision references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoredBlueprintAssessmentEntry {
    Fixed {
        question_revision: QuestionRevisionReference,
        points_possible: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    Pool {
        question_pool_revision: question_model::QuestionPoolRevisionReference,
        selection_count: std::num::NonZeroU32,
        points_per_item: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
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
        pool_revisions: &BTreeMap<QuestionId, question_model::QuestionPoolRevisionReference>,
    ) -> Result<Self, StoreError> {
        input.validate().map_err(invalid_content)?;
        let modules = input
            .modules
            .into_iter()
            .map(|module| {
                let assessments = module
                    .assessments
                    .into_iter()
                    .map(|content| {
                        Ok(StoredBlueprintAssessment {
                            blueprint_assessment_reference: BlueprintAssessmentReference::from_uuid(
                                random_uuid()?,
                            ),
                            content: StoredBlueprintAssessmentContent::from_input(
                                content,
                                pins,
                                pool_revisions,
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?;
                Ok(StoredBlueprintModule {
                    blueprint_module_reference: BlueprintModuleReference::from_uuid(random_uuid()?),
                    label: module.label,
                    assessments,
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
        pool_revisions: &BTreeMap<QuestionId, question_model::QuestionPoolRevisionReference>,
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
                let assessments = module
                    .assessments
                    .into_iter()
                    .map(|assessment| {
                        Self::replacement_assessment(assessment, prior, pins, pool_revisions)
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?;
                Ok(StoredBlueprintModule {
                    blueprint_module_reference: module_reference,
                    label: module.label,
                    assessments,
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
            .flat_map(|module| module.assessments.iter())
            .flat_map(requested_question_ids)
            .collect()
    }

    pub fn requested_question_ids_from_replace(
        input: &ReplaceBlueprintCourseContentInput,
    ) -> Vec<QuestionId> {
        input
            .modules
            .iter()
            .flat_map(|module| module.assessments.iter())
            .flat_map(|assessment| requested_question_ids(&assessment.content))
            .collect()
    }

    pub fn requested_pool_ids_from_create(input: &CreateBlueprintCourseInput) -> Vec<QuestionId> {
        input
            .modules
            .iter()
            .flat_map(|module| module.assessments.iter())
            .flat_map(requested_pool_ids)
            .collect()
    }

    pub fn requested_pool_ids_from_replace(
        input: &ReplaceBlueprintCourseContentInput,
    ) -> Vec<QuestionId> {
        input
            .modules
            .iter()
            .flat_map(|module| module.assessments.iter())
            .flat_map(|assessment| requested_pool_ids(&assessment.content))
            .collect()
    }

    /// Rebuilds the one canonical content checksum used by the new base schema.
    pub fn checksum(&self) -> Result<question_model::BlueprintContentChecksum, StoreError> {
        let modules = self
            .modules
            .iter()
            .map(|module| {
                let assessments = module
                    .assessments
                    .iter()
                    .map(StoredBlueprintAssessment::to_domain)
                    .collect::<Result<Vec<_>, StoreError>>()?;
                BlueprintCourseModuleContent::new(
                    module.blueprint_module_reference,
                    module.label.clone(),
                    assessments,
                )
                .map_err(invalid_content)
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let course = BlueprintCourseContent::new(modules).map_err(invalid_content)?;
        Ok(BlueprintRevisionContent::course(course).checksum())
    }

    fn replacement_assessment(
        assessment: question_model::BlueprintAssessmentReplacementInput,
        prior: &StoredBlueprintCourseContent,
        pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
        pool_revisions: &BTreeMap<QuestionId, question_model::QuestionPoolRevisionReference>,
    ) -> Result<StoredBlueprintAssessment, StoreError> {
        let reference = match assessment.choice {
            BlueprintAssessmentEditChoice::Retained {
                blueprint_assessment_reference,
            } => prior
                .modules
                .iter()
                .flat_map(|module| module.assessments.iter())
                .find(|candidate| {
                    candidate.blueprint_assessment_reference == blueprint_assessment_reference
                })
                .map(|candidate| candidate.blueprint_assessment_reference)
                .ok_or_else(|| invalid("retained Blueprint Assessment Reference"))?,
            BlueprintAssessmentEditChoice::New => {
                BlueprintAssessmentReference::from_uuid(random_uuid()?)
            }
        };
        Ok(StoredBlueprintAssessment {
            blueprint_assessment_reference: reference,
            content: StoredBlueprintAssessmentContent::from_input(
                assessment.content,
                pins,
                pool_revisions,
            )?,
        })
    }
}

impl StoredBlueprintAssessmentContent {
    fn from_input(
        input: BlueprintAssessmentContentInput,
        pins: &BTreeMap<QuestionId, QuestionRevisionReference>,
        pool_revisions: &BTreeMap<QuestionId, question_model::QuestionPoolRevisionReference>,
    ) -> Result<Self, StoreError> {
        input.validate().map_err(invalid_content)?;
        let entries = input
            .entries
            .into_iter()
            .map(|entry| match entry {
                question_model::BlueprintAssessmentEntryInput::Fixed(value) => {
                    Ok(StoredBlueprintAssessmentEntry::Fixed {
                        question_revision: pin(&value.question_id, pins)?,
                        points_possible: value.points_possible,
                        scoring_rule: value.scoring_rule,
                        question_attempt_limit: value.question_attempt_limit,
                        question_attempt_time_limit: value.question_attempt_time_limit,
                    })
                }
                question_model::BlueprintAssessmentEntryInput::Pool(value) => {
                    Ok(StoredBlueprintAssessmentEntry::Pool {
                        question_pool_revision: pool_revisions
                            .get(&value.question_pool_id)
                            .cloned()
                            .ok_or_else(|| invalid("current immutable Question Pool Revision"))?,
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
        })
    }

    fn to_domain(
        &self,
        blueprint_assessment_reference: BlueprintAssessmentReference,
    ) -> Result<BlueprintAssessmentContent, StoreError> {
        let entries = self
            .entries
            .iter()
            .map(|entry| match entry {
                StoredBlueprintAssessmentEntry::Fixed {
                    question_revision,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => Ok(BlueprintAssessmentEntryContent::Fixed {
                    reference: question_revision.clone(),
                    points_possible: *points_possible,
                    scoring_rule: *scoring_rule,
                    question_attempt_limit: *question_attempt_limit,
                    question_attempt_time_limit: *question_attempt_time_limit,
                }),
                StoredBlueprintAssessmentEntry::Pool {
                    question_pool_revision,
                    selection_count,
                    points_per_item,
                    scoring_rule,
                    selection_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => Ok(BlueprintAssessmentEntryContent::Pool(
                    BlueprintQuestionPoolContent::new(
                        question_pool_revision.clone(),
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
        BlueprintAssessmentContent::new(
            blueprint_assessment_reference,
            AssessmentTitle::try_new(self.title.clone())
                .map_err(|_| invalid("Blueprint Assessment title"))?,
            self.instructions.clone(),
            entries,
            self.defaults.clone(),
        )
        .map_err(invalid_content)
    }
}

impl StoredBlueprintAssessment {
    fn to_domain(&self) -> Result<BlueprintAssessmentContent, StoreError> {
        self.content.to_domain(self.blueprint_assessment_reference)
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
    /// Requests C49's Private-to-Public lifecycle transition for an owner.
    async fn publish_blueprint(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
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
    /// Requests C49's permitted Public-to-Private transition for an owner.
    /// The Store transaction denies it after the lineage has been adopted.
    async fn return_blueprint_to_private(
        &self,
        session: SessionTokenHash,
        reference: BlueprintCourseReference,
        expected_metadata_etag: BlueprintMetadataEtag,
    ) -> Result<BlueprintMetadataState, StoreError>;
}

fn requested_question_ids(input: &BlueprintAssessmentContentInput) -> Vec<QuestionId> {
    input
        .entries
        .iter()
        .flat_map(|entry| match entry {
            question_model::BlueprintAssessmentEntryInput::Fixed(value) => {
                vec![value.question_id.clone()]
            }
            question_model::BlueprintAssessmentEntryInput::Pool(_) => Vec::new(),
        })
        .collect()
}

fn requested_pool_ids(input: &BlueprintAssessmentContentInput) -> Vec<QuestionId> {
    input
        .entries
        .iter()
        .filter_map(|entry| match entry {
            question_model::BlueprintAssessmentEntryInput::Pool(value) => {
                Some(value.question_pool_id.clone())
            }
            question_model::BlueprintAssessmentEntryInput::Fixed(_) => None,
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
        AssessmentActivityRules, BlueprintAssessmentDefaults, LateWorkRule,
        StudentFeedbackReleaseRule,
    };
    use uuid::Uuid;

    fn content(assessment_identity: u128) -> StoredBlueprintCourseContent {
        StoredBlueprintCourseContent {
            modules: vec![StoredBlueprintModule {
                blueprint_module_reference: BlueprintModuleReference::from_uuid(Uuid::from_u128(1)),
                label: "Module".to_string(),
                assessments: vec![StoredBlueprintAssessment {
                    blueprint_assessment_reference: BlueprintAssessmentReference::from_uuid(
                        Uuid::from_u128(assessment_identity),
                    ),
                    content: StoredBlueprintAssessmentContent {
                        title: "Assessment".to_string(),
                        instructions: AssessmentInstructions::default(),
                        entries: vec![StoredBlueprintAssessmentEntry::Fixed {
                            question_revision: QuestionRevisionReference {
                                question_id: "7K3M-X9QX".parse().expect("Question ID"),
                                revision_number: question_model::QuestionRevisionNumber::new(1)
                                    .expect("revision"),
                            },
                            points_possible: AssessmentPointValue::from_whole(1),
                            scoring_rule: AssessmentEntryScoringRule::Normal,
                            question_attempt_limit: QuestionAttemptLimit { max_attempts: None },
                            question_attempt_time_limit: QuestionAttemptTimeLimit::Unlimited,
                        }],
                        defaults: BlueprintAssessmentDefaults {
                            assessment_attempt_time_limit_seconds: None,
                            attempt_limit: None,
                            late_work_rule: LateWorkRule::Accept,
                            activity_rules: AssessmentActivityRules::default(),
                            student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                        },
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
