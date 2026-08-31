//! Validated Blueprint Revision Content and target-term schedule projection.

use std::collections::BTreeSet;
use std::num::NonZeroU64;
use std::str::FromStr;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ActivityTimestamp, AssignmentEntryScoringRule, AssignmentInstructions, AssignmentPointValue,
    AssignmentRevisionDefinitionField, AssignmentRevisionDefinitionLocalError, AssignmentTitle,
    BaseAssignmentPolicy, BlueprintCourseValidationError, CourseInstanceReference,
    CourseLocalDateAndTime, CourseTerm, CourseTimeZone, LocalTimeOfDay,
    MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES, QuestionVersionReference,
    RelativeAssignmentSchedule, RelativeAssignmentScheduleMoment, ReusableAssignmentDefaults,
    validate_blueprint_course_title,
};

mod contracts;

pub use contracts::*;

const DOMAIN: &[u8] = b"ple:blueprint-revision-content\0";
/// Current normalized Blueprint Revision Content encoding version.
pub const BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq)]
/// Validated Blueprint Revision Content stored independently from operation provenance.
pub enum BlueprintRevisionContent {
    /// One Blueprint Assignment definition.
    Assignment(BlueprintAssignmentContent),
    /// One Blueprint Course definition.
    Course(BlueprintCourseContent),
}

impl BlueprintRevisionContent {
    /// Wraps one validated Blueprint Assignment definition.
    pub fn assignment(value: BlueprintAssignmentContent) -> Self {
        Self::Assignment(value)
    }
    /// Wraps one validated Blueprint Course definition.
    pub fn course(value: BlueprintCourseContent) -> Self {
        Self::Course(value)
    }
    /// Returns the encoding version persisted beside this Blueprint Revision Content.
    pub const fn canonical_version(&self) -> u8 {
        BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION
    }
    /// Produces the one validated persistence record for this Blueprint Revision Content.
    pub fn canonical_envelope(&self) -> BlueprintRevisionContentRecord {
        let canonical_bytes = canonical_bytes(self);
        let digest = BlueprintContentDigest(Sha256::digest(&canonical_bytes).into());
        BlueprintRevisionContentRecord {
            version: BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION,
            canonical_bytes,
            digest,
        }
    }
    /// Computes the full versioned Blueprint Revision Content digest.
    pub fn digest(&self) -> BlueprintContentDigest {
        self.canonical_envelope().digest()
    }
    /// Checks Blueprint Revision Content and reports both digests when it changed.
    pub fn compare(&self, other: &Self) -> BlueprintContentCheck {
        if self == other {
            BlueprintContentCheck::Equivalent {
                digest: self.digest(),
            }
        } else {
            BlueprintContentCheck::Changed {
                expected: self.digest(),
                actual: other.digest(),
            }
        }
    }
}

/// One validated reusable assignment with trusted immutable question pins.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintAssignmentContent {
    title: AssignmentTitle,
    instructions: AssignmentInstructions,
    entries: Vec<BlueprintAssignmentEntryContent>,
    defaults: ReusableAssignmentDefaults,
    schedule: RelativeAssignmentSchedule,
}
impl BlueprintAssignmentContent {
    /// Validates all reusable assignment meaning before constructing a baseline.
    pub fn new(
        title: AssignmentTitle,
        instructions: AssignmentInstructions,
        entries: Vec<BlueprintAssignmentEntryContent>,
        defaults: ReusableAssignmentDefaults,
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
                BlueprintAssignmentEntryContent::Pool(pool) => Some(pool.candidates.len()),
                _ => None,
            })
            .try_fold(0_usize, |total, value| {
                total
                    .checked_add(value)
                    .ok_or(BlueprintCourseValidationError::TooManyPoolCandidates)
            })?;
        if total > MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES {
            return Err(BlueprintCourseValidationError::TooManyPoolCandidates);
        }
        Ok(Self {
            title,
            instructions,
            entries,
            defaults,
            schedule,
        })
    }
    /// Returns the reusable assignment title.
    pub fn title(&self) -> &str {
        self.title.as_str()
    }
    /// Returns the student-facing reusable instructions.
    pub fn instructions(&self) -> &AssignmentInstructions {
        &self.instructions
    }
    /// Returns the validated reusable policy defaults.
    pub fn defaults(&self) -> &ReusableAssignmentDefaults {
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
            return Err(BlueprintCourseValidationError::InvalidModuleDefinitionCount);
        }
        Ok(Self { label, assignments })
    }
    /// Returns the reusable module label.
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Returns reusable assignments in meaningful authored order.
    pub fn assignments(&self) -> &[BlueprintAssignmentContent] {
        &self.assignments
    }
}

/// One validated Blueprint Course definition.
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
    /// One fixed immutable Question Version and its scoring rule.
    Fixed {
        /// Exact immutable publication pin authorized for the destination.
        reference: QuestionVersionReference,
        /// Exact points copied into the destination assignment.
        points_possible: AssignmentPointValue,
        /// Scoring treatment copied into the destination assignment.
        scoring_rule: AssignmentEntryScoringRule,
    },
    /// One validated deterministic item pool.
    Pool(BlueprintQuestionPoolContent),
}
/// One validated ordered pool of exact immutable publication pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintQuestionPoolContent {
    candidates: Vec<QuestionVersionReference>,
    draw_count: u32,
    points_per_item: AssignmentPointValue,
    scoring_rule: AssignmentEntryScoringRule,
    selection_rule: crate::QuestionPoolSelectionRule,
}
impl BlueprintQuestionPoolContent {
    /// Validates pool cardinality, uniqueness, and draw bounds.
    pub fn new(
        candidates: Vec<QuestionVersionReference>,
        draw_count: u32,
        points_per_item: AssignmentPointValue,
        scoring_rule: AssignmentEntryScoringRule,
        selection_rule: crate::QuestionPoolSelectionRule,
    ) -> Result<Self, BlueprintCourseValidationError> {
        if candidates.is_empty() || candidates.len() > MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL {
            return Err(BlueprintCourseValidationError::InvalidPoolCandidates);
        }
        if draw_count == 0 || usize::try_from(draw_count).ok() > Some(candidates.len()) {
            return Err(BlueprintCourseValidationError::InvalidPoolDrawCount);
        }
        if candidates.iter().collect::<BTreeSet<_>>().len() != candidates.len() {
            return Err(BlueprintCourseValidationError::DuplicatePoolCandidate);
        }
        Ok(Self {
            candidates,
            draw_count,
            points_per_item,
            scoring_rule,
            selection_rule,
        })
    }
    /// Returns candidate pins in meaningful authored order.
    pub fn candidates(&self) -> &[QuestionVersionReference] {
        &self.candidates
    }
    /// Returns the number of candidates selected for one run.
    pub fn draw_count(&self) -> u32 {
        self.draw_count
    }
    /// Returns the point value assigned to every drawn item.
    pub fn points_per_item(&self) -> AssignmentPointValue {
        self.points_per_item
    }
    /// Returns the complete reviewed selection behavior.
    pub fn selection_rule(&self) -> crate::QuestionPoolSelectionRule {
        self.selection_rule
    }
}
/// Full server-side SHA-256 binding of canonical Blueprint Revision Content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlueprintContentDigest([u8; 32]);
impl BlueprintContentDigest {
    /// Returns all persistence bytes without truncation.
    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

/// Validated server-side canonical Blueprint Revision Content bytes and digest.
///
/// Construction remains payload-owned so arbitrary persisted bytes cannot be
/// mistaken for normalized qmodel meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintRevisionContentRecord {
    version: u8,
    canonical_bytes: Vec<u8>,
    digest: BlueprintContentDigest,
}

impl BlueprintRevisionContentRecord {
    /// Returns the canonical encoding version included in the hashed bytes.
    pub const fn version(&self) -> u8 {
        self.version
    }

    /// Borrows the complete domain-separated Blueprint Revision Content encoding.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the complete SHA-256 digest of `canonical_bytes`.
    pub const fn digest(&self) -> BlueprintContentDigest {
        self.digest
    }
}
/// Blueprint Revision Content check used by fast-forward and divergence decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintContentCheck {
    /// Both Blueprint Revision Content values contain exactly the same content.
    Equivalent {
        /// Shared digest recorded with the equivalent baseline.
        digest: BlueprintContentDigest,
    },
    /// At least one Blueprint Revision Content field differs.
    Changed {
        /// Digest of the observed Blueprint Revision Content.
        expected: BlueprintContentDigest,
        /// Digest of the proposed or current Blueprint Revision Content.
        actual: BlueprintContentDigest,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CanonicalPayload<'a> {
    version: u8,
    meaning: CanonicalMeaning<'a>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalMeaning<'a> {
    Assignment {
        definition: CanonicalAssignment<'a>,
    },
    Course {
        title: &'a str,
        modules: Vec<CanonicalModule<'a>>,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CanonicalModule<'a> {
    label: &'a str,
    assignments: Vec<CanonicalAssignment<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CanonicalAssignment<'a> {
    title: &'a str,
    instructions: &'a AssignmentInstructions,
    entries: Vec<CanonicalEntry<'a>>,
    defaults: &'a ReusableAssignmentDefaults,
    schedule: &'a RelativeAssignmentSchedule,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CanonicalEntry<'a> {
    Fixed {
        reference: &'a QuestionVersionReference,
        points_possible: AssignmentPointValue,
        scoring_rule: AssignmentEntryScoringRule,
    },
    Pool {
        candidates: &'a [QuestionVersionReference],
        draw_count: u32,
        points_per_item: AssignmentPointValue,
        scoring_rule: AssignmentEntryScoringRule,
        selection_rule: crate::QuestionPoolSelectionRule,
    },
}

fn canonical_bytes(payload: &BlueprintRevisionContent) -> Vec<u8> {
    let meaning = match payload {
        BlueprintRevisionContent::Assignment(assignment) => CanonicalMeaning::Assignment {
            definition: canonical_assignment(assignment),
        },
        BlueprintRevisionContent::Course(course) => CanonicalMeaning::Course {
            title: course.title(),
            modules: course
                .modules()
                .iter()
                .map(|module| CanonicalModule {
                    label: module.label(),
                    assignments: module
                        .assignments()
                        .iter()
                        .map(canonical_assignment)
                        .collect(),
                })
                .collect(),
        },
    };
    let json = serde_json::to_vec(&CanonicalPayload {
        version: BLUEPRINT_REVISION_CONTENT_ENCODING_VERSION,
        meaning,
    })
    .expect("validated private Blueprint Revision Content serializes");
    let mut bytes = Vec::with_capacity(DOMAIN.len() + json.len());
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(&json);
    bytes
}

fn canonical_assignment(assignment: &BlueprintAssignmentContent) -> CanonicalAssignment<'_> {
    CanonicalAssignment {
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
                } => CanonicalEntry::Fixed {
                    reference,
                    points_possible: *points_possible,
                    scoring_rule: *scoring_rule,
                },
                BlueprintAssignmentEntryContent::Pool(pool) => CanonicalEntry::Pool {
                    candidates: &pool.candidates,
                    draw_count: pool.draw_count,
                    points_per_item: pool.points_per_item,
                    scoring_rule: pool.scoring_rule,
                    selection_rule: pool.selection_rule,
                },
            })
            .collect(),
        defaults: assignment.defaults(),
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

/// Exact immutable Course Schedule Revision locator.
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

/// One relative moment resolved into target-course local and absolute time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ResolvedAssignmentScheduleMoment {
    /// Exact wall-clock value in the target course's authoritative zone.
    pub local: CourseLocalDateAndTime,
    /// Server-resolved absolute timestamp persisted by teaching state.
    pub timestamp: ActivityTimestamp,
}
/// Complete answer-free target-term schedule preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ResolvedAssignmentSchedule {
    /// Authoritative target-course IANA zone for every local value.
    pub time_zone: CourseTimeZone,
    /// Resolved first student-availability moment when configured.
    pub available_at: Option<ResolvedAssignmentScheduleMoment>,
    /// Resolved ordinary due moment when configured.
    pub due_at: Option<ResolvedAssignmentScheduleMoment>,
    /// Resolved hard close moment when configured.
    pub closes_at: Option<ResolvedAssignmentScheduleMoment>,
}
impl RelativeAssignmentSchedule {
    /// Projects a stored course-owned policy into reusable calendar-relative meaning.
    ///
    /// The source term's IANA zone is the sole authority for this conversion.
    /// Each absolute timestamp must project to an inclusive source-term local date,
    /// and the resulting related moments must remain chronological. This gives
    /// adoption callers a typed correction rather than discarding a stored date
    /// when a course is shifted. (ASVS 2.2.1, 2.2.2, 2.2.3)
    pub fn from_base_policy(
        policy: &BaseAssignmentPolicy,
        source_term: &CourseTerm,
    ) -> Result<Self, AssignmentRevisionDefinitionLocalError> {
        let schedule = Self {
            available_at: project_relative_moment(
                policy.available_at,
                source_term,
                AssignmentRevisionDefinitionField::AvailableAt,
            )?,
            due_at: project_relative_moment(
                policy.due_at,
                source_term,
                AssignmentRevisionDefinitionField::DueAt,
            )?,
            closes_at: project_relative_moment(
                policy.closes_at,
                source_term,
                AssignmentRevisionDefinitionField::ClosesAt,
            )?,
        };
        schedule
            .validate()
            .map_err(|_| AssignmentRevisionDefinitionLocalError::ScheduleOutOfOrder)?;
        Ok(schedule)
    }

    /// Resolves calendar offsets in the target term's IANA zone.
    ///
    /// Calendar-day arithmetic occurs before the existing course-local resolver
    /// supplies inclusive-bound and field-specific DST corrections.
    pub fn resolve_for_target_term(
        &self,
        term: &CourseTerm,
    ) -> Result<ResolvedAssignmentSchedule, AssignmentRevisionDefinitionLocalError> {
        self.validate()
            .map_err(|_| AssignmentRevisionDefinitionLocalError::ScheduleOutOfOrder)?;
        Ok(ResolvedAssignmentSchedule {
            time_zone: term.time_zone().clone(),
            available_at: resolve(
                self.available_at.as_ref(),
                term,
                AssignmentRevisionDefinitionField::AvailableAt,
            )?,
            due_at: resolve(
                self.due_at.as_ref(),
                term,
                AssignmentRevisionDefinitionField::DueAt,
            )?,
            closes_at: resolve(
                self.closes_at.as_ref(),
                term,
                AssignmentRevisionDefinitionField::ClosesAt,
            )?,
        })
    }
}

fn project_relative_moment(
    value: Option<ActivityTimestamp>,
    source_term: &CourseTerm,
    field: AssignmentRevisionDefinitionField,
) -> Result<Option<RelativeAssignmentScheduleMoment>, AssignmentRevisionDefinitionLocalError> {
    value
        .map(|value| {
            let local = CourseLocalDateAndTime::from_activity_timestamp(value, source_term, field)?;
            let date = NaiveDate::parse_from_str(&local.as_str()[..10], "%Y-%m-%d")
                .expect("validated course-local date");
            let start = NaiveDate::parse_from_str(source_term.start_date().as_str(), "%Y-%m-%d")
                .expect("validated course term");
            let day_offset = i32::try_from(date.signed_duration_since(start).num_days())
                .map_err(|_| AssignmentRevisionDefinitionLocalError::TimestampOutOfRange(field))?;
            let local_time =
                LocalTimeOfDay::parse(&local.as_str()[11..]).expect("validated course-local time");
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
    field: AssignmentRevisionDefinitionField,
) -> Result<Option<ResolvedAssignmentScheduleMoment>, AssignmentRevisionDefinitionLocalError> {
    value
        .map(|value| {
            let start = NaiveDate::parse_from_str(term.start_date().as_str(), "%Y-%m-%d")
                .expect("valid term");
            let date = start
                .checked_add_signed(Duration::days(i64::from(value.day_offset)))
                .ok_or(AssignmentRevisionDefinitionLocalError::TimestampOutOfRange(
                    field,
                ))?;
            let local = CourseLocalDateAndTime::parse(&format!(
                "{}T{}",
                date.format("%Y-%m-%d"),
                value.local_time.as_str()
            ))
            .map_err(|_| AssignmentRevisionDefinitionLocalError::TimestampOutOfRange(field))?;
            Ok(ResolvedAssignmentScheduleMoment {
                timestamp: local.resolve_for_course(term, field)?,
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
        let term =
            CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("term");
        let resolved = schedule
            .resolve_for_target_term(&term)
            .expect("resolved schedule");
        let wire = serde_json::to_value(&resolved).expect("schedule serializes");
        assert!(wire.get("time_zone").is_some());
        assert!(wire.get("available_at").is_some());
        assert!(wire.get("timeZone").is_none());
        let mut forged = wire;
        forged["authority"] = serde_json::json!("instructor");
        assert!(serde_json::from_value::<ResolvedAssignmentSchedule>(forged).is_err());
    }
}
