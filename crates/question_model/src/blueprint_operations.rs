//! Validated Blueprint Revision Content and target-term schedule resolution.

use std::collections::BTreeSet;
use std::num::NonZeroU64;
use std::str::FromStr;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AccountTimeZone, AssignmentAuthoredContentField, AssignmentAuthoredContentLocalError,
    AssignmentEntryScoringRule, AssignmentInstructions, AssignmentPointValue, AssignmentTitle,
    BaseAssignmentPolicy, BlueprintAssignmentDefaults, BlueprintCourseValidationError,
    CourseInstanceReference, CourseTerm, LocalDateAndTime, LocalTimeOfDay,
    MAX_ASSIGNMENT_ORDERED_ENTRIES, MAX_ASSIGNMENT_QUESTION_POOL_ITEMS,
    MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY, QuestionAttemptLimit, QuestionAttemptTimeLimit,
    QuestionRevisionReference, RelativeAssignmentSchedule, RelativeAssignmentScheduleMoment,
    Timestamp, validate_blueprint_course_title,
};

mod contracts;

pub use contracts::*;

const DOMAIN: &[u8] = b"ple:blueprint-revision-content\0";
/// Current normalized Blueprint Revision Content encoding version.
pub const BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION: u8 = 3;
const BLUEPRINT_REVISION_CONTENT_LEGACY_ENCODING_VERSION: u8 = 2;

#[derive(Debug, Clone, PartialEq)]
/// Validated Blueprint Revision Content stored independently from operation evidence.
pub enum BlueprintRevisionContent {
    /// One Blueprint Assignment Content record.
    Assignment(BlueprintAssignmentContent),
    /// One Blueprint Course Content record.
    Course(BlueprintCourseContent),
}

impl BlueprintRevisionContent {
    /// Wraps one validated Blueprint Assignment Content record.
    pub fn assignment(value: BlueprintAssignmentContent) -> Self {
        Self::Assignment(value)
    }
    /// Wraps one validated Blueprint Course Content record.
    pub fn course(value: BlueprintCourseContent) -> Self {
        Self::Course(value)
    }
    /// Returns the encoding version persisted beside this Blueprint Revision Content.
    pub const fn encoding_version(&self) -> u8 {
        BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION
    }
    /// Produces the one validated persistence record for this Blueprint Revision Content.
    pub fn encoding_record(&self) -> BlueprintRevisionContentRecord {
        self.encoding_record_for_version(BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION)
            .expect("the current Blueprint Revision Content encoding version is supported")
    }
    /// Computes the full versioned Blueprint Revision Content checksum.
    pub fn checksum(&self) -> BlueprintContentChecksum {
        self.encoding_record().checksum()
    }
    /// Computes one recognized historic or current persistence checksum.
    ///
    /// The storage adapter uses version two only to verify immutable records
    /// written before submitted-response timing became explicit. New records
    /// always use the current version returned by [`Self::encoding_version`].
    pub fn checksum_for_encoding_version(&self, version: u8) -> Option<BlueprintContentChecksum> {
        self.encoding_record_for_version(version)
            .map(|record| record.checksum())
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

    fn encoding_record_for_version(&self, version: u8) -> Option<BlueprintRevisionContentRecord> {
        let encoded_bytes = match version {
            BLUEPRINT_REVISION_CONTENT_LEGACY_ENCODING_VERSION => {
                deterministic_encoded_bytes_v2(self)
            }
            BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION => deterministic_encoded_bytes_v3(self),
            _ => return None,
        };
        let checksum = BlueprintContentChecksum(Sha256::digest(&encoded_bytes).into());
        Some(BlueprintRevisionContentRecord {
            version,
            encoded_bytes,
            checksum,
        })
    }
}

/// One validated Blueprint Assignment with trusted immutable question pins.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintAssignmentContent {
    title: AssignmentTitle,
    instructions: AssignmentInstructions,
    entries: Vec<BlueprintAssignmentEntryContent>,
    defaults: BlueprintAssignmentDefaults,
    schedule: RelativeAssignmentSchedule,
}
impl BlueprintAssignmentContent {
    /// Validates all Blueprint Assignment meaning before constructing a baseline.
    pub fn new(
        title: AssignmentTitle,
        instructions: AssignmentInstructions,
        entries: Vec<BlueprintAssignmentEntryContent>,
        defaults: BlueprintAssignmentDefaults,
        schedule: RelativeAssignmentSchedule,
    ) -> Result<Self, BlueprintCourseValidationError> {
        if entries.is_empty() || entries.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidEntryCount);
        }
        defaults.validate()?;
        schedule.validate()?;
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
            title,
            instructions,
            entries,
            defaults,
            schedule,
        })
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
    /// Returns the target-term-relative schedule defaults.
    pub fn schedule(&self) -> &RelativeAssignmentSchedule {
        &self.schedule
    }
}

/// One validated labelled module in Blueprint Course Content.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintCourseModuleContent {
    label: String,
    assignments: Vec<BlueprintAssignmentContent>,
}
impl BlueprintCourseModuleContent {
    /// Validates a module label and its nonempty ordered assignments.
    pub fn new(
        label: String,
        assignments: Vec<BlueprintAssignmentContent>,
    ) -> Result<Self, BlueprintCourseValidationError> {
        validate_blueprint_course_title(&label)
            .map_err(|_| BlueprintCourseValidationError::InvalidModuleLabel)?;
        if assignments.is_empty() || assignments.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleAssignmentCount);
        }
        Ok(Self { label, assignments })
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
    title: String,
    modules: Vec<BlueprintCourseModuleContent>,
}
impl BlueprintCourseContent {
    /// Validates a reusable course title and its nonempty ordered modules.
    pub fn new(
        title: String,
        modules: Vec<BlueprintCourseModuleContent>,
    ) -> Result<Self, BlueprintCourseValidationError> {
        validate_blueprint_course_title(&title)
            .map_err(|_| BlueprintCourseValidationError::InvalidBlueprintTitle)?;
        if modules.is_empty() || modules.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(BlueprintCourseValidationError::InvalidModuleCount);
        }
        Ok(Self { title, modules })
    }
    /// Returns the reusable course title.
    pub fn title(&self) -> &str {
        &self.title
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
    version: u8,
    encoded_bytes: Vec<u8>,
    checksum: BlueprintContentChecksum,
}

impl BlueprintRevisionContentRecord {
    /// Returns the encoding version included in the hashed bytes.
    pub const fn version(&self) -> u8 {
        self.version
    }

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
    version: u8,
    meaning: EncodedMeaning<'a>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EncodedMeaning<'a> {
    Assignment {
        content: EncodedAssignment<'a>,
    },
    Course {
        title: &'a str,
        modules: Vec<EncodedModule<'a>>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedModule<'a> {
    label: &'a str,
    assignments: Vec<EncodedAssignment<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedAssignment<'a> {
    title: &'a str,
    instructions: &'a AssignmentInstructions,
    entries: Vec<EncodedEntry<'a>>,
    defaults: &'a BlueprintAssignmentDefaults,
    schedule: &'a RelativeAssignmentSchedule,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedPayloadV2<'a> {
    version: u8,
    meaning: EncodedMeaningV2<'a>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum EncodedMeaningV2<'a> {
    Assignment {
        content: EncodedAssignmentV2<'a>,
    },
    Course {
        title: &'a str,
        modules: Vec<EncodedModuleV2<'a>>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedModuleV2<'a> {
    label: &'a str,
    assignments: Vec<EncodedAssignmentV2<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedAssignmentV2<'a> {
    title: &'a str,
    instructions: &'a AssignmentInstructions,
    entries: Vec<EncodedEntry<'a>>,
    defaults: EncodedBlueprintAssignmentDefaultsV2<'a>,
    schedule: &'a RelativeAssignmentSchedule,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedBlueprintAssignmentDefaultsV2<'a> {
    assignment_attempt_time_limit_seconds: &'a Option<std::num::NonZeroU32>,
    attempt_limit: &'a Option<std::num::NonZeroU32>,
    late_work_rule: crate::LateWorkRule,
    assignment_deadline_rule: crate::AssignmentDeadlineRule,
    activity_rules: &'a crate::AssignmentActivityRules,
    student_feedback_release_rule: EncodedStudentFeedbackReleaseRuleV2,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct EncodedStudentFeedbackReleaseRuleV2 {
    score: crate::StudentFeedbackReleaseTiming,
    per_item_correctness: crate::StudentFeedbackReleaseTiming,
    question_feedback: crate::StudentFeedbackReleaseTiming,
    question_answer: crate::StudentFeedbackReleaseTiming,
    question_answer_explanation: crate::StudentFeedbackReleaseTiming,
    class_statistics: crate::StudentFeedbackReleaseTiming,
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

fn deterministic_encoded_bytes_v3(payload: &BlueprintRevisionContent) -> Vec<u8> {
    let meaning = match payload {
        BlueprintRevisionContent::Assignment(assignment) => EncodedMeaning::Assignment {
            content: encode_assignment(assignment),
        },
        BlueprintRevisionContent::Course(course) => EncodedMeaning::Course {
            title: course.title(),
            modules: course
                .modules()
                .iter()
                .map(|module| EncodedModule {
                    label: module.label(),
                    assignments: module.assignments().iter().map(encode_assignment).collect(),
                })
                .collect(),
        },
    };
    let json = serde_json::to_vec(&EncodedPayload {
        version: BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION,
        meaning,
    })
    .expect("validated private Blueprint Revision Content serializes");
    let mut bytes = Vec::with_capacity(DOMAIN.len() + json.len());
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(&json);
    bytes
}

fn deterministic_encoded_bytes_v2(payload: &BlueprintRevisionContent) -> Vec<u8> {
    let meaning = match payload {
        BlueprintRevisionContent::Assignment(assignment) => EncodedMeaningV2::Assignment {
            content: encode_assignment_v2(assignment),
        },
        BlueprintRevisionContent::Course(course) => EncodedMeaningV2::Course {
            title: course.title(),
            modules: course
                .modules()
                .iter()
                .map(|module| EncodedModuleV2 {
                    label: module.label(),
                    assignments: module
                        .assignments()
                        .iter()
                        .map(encode_assignment_v2)
                        .collect(),
                })
                .collect(),
        },
    };
    let json = serde_json::to_vec(&EncodedPayloadV2 {
        version: BLUEPRINT_REVISION_CONTENT_LEGACY_ENCODING_VERSION,
        meaning,
    })
    .expect("validated private Blueprint Revision Content serializes");
    let mut bytes = Vec::with_capacity(DOMAIN.len() + json.len());
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(&json);
    bytes
}

fn encode_assignment(assignment: &BlueprintAssignmentContent) -> EncodedAssignment<'_> {
    EncodedAssignment {
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
        schedule: assignment.schedule(),
    }
}

fn encode_assignment_v2(assignment: &BlueprintAssignmentContent) -> EncodedAssignmentV2<'_> {
    let feedback = assignment.defaults().student_feedback_release_rule;
    EncodedAssignmentV2 {
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
        defaults: EncodedBlueprintAssignmentDefaultsV2 {
            assignment_attempt_time_limit_seconds: &assignment
                .defaults()
                .assignment_attempt_time_limit_seconds,
            attempt_limit: &assignment.defaults().attempt_limit,
            late_work_rule: assignment.defaults().late_work_rule,
            assignment_deadline_rule: assignment.defaults().assignment_deadline_rule,
            activity_rules: &assignment.defaults().activity_rules,
            student_feedback_release_rule: EncodedStudentFeedbackReleaseRuleV2 {
                score: feedback.score,
                per_item_correctness: feedback.per_item_correctness,
                question_feedback: feedback.question_feedback,
                question_answer: feedback.question_answer,
                question_answer_explanation: feedback.question_answer_explanation,
                class_statistics: feedback.class_statistics,
            },
        },
        schedule: assignment.schedule(),
    }
}

/// Positive revision number within one Course Instance's schedule history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseScheduleRevisionNumber(NonZeroU64);
impl CourseScheduleRevisionNumber {
    /// Rebuilds a positive PostgreSQL-`BIGINT` revision.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }
    /// Returns the exact positive revision scalar.
    pub fn value(self) -> u64 {
        self.0.get()
    }
}
impl FromStr for CourseScheduleRevisionNumber {
    type Err = CourseScheduleRevisionNumberError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(CourseScheduleRevisionNumberError);
        }
        value
            .parse()
            .ok()
            .and_then(Self::new)
            .ok_or(CourseScheduleRevisionNumberError)
    }
}
impl TryFrom<String> for CourseScheduleRevisionNumber {
    type Error = CourseScheduleRevisionNumberError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl From<CourseScheduleRevisionNumber> for String {
    fn from(value: CourseScheduleRevisionNumber) -> Self {
        value.value().to_string()
    }
}
impl std::fmt::Display for CourseScheduleRevisionNumber {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}
/// A Course Schedule Revision Number was not one canonical positive decimal value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseScheduleRevisionNumberError;

impl std::fmt::Display for CourseScheduleRevisionNumberError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("course schedule revision must be a canonical positive decimal")
    }
}

impl std::error::Error for CourseScheduleRevisionNumberError {}

/// Exact immutable Course Schedule Revision Reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseScheduleRevisionReference {
    pub course: CourseInstanceReference,
    pub revision_number: CourseScheduleRevisionNumber,
}

impl CourseScheduleRevisionReference {
    /// Binds one positive schedule revision number to its exact Course Instance.
    pub const fn new(
        course: CourseInstanceReference,
        revision_number: CourseScheduleRevisionNumber,
    ) -> Self {
        Self {
            course,
            revision_number,
        }
    }
}

/// One relative moment resolved in the acting Account zone and as an absolute time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ResolvedAssignmentScheduleMoment {
    /// Exact wall-clock value in the acting Account's authorized zone.
    pub local: LocalDateAndTime,
    /// Server-resolved absolute timestamp persisted by teaching state.
    pub timestamp: Timestamp,
}
/// Complete answer-free target-term schedule preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ResolvedAssignmentSchedule {
    /// Resolved first student-availability moment when configured.
    pub available_at: Option<ResolvedAssignmentScheduleMoment>,
    /// Resolved ordinary due moment when configured.
    pub due_at: Option<ResolvedAssignmentScheduleMoment>,
    /// Resolved hard close moment when configured.
    pub closes_at: Option<ResolvedAssignmentScheduleMoment>,
}
impl RelativeAssignmentSchedule {
    /// Projects a stored policy into reusable calendar-relative meaning in the acting Account zone.
    /// Each absolute timestamp must project to an inclusive source-term local date,
    /// and the resulting related moments must remain chronological. This gives
    /// adoption callers a typed correction rather than discarding a stored date
    /// when a course is shifted. (ASVS 2.2.1, 2.2.2, 2.2.3)
    pub fn from_base_policy(
        policy: &BaseAssignmentPolicy,
        source_term: &CourseTerm,
        account_time_zone: &AccountTimeZone,
    ) -> Result<Self, AssignmentAuthoredContentLocalError> {
        let schedule = Self {
            available_at: project_relative_moment(
                policy.available_at,
                source_term,
                account_time_zone,
                AssignmentAuthoredContentField::AvailableAt,
            )?,
            due_at: project_relative_moment(
                policy.due_at,
                source_term,
                account_time_zone,
                AssignmentAuthoredContentField::DueAt,
            )?,
            closes_at: project_relative_moment(
                policy.closes_at,
                source_term,
                account_time_zone,
                AssignmentAuthoredContentField::ClosesAt,
            )?,
        };
        schedule
            .validate()
            .map_err(|_| AssignmentAuthoredContentLocalError::ScheduleOutOfOrder)?;
        Ok(schedule)
    }

    /// Resolves calendar offsets in the acting Account's IANA zone.
    pub fn resolve_for_target_term(
        &self,
        term: &CourseTerm,
        account_time_zone: &AccountTimeZone,
    ) -> Result<ResolvedAssignmentSchedule, AssignmentAuthoredContentLocalError> {
        self.validate()
            .map_err(|_| AssignmentAuthoredContentLocalError::ScheduleOutOfOrder)?;
        Ok(ResolvedAssignmentSchedule {
            available_at: resolve(
                self.available_at.as_ref(),
                term,
                account_time_zone,
                AssignmentAuthoredContentField::AvailableAt,
            )?,
            due_at: resolve(
                self.due_at.as_ref(),
                term,
                account_time_zone,
                AssignmentAuthoredContentField::DueAt,
            )?,
            closes_at: resolve(
                self.closes_at.as_ref(),
                term,
                account_time_zone,
                AssignmentAuthoredContentField::ClosesAt,
            )?,
        })
    }
}

fn project_relative_moment(
    value: Option<Timestamp>,
    source_term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
    field: AssignmentAuthoredContentField,
) -> Result<Option<RelativeAssignmentScheduleMoment>, AssignmentAuthoredContentLocalError> {
    value
        .map(|value| {
            let local = LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                value,
                source_term,
                account_time_zone,
                field,
            )?;
            let date = NaiveDate::parse_from_str(&local.as_str()[..10], "%Y-%m-%d")
                .expect("validated Account-local date");
            let start = NaiveDate::parse_from_str(source_term.start_date().as_str(), "%Y-%m-%d")
                .expect("validated course term");
            let day_offset = i32::try_from(date.signed_duration_since(start).num_days())
                .map_err(|_| AssignmentAuthoredContentLocalError::TimestampOutOfRange(field))?;
            let local_time =
                LocalTimeOfDay::parse(&local.as_str()[11..]).expect("validated Account-local time");
            Ok(RelativeAssignmentScheduleMoment {
                day_offset,
                local_time,
            })
        })
        .transpose()
}
fn resolve(
    value: Option<&RelativeAssignmentScheduleMoment>,
    term: &CourseTerm,
    account_time_zone: &AccountTimeZone,
    field: AssignmentAuthoredContentField,
) -> Result<Option<ResolvedAssignmentScheduleMoment>, AssignmentAuthoredContentLocalError> {
    value
        .map(|value| {
            let start = NaiveDate::parse_from_str(term.start_date().as_str(), "%Y-%m-%d")
                .expect("valid term");
            let date = start
                .checked_add_signed(Duration::days(i64::from(value.day_offset)))
                .ok_or(AssignmentAuthoredContentLocalError::TimestampOutOfRange(
                    field,
                ))?;
            let local = LocalDateAndTime::parse(&format!(
                "{}T{}",
                date.format("%Y-%m-%d"),
                value.local_time.as_str()
            ))
            .map_err(|_| AssignmentAuthoredContentLocalError::TimestampOutOfRange(field))?;
            Ok(ResolvedAssignmentScheduleMoment {
                timestamp: local.resolve_in_account_time_zone(term, account_time_zone, field)?,
                local,
            })
        })
        .transpose()
}

#[cfg(test)]
mod wire_tests {
    use super::*;

    #[test]
    fn resolved_schedule_uses_snake_case_and_refuses_unknown_fields() {
        let schedule = RelativeAssignmentSchedule {
            available_at: Some(RelativeAssignmentScheduleMoment {
                day_offset: 0,
                local_time: LocalTimeOfDay::parse("08:00:00.000").expect("time"),
            }),
            due_at: None,
            closes_at: None,
        };
        let term = CourseTerm::from_parts("2026-08-24", "2026-12-12").expect("term");
        let account_time_zone = AccountTimeZone::parse("America/Chicago").expect("zone");
        let resolved = schedule
            .resolve_for_target_term(&term, &account_time_zone)
            .expect("resolved schedule");
        let wire = serde_json::to_value(&resolved).expect("schedule serializes");
        assert!(wire.get("time_zone").is_none());
        assert!(wire.get("available_at").is_some());
        assert!(wire.get("timeZone").is_none());
        let mut forged = wire;
        forged["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<ResolvedAssignmentSchedule>(forged).is_err());
    }
}
