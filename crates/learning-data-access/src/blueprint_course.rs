//! Session-authorized persistence contracts for Blueprint Revisions and lineage metadata.

use std::collections::BTreeMap;

use async_trait::async_trait;
use question_model::{
    AssessmentEntryScoringRule, AssessmentInstructions, AssessmentPointValue, AssessmentTitle,
    BlueprintAssessmentContent, BlueprintAssessmentContentInput, BlueprintAssessmentEditChoice,
    BlueprintAssessmentEntryContent, BlueprintAssessmentId, BlueprintAvailability,
    BlueprintCourseContent, BlueprintCourseId, BlueprintCourseModuleContent,
    BlueprintCourseReadAccess, BlueprintCourseValidationError, BlueprintEditNumber,
    BlueprintMetadataState, BlueprintModuleEditChoice, BlueprintModuleId,
    BlueprintQuestionPoolContent, BlueprintRevisionContent, BlueprintRevisionNumber,
    BlueprintRevisionTuple, CanonicalBlueprintCourse, CreateBlueprintCourseInput,
    CreateBlueprintCourseReceipt, QuestionAttemptLimit, QuestionAttemptTimeLimit, QuestionId,
    QuestionPoolSelectionRule, QuestionRevisionTuple, RenameBlueprintCourseInput,
    ReplaceBlueprintCourseContentInput, RequestChecksum, ReusablePoolView,
    SaveBlueprintCourseReceipt,
};
use serde::{Deserialize, Serialize};

use crate::{Page, PageRequest, SessionTokenHash, StoreError};

/// Bounded ordinary discovery, including optional literal name narrowing.
#[derive(Debug, Clone)]
pub struct BlueprintCourseListRequest {
    pub page: PageRequest,
    pub query: String,
    pub include_archived: bool,
    pub public_only: bool,
    pub promoted_only: bool,
    pub discipline_uuid: Option<uuid::Uuid>,
    pub subject_uuid: Option<uuid::Uuid>,
    pub topic_uuid: Option<uuid::Uuid>,
    pub subtopic_uuid: Option<uuid::Uuid>,
    pub cross_discipline: bool,
}

/// Sysadmin-only projection; promotion is lineage metadata, not Revision content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredBlueprintPromotion {
    pub promoted: bool,
    pub blueprint_edit_number: BlueprintEditNumber,
}

/// Explicit reviewed choices; content and names are always read by the Store.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyBlueprintForkInput {
    pub expected_source: BlueprintRevisionTuple,
    pub expected_fork: BlueprintRevisionTuple,
    pub expected_source_blueprint_edit_number: BlueprintEditNumber,
    pub expected_fork_blueprint_edit_number: BlueprintEditNumber,
    pub source_short_name: bool,
    pub source_long_name: bool,
    pub selection: question_model::blueprint_course::BlueprintForkApplySelection,
}

/// Ordinary Save and metadata results committed in one operation.
#[derive(Debug, Clone)]
pub struct ApplyBlueprintForkResult {
    pub save: SaveBlueprintCourseReceipt,
    pub metadata: BlueprintMetadataState,
}

/// One current readable Blueprint lineage and the content selected by its read boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBlueprintCourse {
    pub classification: question_model::CourseClassification,
    pub id: BlueprintCourseId,
    pub short_name: String,
    pub long_name: String,
    pub availability: BlueprintAvailability,
    pub blueprint_edit_number: BlueprintEditNumber,
    pub current_revision_number: BlueprintRevisionNumber,
    /// Exact immutable ancestry, filtered by current source visibility.
    pub fork_source_tuple: Option<BlueprintRevisionTuple>,
    pub read_access: BlueprintCourseReadAccess,
    /// The exact current immutable Revision content.
    pub content: StoredBlueprintCourseContent,
}

/// Compact answer-free readable Blueprint lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBlueprintCourseSummary {
    pub classification: question_model::CourseClassification,
    pub id: BlueprintCourseId,
    pub short_name: String,
    pub long_name: String,
    pub availability: BlueprintAvailability,
    pub blueprint_edit_number: BlueprintEditNumber,
    pub current_revision_number: BlueprintRevisionNumber,
    pub read_access: BlueprintCourseReadAccess,
    /// Lifetime Course Instances adopted from this Blueprint lineage, across Revisions.
    pub total_adoptions: u64,
    /// Students counted once per adopted Course Instance, including ended memberships.
    pub total_students_ever_enrolled: u64,
}

/// Exact immutable Blueprint Revision content, including after lineage archive.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredBlueprintRevision {
    pub blueprint_revision_tuple: BlueprintRevisionTuple,
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
    pub blueprint_module_id: BlueprintModuleId,
    pub label: String,
    pub assessments: Vec<StoredBlueprintAssessment>,
}

/// One retained Blueprint Assessment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintAssessment {
    pub blueprint_assessment_id: BlueprintAssessmentId,
    pub content: StoredBlueprintAssessmentContent,
}

/// Reusable assessment content after Question IDs become exact Revision pins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct StoredBlueprintAssessmentContent {
    pub assessment_type: question_model::AssessmentType,
    pub title: String,
    pub instructions: AssessmentInstructions,
    pub entries: Vec<StoredBlueprintAssessmentEntry>,
    pub defaults: question_model::BlueprintAssessmentDefaults,
}

/// Stored entry retaining exact immutable Question Revision Tuples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoredBlueprintAssessmentEntry {
    Fixed {
        question_revision_tuple: QuestionRevisionTuple,
        points_possible: AssessmentPointValue,
        scoring_rule: AssessmentEntryScoringRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    Pool {
        question_pool_id: question_model::QuestionId,
        question_pool_edit_number: question_model::QuestionPoolEditNumber,
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
        pool_edit_numbers: &BTreeMap<QuestionId, question_model::QuestionPoolEditNumber>,
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
                            blueprint_assessment_id: BlueprintAssessmentId::from_uuid(
                                random_uuid()?
                            ),
                            content: StoredBlueprintAssessmentContent::from_input(
                                content,
                                pool_edit_numbers,
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?;
                Ok(StoredBlueprintModule {
                    blueprint_module_id: BlueprintModuleId::from_uuid(random_uuid()?),
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
        pool_edit_numbers: &BTreeMap<QuestionId, question_model::QuestionPoolEditNumber>,
    ) -> Result<Self, StoreError> {
        input.validate().map_err(invalid_content)?;
        let modules = input
            .modules
            .into_iter()
            .map(|module| {
                let module_id = match module.choice {
                    BlueprintModuleEditChoice::Retained {
                        blueprint_module_id,
                    } => prior
                        .modules
                        .iter()
                        .find(|candidate| candidate.blueprint_module_id == blueprint_module_id)
                        .map(|candidate| candidate.blueprint_module_id)
                        .ok_or_else(|| invalid("retained Blueprint Module ID"))?,
                    BlueprintModuleEditChoice::New => BlueprintModuleId::from_uuid(random_uuid()?),
                };
                let assessments = module
                    .assessments
                    .into_iter()
                    .map(|assessment| {
                        Self::replacement_assessment(assessment, prior, pool_edit_numbers)
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?;
                Ok(StoredBlueprintModule {
                    blueprint_module_id: module_id,
                    label: module.label,
                    assessments,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let content = Self { modules };
        content.checksum()?;
        Ok(content)
    }

    pub fn requested_question_revisions_from_create(
        input: &CreateBlueprintCourseInput,
    ) -> Vec<QuestionRevisionTuple> {
        input
            .modules
            .iter()
            .flat_map(|module| module.assessments.iter())
            .flat_map(requested_question_revisions)
            .collect()
    }

    pub fn requested_question_revisions_from_replace(
        input: &ReplaceBlueprintCourseContentInput,
    ) -> Vec<QuestionRevisionTuple> {
        input
            .modules
            .iter()
            .flat_map(|module| module.assessments.iter())
            .flat_map(|assessment| requested_question_revisions(&assessment.content))
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
        Ok(BlueprintRevisionContent::course(self.to_domain()?).checksum())
    }

    /// Reconstructs exact reusable content without current Question metadata enrichment.
    pub fn to_domain(&self) -> Result<BlueprintCourseContent, StoreError> {
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
                    module.blueprint_module_id,
                    module.label.clone(),
                    assessments,
                )
                .map_err(invalid_content)
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        BlueprintCourseContent::new(modules).map_err(invalid_content)
    }

    fn replacement_assessment(
        assessment: question_model::BlueprintAssessmentReplacementInput,
        prior: &StoredBlueprintCourseContent,
        pool_edit_numbers: &BTreeMap<QuestionId, question_model::QuestionPoolEditNumber>,
    ) -> Result<StoredBlueprintAssessment, StoreError> {
        let blueprint_assessment_id = match assessment.choice {
            BlueprintAssessmentEditChoice::Retained {
                blueprint_assessment_id,
            } => prior
                .modules
                .iter()
                .flat_map(|module| module.assessments.iter())
                .find(|candidate| candidate.blueprint_assessment_id == blueprint_assessment_id)
                .map(|candidate| candidate.blueprint_assessment_id)
                .ok_or_else(|| invalid("retained Blueprint Assessment ID"))?,
            BlueprintAssessmentEditChoice::New => BlueprintAssessmentId::from_uuid(random_uuid()?),
        };
        Ok(StoredBlueprintAssessment {
            blueprint_assessment_id,
            content: StoredBlueprintAssessmentContent::from_input(
                assessment.content,
                pool_edit_numbers,
            )?,
        })
    }
}

impl StoredBlueprintAssessmentContent {
    fn from_input(
        input: BlueprintAssessmentContentInput,
        _pool_edit_numbers: &BTreeMap<QuestionId, question_model::QuestionPoolEditNumber>,
    ) -> Result<Self, StoreError> {
        input.validate().map_err(invalid_content)?;
        let entries = input
            .entries
            .into_iter()
            .map(|entry| match entry {
                question_model::BlueprintAssessmentEntryInput::Fixed(value) => {
                    Ok(StoredBlueprintAssessmentEntry::Fixed {
                        question_revision_tuple: value.question_revision_tuple,
                        points_possible: value.points_possible,
                        scoring_rule: value.scoring_rule,
                        question_attempt_limit: value.question_attempt_limit,
                        question_attempt_time_limit: value.question_attempt_time_limit,
                    })
                }
                question_model::BlueprintAssessmentEntryInput::Pool(value) => {
                    Ok(StoredBlueprintAssessmentEntry::Pool {
                        question_pool_id: match &value.pool {
                            question_model::BlueprintPoolInputChoice::Import {
                                question_pool_id,
                                ..
                            }
                            | question_model::BlueprintPoolInputChoice::Retained {
                                question_pool_id,
                                ..
                            } => question_pool_id.clone(),
                        },
                        question_pool_edit_number: match &value.pool {
                            question_model::BlueprintPoolInputChoice::Import {
                                question_pool_edit_number,
                                ..
                            }
                            | question_model::BlueprintPoolInputChoice::Retained {
                                question_pool_edit_number,
                                ..
                            } => *question_pool_edit_number,
                        },
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
            assessment_type: input.assessment_type,
            title: input.title,
            instructions: input.instructions,
            entries,
            defaults: input.defaults,
        })
    }

    fn to_domain(
        &self,
        blueprint_assessment_id: BlueprintAssessmentId,
    ) -> Result<BlueprintAssessmentContent, StoreError> {
        let entries = self
            .entries
            .iter()
            .map(|entry| match entry {
                StoredBlueprintAssessmentEntry::Fixed {
                    question_revision_tuple,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => Ok(BlueprintAssessmentEntryContent::Fixed {
                    question_revision_tuple: question_revision_tuple.clone(),
                    points_possible: *points_possible,
                    scoring_rule: *scoring_rule,
                    question_attempt_limit: *question_attempt_limit,
                    question_attempt_time_limit: *question_attempt_time_limit,
                }),
                StoredBlueprintAssessmentEntry::Pool {
                    question_pool_id,
                    question_pool_edit_number,
                    selection_count,
                    points_per_item,
                    scoring_rule,
                    selection_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => Ok(BlueprintAssessmentEntryContent::Pool(
                    BlueprintQuestionPoolContent::new(ReusablePoolView {
                        question_pool_id: question_pool_id.clone(),
                        question_pool_edit_number: *question_pool_edit_number,
                        selection_count: *selection_count,
                        points_per_item: *points_per_item,
                        scoring_rule: *scoring_rule,
                        selection_rule: *selection_rule,
                        question_attempt_limit: *question_attempt_limit,
                        question_attempt_time_limit: *question_attempt_time_limit,
                    })
                    .map_err(invalid_content)?,
                )),
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        BlueprintAssessmentContent::new(
            blueprint_assessment_id,
            self.assessment_type,
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
        self.content.to_domain(self.blueprint_assessment_id)
    }
}

/// Store boundary for immutable Blueprint Revisions and lineage metadata.
#[derive(Debug, Clone)]
pub struct StoredBlueprintPoolMembers {
    pub question_pool_id: question_model::QuestionId,
    pub question_pool_edit_number: question_model::QuestionPoolEditNumber,
    pub members: Vec<QuestionRevisionTuple>,
}

#[async_trait]
pub trait BlueprintPromotionStore: Send + Sync {
    async fn load_blueprint_promotion(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<StoredBlueprintPromotion, StoreError>;
    async fn set_blueprint_promotion(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        promoted: bool,
    ) -> Result<StoredBlueprintPromotion, StoreError>;
}

#[async_trait]
pub trait BlueprintCourseStore: Send + Sync {
    async fn update_blueprint_classification(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        classification: question_model::CourseClassification,
    ) -> Result<BlueprintMetadataState, StoreError>;
    async fn load_blueprint_pool_members(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        assessment: BlueprintAssessmentId,
        question_pool_id: QuestionId,
    ) -> Result<StoredBlueprintPoolMembers, StoreError>;
    async fn apply_blueprint_fork(
        &self,
        session: SessionTokenHash,
        input: ApplyBlueprintForkInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<ApplyBlueprintForkResult, StoreError>;
    async fn list_blueprint_courses(
        &self,
        session: SessionTokenHash,
        request: BlueprintCourseListRequest,
    ) -> Result<Page<StoredBlueprintCourseSummary>, StoreError>;
    async fn load_blueprint_course(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<StoredBlueprintCourse, StoreError>;
    async fn load_blueprint_revision(
        &self,
        session: SessionTokenHash,
        blueprint_revision_tuple: BlueprintRevisionTuple,
    ) -> Result<StoredBlueprintRevision, StoreError>;
    /// Reads the current authorized Blueprint as complete reusable exchange data.
    async fn export_blueprint_course(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
    ) -> Result<CanonicalBlueprintCourse, StoreError>;
    /// Creates a distinct actor-owned Private Blueprint through the ordinary
    /// Revision 1 transaction, allocating fresh child and Pool identities.
    async fn import_blueprint_course(
        &self,
        session: SessionTokenHash,
        request_checksum: RequestChecksum,
        exchange: CanonicalBlueprintCourse,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<CreateBlueprintCourseReceipt, StoreError>;
    async fn create_blueprint_course(
        &self,
        session: SessionTokenHash,
        request_checksum: RequestChecksum,
        input: CreateBlueprintCourseInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<CreateBlueprintCourseReceipt, StoreError>;
    async fn save_blueprint_course(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        expected_revision_number: BlueprintRevisionNumber,
        request_checksum: RequestChecksum,
        input: ReplaceBlueprintCourseContentInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<SaveBlueprintCourseReceipt, StoreError>;
    async fn rename_blueprint_course(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        input: RenameBlueprintCourseInput,
    ) -> Result<BlueprintMetadataState, StoreError>;
    /// Requests C49's Private-to-Public lifecycle transition for an owner.
    async fn publish_blueprint(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
    ) -> Result<BlueprintMetadataState, StoreError>;
    async fn archive_blueprint(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        confirmation_title: &str,
    ) -> Result<BlueprintMetadataState, StoreError>;
    async fn restore_blueprint(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
    ) -> Result<BlueprintMetadataState, StoreError>;
    /// Requests C49's permitted Public-to-Private transition for an owner.
    /// The Store transaction denies it after the lineage has been adopted.
    async fn return_blueprint_to_private(
        &self,
        session: SessionTokenHash,
        blueprint_course_id: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
    ) -> Result<BlueprintMetadataState, StoreError>;
}

fn requested_question_revisions(
    input: &BlueprintAssessmentContentInput,
) -> Vec<QuestionRevisionTuple> {
    input
        .entries
        .iter()
        .flat_map(|entry| match entry {
            question_model::BlueprintAssessmentEntryInput::Fixed(value) => {
                vec![value.question_revision_tuple.clone()]
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
            question_model::BlueprintAssessmentEntryInput::Pool(value) => match &value.pool {
                question_model::BlueprintPoolInputChoice::Import {
                    question_pool_id, ..
                }
                | question_model::BlueprintPoolInputChoice::Retained {
                    question_pool_id, ..
                } => Some(question_pool_id.clone()),
            },
            question_model::BlueprintAssessmentEntryInput::Fixed(_) => None,
        })
        .collect()
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
                blueprint_module_id: BlueprintModuleId::from_uuid(Uuid::from_u128(1)),
                label: "Module".to_string(),
                assessments: vec![StoredBlueprintAssessment {
                    blueprint_assessment_id: BlueprintAssessmentId::from_uuid(Uuid::from_u128(
                        assessment_identity,
                    )),
                    content: StoredBlueprintAssessmentContent {
                        assessment_type: question_model::AssessmentType::RegularAssignment,
                        title: "Assessment".to_string(),
                        instructions: AssessmentInstructions::default(),
                        entries: vec![StoredBlueprintAssessmentEntry::Fixed {
                            question_revision_tuple: QuestionRevisionTuple {
                                question_id: "7K3M-19QX".parse().expect("Question ID"),
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
