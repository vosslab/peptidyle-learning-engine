//! Validated B2 semantic meaning and target-term schedule projection.

use std::collections::BTreeSet;
use std::num::NonZeroU64;
use std::str::FromStr;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ActivityTimestamp, AssignmentInstructions, AssignmentScoringMode,
    AssignmentTeachingSettingsField, AssignmentTeachingSettingsLocalError, BaseAssignmentPolicy,
    CourseLocalDateTime, CourseTerm, IanaTimeZone, LocalTimeOfDay,
    MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP, MAX_ASSIGNMENT_ORDERED_ENTRIES,
    MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES, PointValue, PoolDrawAlgorithm, ProblemVersionRef,
    RelativeAssignmentSchedule, RelativeScheduleMoment, ReusableAssignmentDefaults,
    ReusableCurriculumValidationError, SelectionOrdering, validate_reusable_curriculum_title,
};

mod contracts;

pub use contracts::*;

const DOMAIN: &[u8] = b"ple:curriculum-semantic\0";
/// Current normalized semantic-payload encoding version stored with B2 baselines.
pub const CURRICULUM_SEMANTIC_CANONICAL_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq)]
/// Validated reusable meaning stored independently from adoption provenance.
pub enum CurriculumSemanticPayload {
    /// One Blueprint-sized reusable assignment definition.
    Assignment(CurriculumSemanticAssignment),
    /// One BlueprintCourse-sized reusable course tree.
    Course(CurriculumSemanticCourse),
}

impl CurriculumSemanticPayload {
    /// Wraps one validated assignment-sized semantic payload.
    pub fn assignment(value: CurriculumSemanticAssignment) -> Self {
        Self::Assignment(value)
    }
    /// Wraps one validated course-sized semantic payload.
    pub fn course(value: CurriculumSemanticCourse) -> Self {
        Self::Course(value)
    }
    /// Returns the encoding version persisted beside this semantic baseline.
    pub const fn canonical_version(&self) -> u8 {
        CURRICULUM_SEMANTIC_CANONICAL_VERSION
    }
    /// Produces the one validated persistence envelope for this reusable meaning.
    pub fn canonical_envelope(&self) -> CurriculumSemanticEnvelope {
        let canonical_bytes = canonical_bytes(self);
        let digest = CurriculumSemanticDigest(Sha256::digest(&canonical_bytes).into());
        CurriculumSemanticEnvelope {
            version: CURRICULUM_SEMANTIC_CANONICAL_VERSION,
            canonical_bytes,
            digest,
        }
    }
    /// Computes the full versioned digest of reusable meaning only.
    pub fn digest(&self) -> CurriculumSemanticDigest {
        self.canonical_envelope().digest()
    }
    /// Compares validated meaning and reports both digests when it changed.
    pub fn compare(&self, other: &Self) -> CurriculumSemanticComparison {
        if self == other {
            CurriculumSemanticComparison::Equivalent {
                digest: self.digest(),
            }
        } else {
            CurriculumSemanticComparison::Changed {
                expected: self.digest(),
                actual: other.digest(),
            }
        }
    }
}

/// One validated reusable assignment with trusted immutable question pins.
#[derive(Debug, Clone, PartialEq)]
pub struct CurriculumSemanticAssignment {
    title: String,
    instructions: AssignmentInstructions,
    entries: Vec<CurriculumSemanticAssignmentEntry>,
    defaults: ReusableAssignmentDefaults,
    schedule: RelativeAssignmentSchedule,
}
impl CurriculumSemanticAssignment {
    /// Validates all reusable assignment meaning before constructing a baseline.
    pub fn new(
        title: String,
        instructions: AssignmentInstructions,
        entries: Vec<CurriculumSemanticAssignmentEntry>,
        defaults: ReusableAssignmentDefaults,
        schedule: RelativeAssignmentSchedule,
    ) -> Result<Self, ReusableCurriculumValidationError> {
        validate_reusable_curriculum_title(&title)
            .map_err(|_| ReusableCurriculumValidationError::InvalidDefinitionTitle)?;
        if entries.is_empty() || entries.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(ReusableCurriculumValidationError::InvalidEntryCount);
        }
        defaults.validate()?;
        schedule.validate()?;
        let total = entries
            .iter()
            .filter_map(|entry| match entry {
                CurriculumSemanticAssignmentEntry::Pool(pool) => Some(pool.candidates.len()),
                _ => None,
            })
            .try_fold(0_usize, |total, value| {
                total
                    .checked_add(value)
                    .ok_or(ReusableCurriculumValidationError::TooManyPoolCandidates)
            })?;
        if total > MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES {
            return Err(ReusableCurriculumValidationError::TooManyPoolCandidates);
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
        &self.title
    }
    /// Returns the learner-facing reusable instructions.
    pub fn instructions(&self) -> &AssignmentInstructions {
        &self.instructions
    }
    /// Returns the validated reusable policy defaults.
    pub fn defaults(&self) -> &ReusableAssignmentDefaults {
        &self.defaults
    }
    /// Returns fixed questions and pools in meaningful authored order.
    pub fn entries(&self) -> &[CurriculumSemanticAssignmentEntry] {
        &self.entries
    }
    /// Returns the target-term-relative schedule defaults.
    pub fn schedule(&self) -> &RelativeAssignmentSchedule {
        &self.schedule
    }
}

/// One validated labelled module in a course-sized semantic payload.
#[derive(Debug, Clone, PartialEq)]
pub struct CurriculumSemanticModule {
    label: String,
    assignments: Vec<CurriculumSemanticAssignment>,
}
impl CurriculumSemanticModule {
    /// Validates a module label and its nonempty ordered assignments.
    pub fn new(
        label: String,
        assignments: Vec<CurriculumSemanticAssignment>,
    ) -> Result<Self, ReusableCurriculumValidationError> {
        validate_reusable_curriculum_title(&label)
            .map_err(|_| ReusableCurriculumValidationError::InvalidModuleLabel)?;
        if assignments.is_empty() || assignments.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(ReusableCurriculumValidationError::InvalidModuleDefinitionCount);
        }
        Ok(Self { label, assignments })
    }
    /// Returns the reusable module label.
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Returns reusable assignments in meaningful authored order.
    pub fn assignments(&self) -> &[CurriculumSemanticAssignment] {
        &self.assignments
    }
}

/// One validated course-sized reusable semantic tree.
#[derive(Debug, Clone, PartialEq)]
pub struct CurriculumSemanticCourse {
    title: String,
    modules: Vec<CurriculumSemanticModule>,
}
impl CurriculumSemanticCourse {
    /// Validates a reusable course title and its nonempty ordered modules.
    pub fn new(
        title: String,
        modules: Vec<CurriculumSemanticModule>,
    ) -> Result<Self, ReusableCurriculumValidationError> {
        validate_reusable_curriculum_title(&title)
            .map_err(|_| ReusableCurriculumValidationError::InvalidBlueprintTitle)?;
        if modules.is_empty() || modules.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(ReusableCurriculumValidationError::InvalidModuleCount);
        }
        Ok(Self { title, modules })
    }
    /// Returns the reusable course title.
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Returns reusable modules in meaningful authored order.
    pub fn modules(&self) -> &[CurriculumSemanticModule] {
        &self.modules
    }
}

/// One ordered semantic assignment entry containing only trusted exact pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurriculumSemanticAssignmentEntry {
    /// One fixed immutable publication and its reusable scoring meaning.
    Fixed {
        /// Exact immutable publication pin authorized for the destination.
        reference: ProblemVersionRef,
        /// Exact points copied into the destination assignment.
        points_possible: PointValue,
        /// Scoring treatment copied into the destination assignment.
        scoring_mode: AssignmentScoringMode,
    },
    /// One validated deterministic item pool.
    Pool(CurriculumSemanticPool),
}
/// One validated ordered pool of exact immutable publication pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurriculumSemanticPool {
    candidates: Vec<ProblemVersionRef>,
    draw_count: u32,
    points_per_item: PointValue,
    ordering: SelectionOrdering,
    algorithm: PoolDrawAlgorithm,
}
impl CurriculumSemanticPool {
    /// Validates pool cardinality, uniqueness, and draw bounds.
    pub fn new(
        candidates: Vec<ProblemVersionRef>,
        draw_count: u32,
        points_per_item: PointValue,
        ordering: SelectionOrdering,
        algorithm: PoolDrawAlgorithm,
    ) -> Result<Self, ReusableCurriculumValidationError> {
        if candidates.is_empty() || candidates.len() > MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP
        {
            return Err(ReusableCurriculumValidationError::InvalidPoolCandidates);
        }
        if draw_count == 0 || usize::try_from(draw_count).ok() > Some(candidates.len()) {
            return Err(ReusableCurriculumValidationError::InvalidPoolDrawCount);
        }
        if candidates.iter().collect::<BTreeSet<_>>().len() != candidates.len() {
            return Err(ReusableCurriculumValidationError::DuplicatePoolCandidate);
        }
        Ok(Self {
            candidates,
            draw_count,
            points_per_item,
            ordering,
            algorithm,
        })
    }
    /// Returns candidate pins in meaningful authored order.
    pub fn candidates(&self) -> &[ProblemVersionRef] {
        &self.candidates
    }
    /// Returns the number of candidates selected for one run.
    pub fn draw_count(&self) -> u32 {
        self.draw_count
    }
    /// Returns the point value assigned to every drawn item.
    pub fn points_per_item(&self) -> PointValue {
        self.points_per_item
    }
    /// Returns the selected-item presentation ordering.
    pub fn ordering(&self) -> SelectionOrdering {
        self.ordering
    }
    /// Returns the immutable deterministic pool algorithm version.
    pub fn algorithm(&self) -> PoolDrawAlgorithm {
        self.algorithm
    }
}
/// Full server-side SHA-256 binding of one canonical semantic payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CurriculumSemanticDigest([u8; 32]);
impl CurriculumSemanticDigest {
    /// Returns all persistence bytes without truncation.
    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }

    #[cfg(test)]
    pub(crate) const fn test_value(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// Validated server-side canonical semantic bytes and their complete digest.
///
/// Construction remains payload-owned so arbitrary persisted bytes cannot be
/// mistaken for normalized qmodel meaning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurriculumSemanticEnvelope {
    version: u8,
    canonical_bytes: Vec<u8>,
    digest: CurriculumSemanticDigest,
}

impl CurriculumSemanticEnvelope {
    /// Returns the canonical encoding version included in the hashed bytes.
    pub const fn version(&self) -> u8 {
        self.version
    }

    /// Borrows the complete domain-separated canonical semantic encoding.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the complete SHA-256 digest of `canonical_bytes`.
    pub const fn digest(&self) -> CurriculumSemanticDigest {
        self.digest
    }
}
/// Meaning-level comparison used by fast-forward and divergence decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurriculumSemanticComparison {
    /// Both validated payloads contain exactly the same reusable meaning.
    Equivalent {
        /// Shared digest recorded with the equivalent baseline.
        digest: CurriculumSemanticDigest,
    },
    /// At least one reusable semantic field differs.
    Changed {
        /// Digest of the observed baseline meaning.
        expected: CurriculumSemanticDigest,
        /// Digest of the proposed or current meaning.
        actual: CurriculumSemanticDigest,
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
        reference: &'a ProblemVersionRef,
        points_possible: PointValue,
        scoring_mode: AssignmentScoringMode,
    },
    Pool {
        candidates: &'a [ProblemVersionRef],
        draw_count: u32,
        points_per_item: PointValue,
        ordering: SelectionOrdering,
        algorithm: PoolDrawAlgorithm,
    },
}

fn canonical_bytes(payload: &CurriculumSemanticPayload) -> Vec<u8> {
    let meaning = match payload {
        CurriculumSemanticPayload::Assignment(assignment) => CanonicalMeaning::Assignment {
            definition: canonical_assignment(assignment),
        },
        CurriculumSemanticPayload::Course(course) => CanonicalMeaning::Course {
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
        version: CURRICULUM_SEMANTIC_CANONICAL_VERSION,
        meaning,
    })
    .expect("validated private semantic wire serializes");
    let mut bytes = Vec::with_capacity(DOMAIN.len() + json.len());
    bytes.extend_from_slice(DOMAIN);
    bytes.extend_from_slice(&json);
    bytes
}

fn canonical_assignment(assignment: &CurriculumSemanticAssignment) -> CanonicalAssignment<'_> {
    CanonicalAssignment {
        title: assignment.title(),
        instructions: assignment.instructions(),
        entries: assignment
            .entries()
            .iter()
            .map(|entry| match entry {
                CurriculumSemanticAssignmentEntry::Fixed {
                    reference,
                    points_possible,
                    scoring_mode,
                } => CanonicalEntry::Fixed {
                    reference,
                    points_possible: *points_possible,
                    scoring_mode: *scoring_mode,
                },
                CurriculumSemanticAssignmentEntry::Pool(pool) => CanonicalEntry::Pool {
                    candidates: &pool.candidates,
                    draw_count: pool.draw_count,
                    points_per_item: pool.points_per_item,
                    ordering: pool.ordering,
                    algorithm: pool.algorithm,
                },
            })
            .collect(),
        defaults: assignment.defaults(),
        schedule: assignment.schedule(),
    }
}

/// Strong course-wide schedule revision binding preview to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CourseScheduleRevision(NonZeroU64);
impl CourseScheduleRevision {
    /// Initial revision for a newly persisted teaching-course schedule.
    pub const INITIAL: Self = Self(NonZeroU64::MIN);
    /// Rebuilds a positive PostgreSQL-`BIGINT` revision.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }
    /// Returns the exact positive revision scalar.
    pub fn value(self) -> u64 {
        self.0.get()
    }
    /// Advances without exceeding the persistence boundary.
    pub fn checked_next(self) -> Option<Self> {
        Self::new(self.value().checked_add(1)?)
    }
}
impl FromStr for CourseScheduleRevision {
    type Err = CourseScheduleRevisionError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(CourseScheduleRevisionError);
        }
        value
            .parse()
            .ok()
            .and_then(Self::new)
            .ok_or(CourseScheduleRevisionError)
    }
}
impl TryFrom<String> for CourseScheduleRevision {
    type Error = CourseScheduleRevisionError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl From<CourseScheduleRevision> for String {
    fn from(value: CourseScheduleRevision) -> Self {
        value.value().to_string()
    }
}
impl std::fmt::Display for CourseScheduleRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}
/// A schedule revision was not one canonical positive decimal value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseScheduleRevisionError;

impl std::fmt::Display for CourseScheduleRevisionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("course schedule revision must be a canonical positive decimal")
    }
}

impl std::error::Error for CourseScheduleRevisionError {}

/// One relative moment resolved into target-course local and absolute time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ResolvedRelativeScheduleMoment {
    /// Exact wall-clock value in the target course's authoritative zone.
    pub local: CourseLocalDateTime,
    /// Server-resolved absolute timestamp persisted by teaching state.
    pub timestamp: ActivityTimestamp,
}
/// Complete answer-free target-term schedule preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ResolvedRelativeAssignmentSchedule {
    /// Authoritative target-course IANA zone for every local value.
    pub time_zone: IanaTimeZone,
    /// Resolved first learner-availability moment when configured.
    pub available_at: Option<ResolvedRelativeScheduleMoment>,
    /// Resolved ordinary due moment when configured.
    pub due_at: Option<ResolvedRelativeScheduleMoment>,
    /// Resolved hard close moment when configured.
    pub closes_at: Option<ResolvedRelativeScheduleMoment>,
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
    ) -> Result<Self, AssignmentTeachingSettingsLocalError> {
        let schedule = Self {
            available_at: project_relative_moment(
                policy.available_at,
                source_term,
                AssignmentTeachingSettingsField::AvailableAt,
            )?,
            due_at: project_relative_moment(
                policy.due_at,
                source_term,
                AssignmentTeachingSettingsField::DueAt,
            )?,
            closes_at: project_relative_moment(
                policy.closes_at,
                source_term,
                AssignmentTeachingSettingsField::ClosesAt,
            )?,
        };
        schedule
            .validate()
            .map_err(|_| AssignmentTeachingSettingsLocalError::ScheduleOutOfOrder)?;
        Ok(schedule)
    }

    /// Resolves calendar offsets in the target term's IANA zone.
    ///
    /// Calendar-day arithmetic occurs before the existing course-local resolver
    /// supplies inclusive-bound and field-specific DST corrections.
    pub fn resolve_for_target_term(
        &self,
        term: &CourseTerm,
    ) -> Result<ResolvedRelativeAssignmentSchedule, AssignmentTeachingSettingsLocalError> {
        self.validate()
            .map_err(|_| AssignmentTeachingSettingsLocalError::ScheduleOutOfOrder)?;
        Ok(ResolvedRelativeAssignmentSchedule {
            time_zone: term.time_zone().clone(),
            available_at: resolve(
                self.available_at.as_ref(),
                term,
                AssignmentTeachingSettingsField::AvailableAt,
            )?,
            due_at: resolve(
                self.due_at.as_ref(),
                term,
                AssignmentTeachingSettingsField::DueAt,
            )?,
            closes_at: resolve(
                self.closes_at.as_ref(),
                term,
                AssignmentTeachingSettingsField::ClosesAt,
            )?,
        })
    }
}

fn project_relative_moment(
    value: Option<ActivityTimestamp>,
    source_term: &CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<Option<RelativeScheduleMoment>, AssignmentTeachingSettingsLocalError> {
    value
        .map(|value| {
            let local = CourseLocalDateTime::from_activity_timestamp(value, source_term, field)?;
            let date = NaiveDate::parse_from_str(&local.as_str()[..10], "%Y-%m-%d")
                .expect("validated course-local date");
            let start = NaiveDate::parse_from_str(source_term.start_date().as_str(), "%Y-%m-%d")
                .expect("validated course term");
            let day_offset = i32::try_from(date.signed_duration_since(start).num_days())
                .map_err(|_| AssignmentTeachingSettingsLocalError::TimestampOutOfRange(field))?;
            let local_time =
                LocalTimeOfDay::parse(&local.as_str()[11..]).expect("validated course-local time");
            Ok(RelativeScheduleMoment {
                day_offset,
                local_time,
            })
        })
        .transpose()
}
fn resolve(
    value: Option<&RelativeScheduleMoment>,
    term: &CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<Option<ResolvedRelativeScheduleMoment>, AssignmentTeachingSettingsLocalError> {
    value
        .map(|value| {
            let start = NaiveDate::parse_from_str(term.start_date().as_str(), "%Y-%m-%d")
                .expect("valid term");
            let date = start
                .checked_add_signed(Duration::days(i64::from(value.day_offset)))
                .ok_or(AssignmentTeachingSettingsLocalError::TimestampOutOfRange(
                    field,
                ))?;
            let local = CourseLocalDateTime::parse(&format!(
                "{}T{}",
                date.format("%Y-%m-%d"),
                value.local_time.as_str()
            ))
            .map_err(|_| AssignmentTeachingSettingsLocalError::TimestampOutOfRange(field))?;
            Ok(ResolvedRelativeScheduleMoment {
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
            available_at: Some(RelativeScheduleMoment {
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
        assert!(serde_json::from_value::<ResolvedRelativeAssignmentSchedule>(forged).is_err());
    }
}
