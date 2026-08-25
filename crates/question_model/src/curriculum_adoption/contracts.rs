//! Typed browser-safe previews and server-owned commands for B2 adoption.

use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::{CourseScheduleRevision, ResolvedRelativeAssignmentSchedule};
use crate::{
    AlphaCourseReference, AlphaCourseRevision, AssignmentReference,
    AssignmentTeachingSettingsFailureCode, AssignmentTeachingSettingsLocalError,
    AssignmentTeachingSettingsValidationFailure, BlueprintReference, BlueprintRevision,
    CourseReference, CourseTerm, ReusableCurriculumTitleError, validate_reusable_curriculum_title,
};

/// Largest browser-supplied idempotency key accepted by one B2 write.
pub const MAX_CURRICULUM_ADOPTION_IDEMPOTENCY_KEY_BYTES: usize = 128;

/// A validated opaque browser key that binds a completed adoption retry.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurriculumAdoptionIdempotencyKey(String);

impl CurriculumAdoptionIdempotencyKey {
    /// Parses the bounded opaque key used for one completed retry.
    pub fn parse(value: &str) -> Result<Self, CurriculumAdoptionIdempotencyKeyError> {
        let valid = !value.is_empty()
            && value.len() <= MAX_CURRICULUM_ADOPTION_IDEMPOTENCY_KEY_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
        valid
            .then(|| Self(value.to_owned()))
            .ok_or(CurriculumAdoptionIdempotencyKeyError)
    }

    /// Returns the opaque browser value without assigning any authority to it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CurriculumAdoptionIdempotencyKey {
    type Error = CurriculumAdoptionIdempotencyKeyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<CurriculumAdoptionIdempotencyKey> for String {
    fn from(value: CurriculumAdoptionIdempotencyKey) -> Self {
        value.0
    }
}

impl std::fmt::Debug for CurriculumAdoptionIdempotencyKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CurriculumAdoptionIdempotencyKey([opaque])")
    }
}

/// An idempotency key was blank, oversized, or outside the opaque-key alphabet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumAdoptionIdempotencyKeyError;

impl std::fmt::Display for CurriculumAdoptionIdempotencyKeyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum adoption idempotency key is invalid")
    }
}

impl std::error::Error for CurriculumAdoptionIdempotencyKeyError {}

macro_rules! positive_revision {
    ($name:ident, $error:ident, $description:literal) => {
        #[doc = $description]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(NonZeroU64);

        impl $name {
            /// Builds a positive revision that fits PostgreSQL `BIGINT`.
            pub fn new(value: u64) -> Option<Self> {
                (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
            }

            /// Returns the exact persistence revision scalar.
            pub fn value(self) -> u64 {
                self.0.get()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "{}", self.value())
            }
        }

        impl FromStr for $name {
            type Err = $error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                if value.is_empty()
                    || value.starts_with('0')
                    || !value.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err($error);
                }
                value.parse().ok().and_then(Self::new).ok_or($error)
            }
        }

        impl TryFrom<String> for $name {
            type Error = $error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }
    };
}

positive_revision!(
    CurriculumImportRevision,
    CurriculumImportRevisionError,
    "Strong revision evidence for one durable curriculum import."
);

/// An import revision was not one canonical positive PostgreSQL-bigint decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumImportRevisionError;

impl std::fmt::Display for CurriculumImportRevisionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum import revision must be a canonical positive decimal")
    }
}

impl std::error::Error for CurriculumImportRevisionError {}

positive_revision!(
    AssignmentRevision,
    AssignmentRevisionError,
    "Strong revision evidence for one ordinary teaching assignment definition."
);

/// An assignment revision was not one canonical positive PostgreSQL-bigint decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentRevisionError;

impl std::fmt::Display for AssignmentRevisionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("assignment revision must be a canonical positive decimal")
    }
}

impl std::error::Error for AssignmentRevisionError {}

/// A validated display title for a new ordinary course or independent Alpha fork.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurriculumAdoptionTitle(String);

impl CurriculumAdoptionTitle {
    /// Builds one bounded, trimmed nonblank title using the shared curriculum rule.
    pub fn parse(value: &str) -> Result<Self, CurriculumAdoptionTitleError> {
        validate_reusable_curriculum_title(value).map_err(CurriculumAdoptionTitleError::from)?;
        Ok(Self(value.to_owned()))
    }

    /// Returns the validated display title.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CurriculumAdoptionTitle {
    type Error = CurriculumAdoptionTitleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<CurriculumAdoptionTitle> for String {
    fn from(value: CurriculumAdoptionTitle) -> Self {
        value.0
    }
}

/// A destination title did not satisfy the shared curriculum text rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumAdoptionTitleError;

impl From<ReusableCurriculumTitleError> for CurriculumAdoptionTitleError {
    fn from(_: ReusableCurriculumTitleError) -> Self {
        Self
    }
}

impl std::fmt::Display for CurriculumAdoptionTitleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum adoption title is invalid")
    }
}

impl std::error::Error for CurriculumAdoptionTitleError {}

/// A revision-bound Blueprint source observed through the authorized read plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedBlueprintSource {
    /// Route locator resolved under the current authorized owner boundary.
    pub reference: BlueprintReference,
    /// Complete source revision selected for preview or write.
    pub revision: BlueprintRevision,
}

/// A revision-bound public Alpha source observed through the authorized read plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedAlphaSource {
    /// Public route locator resolved under approved-Instructor authority.
    pub reference: AlphaCourseReference,
    /// Complete Alpha tree revision selected for preview or write.
    pub revision: AlphaCourseRevision,
}

/// An ordinary assignment revision observed in a course schedule preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservedAssignmentRevision {
    /// Assignment route locator resolved inside the previewed course.
    pub assignment: AssignmentReference,
    /// Exact assignment-definition revision observed by the preview.
    pub revision: AssignmentRevision,
}

/// All revision evidence a whole-course schedule preview binds to apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "CourseScheduleWitnessParts")]
pub struct CourseScheduleWitness {
    /// Course route locator resolved under direct Instructor authority.
    pub course: CourseReference,
    /// Schedule revision advanced by every course-term or base-schedule writer.
    pub schedule_revision: CourseScheduleRevision,
    assignment_revisions: Vec<ObservedAssignmentRevision>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CourseScheduleWitnessParts {
    course: CourseReference,
    schedule_revision: CourseScheduleRevision,
    assignment_revisions: Vec<ObservedAssignmentRevision>,
}

impl TryFrom<CourseScheduleWitnessParts> for CourseScheduleWitness {
    type Error = CourseScheduleWitnessError;

    fn try_from(value: CourseScheduleWitnessParts) -> Result<Self, Self::Error> {
        Self::new(
            value.course,
            value.schedule_revision,
            value.assignment_revisions,
        )
    }
}

impl CourseScheduleWitness {
    /// Builds a deterministic witness, rejecting duplicate assignment bindings.
    pub fn new(
        course: CourseReference,
        schedule_revision: CourseScheduleRevision,
        mut assignment_revisions: Vec<ObservedAssignmentRevision>,
    ) -> Result<Self, CourseScheduleWitnessError> {
        assignment_revisions.sort_unstable();
        if assignment_revisions
            .windows(2)
            .any(|pair| pair[0].assignment == pair[1].assignment)
        {
            return Err(CourseScheduleWitnessError::DuplicateAssignment);
        }
        Ok(Self {
            course,
            schedule_revision,
            assignment_revisions,
        })
    }

    /// Returns assignment revision bindings in deterministic route-reference order.
    pub fn assignment_revisions(&self) -> &[ObservedAssignmentRevision] {
        &self.assignment_revisions
    }
}

/// A course-schedule preview repeated one assignment witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseScheduleWitnessError {
    /// One assignment had more than one revision in the same preview witness.
    DuplicateAssignment,
}

impl std::fmt::Display for CourseScheduleWitnessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("course schedule witness repeats an assignment")
    }
}

impl std::error::Error for CourseScheduleWitnessError {}

/// Browser-safe source descriptor with public locators and observed revisions only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CurriculumSourceView {
    /// A private owner-scoped Blueprint selected under the current session.
    Blueprint(ObservedBlueprintSource),
    /// A public Alpha selected under approved-Instructor authority.
    Alpha(ObservedAlphaSource),
}

/// Field-specific schedule correction derived from existing local-time domain errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumScheduleCorrection {
    /// Existing bounded, answer-free correction details.
    pub correction: AssignmentTeachingSettingsValidationFailure,
}

impl From<AssignmentTeachingSettingsLocalError> for CurriculumScheduleCorrection {
    fn from(value: AssignmentTeachingSettingsLocalError) -> Self {
        Self {
            correction: AssignmentTeachingSettingsValidationFailure {
                error: AssignmentTeachingSettingsFailureCode::AssignmentTeachingSettingsInvalid,
                field: value.field(),
                reason: value.reason(),
                message: value.to_string(),
            },
        }
    }
}

/// Typed route-safe preview requests. The server derives authority and source meaning separately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CurriculumAdoptionPreviewRequest {
    /// Preview a public Alpha fork against the selected observed source revision.
    ForkAlpha { source: ObservedAlphaSource },
    /// Preview one Blueprint definition becoming a normal assignment in a course.
    InstantiateBlueprint {
        source: ObservedBlueprintSource,
        course: CourseReference,
        target_term: CourseTerm,
    },
    /// Preview one Alpha becoming a new ordinary teaching course.
    InstantiateAlpha {
        source: ObservedAlphaSource,
        target_term: CourseTerm,
    },
    /// Preview a new ordinary teaching course from an existing teaching course.
    RolloverCourse {
        witness: CourseScheduleWitness,
        target_term: CourseTerm,
    },
    /// Preview an atomic target-term update for an unissued course.
    ShiftCourseTerm {
        witness: CourseScheduleWitness,
        target_term: CourseTerm,
    },
    /// Preview whether one imported assignment can fast-forward from its source.
    FastForwardAssignment {
        course: CourseReference,
        assignment: ObservedAssignmentRevision,
        import_revision: CurriculumImportRevision,
    },
    /// Preview a separate draft from the selected source after divergence.
    CreateSourceDerivedAssignment {
        course: CourseReference,
        source: CurriculumSourceView,
    },
}

/// One answer-free assignment schedule row in a B2 preview or import view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumAssignmentView {
    /// Assignment route locator scoped by the surrounding course.
    pub reference: AssignmentReference,
    /// Instructor-visible assignment title.
    pub title: CurriculumAdoptionTitle,
    /// Exact assignment-definition revision shown to the Instructor.
    pub revision: AssignmentRevision,
    /// Server-resolved target-term schedule projection.
    pub schedule: ResolvedRelativeAssignmentSchedule,
}

/// One safe preview result. A correction preserves the complete field-specific next action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumAdoptionPreviewView {
    /// Operation represented by the preview.
    pub operation: CurriculumAdoptionOperation,
    /// Course term selected for this preview.
    pub target_term: CourseTerm,
    /// Current course/assignment revisions that apply must repeat.
    pub witness: Option<CourseScheduleWitness>,
    /// Server-resolved schedule results in deterministic assignment order.
    pub assignments: Vec<CurriculumAssignmentView>,
    /// Correction details when a target term contains an invalid local schedule.
    pub corrections: Vec<CurriculumScheduleCorrection>,
}

/// Answer-free durable view of one teaching-course import binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumImportView {
    /// Destination course that owns this import.
    pub course: CourseReference,
    /// Destination assignment that owns this import.
    pub assignment: AssignmentReference,
    /// Observed source binding and source revision.
    pub source: CurriculumImportSourceView,
    /// Revision advanced whenever the import baseline/envelope changes.
    pub revision: CurriculumImportRevision,
    /// Whether current destination reusable meaning still equals its baseline.
    pub reusable_meaning_matches_baseline: bool,
}

/// Answer-free durable view of one course-level Alpha import and its assignment imports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumCourseImportView {
    /// Destination teaching course that owns the imported curriculum.
    pub course: CourseReference,
    /// Public Alpha source and exact revision that established the course import.
    pub source: ObservedAlphaSource,
    /// Current destination term and authoritative IANA zone.
    pub term: CourseTerm,
    /// Current course-wide schedule revision.
    pub schedule_revision: CourseScheduleRevision,
    /// Per-assignment imports in deterministic teaching-assignment order.
    pub assignments: Vec<CurriculumImportView>,
}

/// Safe public source provenance attached to one import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CurriculumImportSourceView {
    /// Imported from a selected Blueprint revision.
    Blueprint(ObservedBlueprintSource),
    /// Imported from a selected Alpha revision.
    Alpha(ObservedAlphaSource),
}

/// One operation at the adoption boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CurriculumAdoptionOperation {
    /// Fork a public Alpha into an independently editable Alpha.
    ForkAlpha,
    /// Create a draft assignment from a Blueprint.
    InstantiateBlueprint,
    /// Create a teaching course from a public Alpha.
    InstantiateAlpha,
    /// Create a new teaching course from copyable source-course definitions.
    RolloverCourse,
    /// Atomically change an unissued course to a new term.
    ShiftCourseTerm,
    /// Fast-forward an untouched imported assignment.
    FastForwardAssignment,
    /// Create a new assignment from a selected source after divergence.
    CreateSourceDerivedAssignment,
}

/// Server-owned write command. It deliberately has no wire serialization.
#[derive(Debug, Clone, PartialEq)]
pub enum CurriculumAdoptionCommand {
    /// Fork command with immutable source binding.
    ForkAlpha(ForkAlphaCommand),
    /// Blueprint-instantiation command with exact source revision.
    InstantiateBlueprint(BlueprintInstantiationCommand),
    /// Alpha-instantiation command with exact source revision.
    InstantiateAlpha(AlphaInstantiationCommand),
    /// Rollover command with source-course schedule witness.
    RolloverCourse(CourseRolloverCommand),
    /// Whole-course target-term shift command.
    ShiftCourseTerm(CourseTermShiftCommand),
    /// Eligible fast-forward command.
    FastForwardAssignment(AssignmentFastForwardCommand),
    /// New independent draft command after a divergent import.
    CreateSourceDerivedAssignment(CreateSourceDerivedAssignmentCommand),
}

/// Server-owned Alpha fork command. Source resolution and authority are Store responsibilities.
#[derive(Debug, Clone, PartialEq)]
pub struct ForkAlphaCommand {
    /// Public source and exact observed revision.
    pub source: ObservedAlphaSource,
    /// Validated independently editable destination title.
    pub title: CurriculumAdoptionTitle,
    /// Client retry binding.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Server-owned Blueprint-to-assignment command.
#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintInstantiationCommand {
    /// Owner-scoped source and observed revision.
    pub source: ObservedBlueprintSource,
    /// Existing ordinary teaching course destination.
    pub course: CourseReference,
    /// Target term resolved by the Store from the destination course.
    pub target_term: CourseTerm,
    /// Preview witness binding the write to its schedule observation.
    pub preview_witness: CourseScheduleWitness,
    /// Client retry binding.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Server-owned Alpha-to-course command.
#[derive(Debug, Clone, PartialEq)]
pub struct AlphaInstantiationCommand {
    /// Public source and exact observed revision.
    pub source: ObservedAlphaSource,
    /// Validated new teaching-course title.
    pub title: CurriculumAdoptionTitle,
    /// Explicit target term with authoritative IANA zone.
    pub target_term: CourseTerm,
    /// Client retry binding.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Server-owned rollover command. The Store keeps learner state out of the destination.
#[derive(Debug, Clone, PartialEq)]
pub struct CourseRolloverCommand {
    /// Source teaching-course schedule witness from the preview.
    pub preview_witness: CourseScheduleWitness,
    /// Validated new teaching-course title.
    pub title: CurriculumAdoptionTitle,
    /// Target term selected by the Instructor.
    pub target_term: CourseTerm,
    /// Client retry binding.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Server-owned atomic term-shift command for one unissued teaching course.
#[derive(Debug, Clone, PartialEq)]
pub struct CourseTermShiftCommand {
    /// Course and all assignment revisions returned by preview.
    pub preview_witness: CourseScheduleWitness,
    /// Target term selected by the Instructor.
    pub target_term: CourseTerm,
    /// Client retry binding.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Server-owned fast-forward command with all required optimistic witnesses.
#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentFastForwardCommand {
    /// Destination course.
    pub course: CourseReference,
    /// Destination assignment and observed definition revision.
    pub assignment: ObservedAssignmentRevision,
    /// Import baseline revision observed by the preview.
    pub import_revision: CurriculumImportRevision,
    /// Source selected for re-read and exact-pin reauthorization.
    pub source: CurriculumSourceView,
    /// Course schedule witness preserves teaching-owned schedule state.
    pub preview_witness: CourseScheduleWitness,
    /// Client retry binding.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Server-owned command that preserves a divergent assignment and creates a separate draft.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateSourceDerivedAssignmentCommand {
    /// Existing destination course.
    pub course: CourseReference,
    /// Selected source and exact observed revision.
    pub source: CurriculumSourceView,
    /// Course schedule witness returned by preview.
    pub preview_witness: CourseScheduleWitness,
    /// Client retry binding.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Browser-safe result projection for a completed adoption write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumAdoptionResultView {
    /// Operation that completed.
    pub operation: CurriculumAdoptionOperation,
    /// Ordinary teaching course produced or updated.
    pub course: CourseReference,
    /// Assignment produced or updated when the operation has one direct assignment result.
    pub assignment: Option<AssignmentReference>,
    /// Resulting course term after the write.
    pub term: CourseTerm,
}

/// Browser-safe immutable receipt binding for a completed adoption operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumAdoptionReceiptBinding {
    /// Operation whose immutable receipt was persisted.
    pub operation: CurriculumAdoptionOperation,
    /// Client key bound to the stored request digest and completed receipt.
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

/// Whether a completed write was newly performed or loaded from a matching receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CurriculumReplayStatus {
    /// The Store performed and persisted the operation now.
    Applied,
    /// The Store returned the matching completed receipt without another mutation.
    Replayed,
}

/// Completed adoption result with the matching immutable receipt binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumAdoptionCompleted {
    /// Safe result projection for the visible Instructor workflow.
    pub result: CurriculumAdoptionResultView,
    /// Whether the result was applied now or replayed from durable evidence.
    pub replay: CurriculumReplayStatus,
    /// Immutable receipt binding for this completed request.
    pub receipt: CurriculumAdoptionReceiptBinding,
}
