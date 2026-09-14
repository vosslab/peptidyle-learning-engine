//! Validated reusable Blueprint Revision Content.

use std::collections::BTreeSet;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    AssignmentEntryScoringRule, AssignmentInstructions, AssignmentPointValue, AssignmentTitle,
    BlueprintAssignmentDefaults, BlueprintAssignmentReference, BlueprintCourseValidationError,
    BlueprintModuleReference, MAX_ASSIGNMENT_ORDERED_ENTRIES, MAX_ASSIGNMENT_QUESTION_POOL_ITEMS,
    MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY, QuestionAttemptLimit, QuestionAttemptTimeLimit,
    QuestionRevisionReference, validate_blueprint_course_title,
};

mod contracts;

pub use contracts::*;

const DOMAIN: &[u8] = b"ple:blueprint-revision-content\0";

#[derive(Debug, Clone, PartialEq)]
/// Validated Blueprint Revision Content stored independently from operation evidence.
pub enum BlueprintRevisionContent {
    /// One Blueprint Assignment Content record.
    Assignment(Box<BlueprintAssignmentContent>),
    /// One Blueprint Course Content record.
    Course(BlueprintCourseContent),
}

impl BlueprintRevisionContent {
    /// Wraps one validated Blueprint Assignment Content record.
    pub fn assignment(value: BlueprintAssignmentContent) -> Self {
        Self::Assignment(Box::new(value))
    }
    /// Wraps one validated Blueprint Course Content record.
    pub fn course(value: BlueprintCourseContent) -> Self {
        Self::Course(value)
    }
    /// Produces the one validated persistence record for this Blueprint Revision Content.
    pub fn encoding_record(&self) -> BlueprintRevisionContentRecord {
        let encoded_bytes = deterministic_encoded_bytes(self);
        let checksum = BlueprintContentChecksum(Sha256::digest(&encoded_bytes).into());
        BlueprintRevisionContentRecord {
            encoded_bytes,
            checksum,
        }
    }
    /// Computes the canonical Blueprint Revision Content checksum.
    pub fn checksum(&self) -> BlueprintContentChecksum {
        self.encoding_record().checksum()
    }
    /// Checks Blueprint Revision Content and reports both checksums when it changed.
    pub fn compare(&self, other: &Self) -> BlueprintContentCheck {
        if self == other {
            BlueprintContentCheck::Equivalent {
                checksum: self.checksum(),
            }
        } else {
            BlueprintContentCheck::Changed {
                expected: self.checksum(),
                actual: other.checksum(),
            }
        }
    }
}

/// One validated Blueprint Assignment with trusted immutable question pins.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintAssignmentContent {
    blueprint_assignment_reference: BlueprintAssignmentReference,
    title: AssignmentTitle,
    instructions: AssignmentInstructions,
    entries: Vec<BlueprintAssignmentEntryContent>,
    defaults: BlueprintAssignmentDefaults,
}
impl BlueprintAssignmentContent {
    /// Validates all Blueprint Assignment meaning before constructing a baseline.
    pub fn new(
        blueprint_assignment_reference: BlueprintAssignmentReference,
        title: AssignmentTitle,
        instructions: AssignmentInstructions,
        entries: Vec<BlueprintAssignmentEntryContent>,
        defaults: BlueprintAssignmentDefaults,
    ) -> Result<Self, BlueprintCourseValidationError> {
        if entries.is_empty() || entries.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidEntryCount);
        }
        defaults.validate()?;
        let total = entries
            .iter()
            .filter_map(|entry| match entry {
                BlueprintAssignmentEntryContent::Pool(pool) => Some(pool.items.len()),
                _ => None,
            })
            .try_fold(0_usize, |total, value| {
                total
                    .checked_add(value)
                    .ok_or(BlueprintCourseValidationError::TooManyQuestionPoolItems)
            })?;
        if total > MAX_ASSIGNMENT_QUESTION_POOL_ITEMS {
            return Err(BlueprintCourseValidationError::TooManyQuestionPoolItems);
        }
        Ok(Self {
            blueprint_assignment_reference,
            title,
            instructions,
            entries,
            defaults,
        })
    }
    /// Returns the stable Blueprint Assignment identity retained across Revisions.
    pub fn blueprint_assignment_reference(&self) -> BlueprintAssignmentReference {
        self.blueprint_assignment_reference
    }
    /// Returns the Blueprint Assignment title.
    pub fn title(&self) -> &str {
        self.title.as_str()
    }
    /// Returns the student-facing reusable instructions.
    pub fn instructions(&self) -> &AssignmentInstructions {
        &self.instructions
    }
    /// Returns the validated reusable policy defaults.
    pub fn defaults(&self) -> &BlueprintAssignmentDefaults {
        &self.defaults
    }
    /// Returns fixed questions and pools in meaningful authored order.
    pub fn entries(&self) -> &[BlueprintAssignmentEntryContent] {
        &self.entries
    }
}

/// One validated labelled module in Blueprint Course Content.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintCourseModuleContent {
    blueprint_module_reference: BlueprintModuleReference,
    label: String,
    assignments: Vec<BlueprintAssignmentContent>,
}
impl BlueprintCourseModuleContent {
    /// Validates a module label and its nonempty ordered assignments.
    pub fn new(
        blueprint_module_reference: BlueprintModuleReference,
        label: String,
        assignments: Vec<BlueprintAssignmentContent>,
    ) -> Result<Self, BlueprintCourseValidationError> {
        validate_blueprint_course_title(&label)
            .map_err(|_| BlueprintCourseValidationError::InvalidModuleLabel)?;
        if assignments.is_empty() || assignments.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleAssignmentCount);
        }
        Ok(Self {
            blueprint_module_reference,
            label,
            assignments,
        })
    }
    /// Returns the stable Blueprint Module identity retained across Revisions.
    pub fn blueprint_module_reference(&self) -> BlueprintModuleReference {
        self.blueprint_module_reference
    }
    /// Returns the reusable module label.
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Returns Blueprint Assignments in meaningful authored order.
    pub fn assignments(&self) -> &[BlueprintAssignmentContent] {
        &self.assignments
    }
}

/// One validated Blueprint Course Content record.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintCourseContent {
    modules: Vec<BlueprintCourseModuleContent>,
}
impl BlueprintCourseContent {
    /// Validates nonempty ordered reusable modules.
    pub fn new(
        modules: Vec<BlueprintCourseModuleContent>,
    ) -> Result<Self, BlueprintCourseValidationError> {
        if modules.is_empty() || modules.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleCount);
        }
        Ok(Self { modules })
    }
    /// Returns reusable modules in meaningful authored order.
    pub fn modules(&self) -> &[BlueprintCourseModuleContent] {
        &self.modules
    }
}

/// One ordered Blueprint Assignment entry containing only trusted exact pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintAssignmentEntryContent {
    /// One fixed immutable Question Revision and its scoring rule.
    Fixed {
        /// Exact immutable publication pin authorized for the destination.
        reference: QuestionRevisionReference,
        /// Exact points copied into the destination assignment.
        points_possible: AssignmentPointValue,
        /// Scoring treatment copied into the destination assignment.
        scoring_rule: AssignmentEntryScoringRule,
        /// Question Attempt retry bound copied into the destination assignment.
        question_attempt_limit: QuestionAttemptLimit,
        /// Question Attempt timing copied into the destination assignment.
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    },
    /// One validated deterministic item pool.
    Pool(BlueprintQuestionPoolContent),
}
/// One validated ordered pool of exact immutable publication pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintQuestionPoolContent {
    items: Vec<QuestionRevisionReference>,
    selection_count: u32,
    points_per_item: AssignmentPointValue,
    scoring_rule: AssignmentEntryScoringRule,
    selection_rule: crate::QuestionPoolSelectionRule,
    question_attempt_limit: QuestionAttemptLimit,
    question_attempt_time_limit: QuestionAttemptTimeLimit,
}
impl BlueprintQuestionPoolContent {
    /// Validates pool cardinality, uniqueness, and selection bounds.
    pub fn new(
        items: Vec<QuestionRevisionReference>,
        selection_count: u32,
        points_per_item: AssignmentPointValue,
        scoring_rule: AssignmentEntryScoringRule,
        selection_rule: crate::QuestionPoolSelectionRule,
        question_attempt_limit: QuestionAttemptLimit,
        question_attempt_time_limit: QuestionAttemptTimeLimit,
    ) -> Result<Self, BlueprintCourseValidationError> {
        if items.is_empty() || items.len() > MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY {
            return Err(BlueprintCourseValidationError::InvalidQuestionPoolItems);
        }
        if selection_count == 0 || usize::try_from(selection_count).ok() > Some(items.len()) {
            return Err(BlueprintCourseValidationError::InvalidPoolSelectionCount);
        }
        if items.iter().collect::<BTreeSet<_>>().len() != items.len() {
            return Err(BlueprintCourseValidationError::DuplicateQuestionPoolItem);
        }
        Ok(Self {
            items,
            selection_count,
            points_per_item,
            scoring_rule,
            selection_rule,
            question_attempt_limit,
            question_attempt_time_limit,
        })
    }
    /// Returns Question Pool Item pins in meaningful authored order.
    pub fn items(&self) -> &[QuestionRevisionReference] {
        &self.items
    }
    /// Returns the number of Question Pool Items selected for one Assignment Attempt.
    pub fn selection_count(&self) -> u32 {
        self.selection_count
    }
    /// Returns the point value assigned to every selected Question.
    pub fn points_per_item(&self) -> AssignmentPointValue {
        self.points_per_item
    }
    /// Returns the complete reviewed selection behavior.
    pub fn selection_rule(&self) -> crate::QuestionPoolSelectionRule {
        self.selection_rule
    }
    /// Returns the uniform Question Attempt retry bound copied to selected Questions.
    pub fn question_attempt_limit(&self) -> &QuestionAttemptLimit {
        &self.question_attempt_limit
    }
    /// Returns the uniform Question Attempt timing copied to selected Questions.
    pub fn question_attempt_time_limit(&self) -> &QuestionAttemptTimeLimit {
        &self.question_attempt_time_limit
    }
}
/// Full server-side SHA-256 binding of encoded Blueprint Revision Content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlueprintContentChecksum([u8; 32]);
impl BlueprintContentChecksum {
    /// Returns all persistence bytes without truncation.
    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Validated server-side encoded Blueprint Revision Content bytes and checksum.
///
/// Construction remains payload-owned so arbitrary persisted bytes cannot be
/// mistaken for normalized qmodel meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintRevisionContentRecord {
    encoded_bytes: Vec<u8>,
    checksum: BlueprintContentChecksum,
}

impl BlueprintRevisionContentRecord {
    /// Borrows the complete domain-separated Blueprint Revision Content encoding.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.encoded_bytes
    }

    /// Returns the complete SHA-256 checksum of `encoded_bytes`.
    pub const fn checksum(&self) -> BlueprintContentChecksum {
        self.checksum
    }
}
/// Blueprint Revision Content check used by fast-forward and divergence decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintContentCheck {
    /// Both Blueprint Revision Content values contain exactly the same content.
    Equivalent {
        /// Shared checksum recorded with the equivalent baseline.
        checksum: BlueprintContentChecksum,
    },
    /// At least one Blueprint Revision Content field differs.
    Changed {
        /// Checksum of the observed Blueprint Revision Content.
        expected: BlueprintContentChecksum,
        /// Checksum of the proposed or current Blueprint Revision Content.
        actual: BlueprintContentChecksum,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedPayload<'a> {
    meaning: EncodedMeaning<'a>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EncodedMeaning<'a> {
    Assignment { content: EncodedAssignment<'a> },
    Course { modules: Vec<EncodedModule<'a>> },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedModule<'a> {
    blueprint_module_reference: BlueprintModuleReference,
    label: &'a str,
    assignments: Vec<EncodedAssignment<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedAssignment<'a> {
    blueprint_assignment_reference: BlueprintAssignmentReference,
    title: &'a str,
    instructions: &'a AssignmentInstructions,
    entries: Vec<EncodedEntry<'a>>,
    defaults: &'a BlueprintAssignmentDefaults,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EncodedEntry<'a> {
    Fixed {
        reference: &'a QuestionRevisionReference,
        points_possible: AssignmentPointValue,
        scoring_rule: AssignmentEntryScoringRule,
        question_attempt_limit: &'a QuestionAttemptLimit,
        question_attempt_time_limit: &'a QuestionAttemptTimeLimit,
    },
    Pool {
        items: &'a [QuestionRevisionReference],
        selection_count: u32,
        points_per_item: AssignmentPointValue,
        scoring_rule: AssignmentEntryScoringRule,
        selection_rule: crate::QuestionPoolSelectionRule,
        question_attempt_limit: &'a QuestionAttemptLimit,
        question_attempt_time_limit: &'a QuestionAttemptTimeLimit,
    },
}

fn deterministic_encoded_bytes(payload: &BlueprintRevisionContent) -> Vec<u8> {
    let meaning = match payload {
        BlueprintRevisionContent::Assignment(assignment) => EncodedMeaning::Assignment {
            content: encode_assignment(assignment),
        },
        BlueprintRevisionContent::Course(course) => EncodedMeaning::Course {
            modules: course
                .modules()
                .iter()
                .map(|module| EncodedModule {
                    blueprint_module_reference: module.blueprint_module_reference(),
                    label: module.label(),
                    assignments: module.assignments().iter().map(encode_assignment).collect(),
                })
                .collect(),
        },
    };
    let json = serde_json::to_vec(&EncodedPayload { meaning })
        .expect("validated private Blueprint Revision Content serializes");
    let mut bytes = Vec::with_capacity(DOMAIN.len() + json.len());
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(&json);
    bytes
}

fn encode_assignment(assignment: &BlueprintAssignmentContent) -> EncodedAssignment<'_> {
    EncodedAssignment {
        blueprint_assignment_reference: assignment.blueprint_assignment_reference(),
        title: assignment.title(),
        instructions: assignment.instructions(),
        entries: assignment
            .entries()
            .iter()
            .map(|entry| match entry {
                BlueprintAssignmentEntryContent::Fixed {
                    reference,
                    points_possible,
                    scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                } => EncodedEntry::Fixed {
                    reference,
                    points_possible: *points_possible,
                    scoring_rule: *scoring_rule,
                    question_attempt_limit,
                    question_attempt_time_limit,
                },
                BlueprintAssignmentEntryContent::Pool(pool) => EncodedEntry::Pool {
                    items: &pool.items,
                    selection_count: pool.selection_count,
                    points_per_item: pool.points_per_item,
                    scoring_rule: pool.scoring_rule,
                    selection_rule: pool.selection_rule,
                    question_attempt_limit: &pool.question_attempt_limit,
                    question_attempt_time_limit: &pool.question_attempt_time_limit,
                },
            })
            .collect(),
        defaults: assignment.defaults(),
    }
}

#[cfg(test)]
mod wire_tests {
    use super::*;

    #[test]
    fn question_pool_keeps_the_exact_question_revision_pin() {
        let question_id: crate::QuestionId = "7K3-M9QX".parse().expect("Question ID");
        let pinned = QuestionRevisionReference {
            question_id: question_id.clone(),
            revision_number: crate::QuestionRevisionNumber::new(1).expect("revision"),
        };
        let newer_revision = QuestionRevisionReference {
            question_id,
            revision_number: crate::QuestionRevisionNumber::new(2).expect("revision"),
        };
        let pool = BlueprintQuestionPoolContent::new(
            vec![pinned.clone()],
            1,
            AssignmentPointValue::from_whole(1),
            AssignmentEntryScoringRule::Normal,
            crate::QuestionPoolSelectionRule {
                selected_question_order:
                    crate::QuestionPoolSelectedQuestionOrder::QuestionPoolOrder,
            },
            QuestionAttemptLimit { max_attempts: None },
            QuestionAttemptTimeLimit::Unlimited,
        )
        .expect("one exact Question Revision is a valid pool");

        assert_eq!(pool.items(), &[pinned]);
        assert_ne!(pool.items(), &[newer_revision]);
    }
}
