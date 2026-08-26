//! Typed browser-safe previews and server-owned commands for B2 adoption.

use std::collections::BTreeSet;
use std::num::NonZeroU64;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, de};

use super::{CourseScheduleRevision, ResolvedRelativeAssignmentSchedule};
use crate::{
    AlphaCourseReference, AlphaCourseRevision, AssignmentReference, AssignmentRevision,
    AssignmentTeachingSettingsFailureCode, AssignmentTeachingSettingsLocalError,
    AssignmentTeachingSettingsValidationFailure, BlueprintReference, BlueprintRevision,
    CourseReference, CourseTerm, MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP,
    MAX_ASSIGNMENT_ORDERED_ENTRIES, MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES, QuestionId,
    ReusableCurriculumTitleError, validate_reusable_curriculum_title,
};

mod assignment_source;
mod bounded;
mod commands;
mod completed;

pub use assignment_source::{
    AssignmentDefinitionSourceView, ObservedAlphaAssignmentSource,
    ObservedAlphaAssignmentSourceError,
};
use bounded::{
    deserialize_assignment_witnesses, deserialize_pin_replacements,
    deserialize_replacement_questions,
};
pub use commands::{
    AlphaInstantiationCommand, AssignmentFastForwardCommand, BlueprintInstantiationCommand,
    CourseRolloverCommand, CourseTermShiftCommand, CreateSourceDerivedAssignmentCommand,
    CurriculumAdoptionCommandError, ForkAlphaCommand,
};
pub use completed::{
    AlphaInstantiationCompleted, AssignmentFastForwardCompleted, BlueprintInstantiationCompleted,
    CourseRolloverCompleted, CourseTermShiftCompleted, CurriculumAdoptionReceiptBinding,
    CurriculumReplayStatus, ForkAlphaCompleted, SourceDerivedAssignmentCompleted,
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

/// Strong revision evidence for one durable curriculum import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurriculumImportRevision(NonZeroU64);

impl CurriculumImportRevision {
    /// Builds a positive revision that fits PostgreSQL `BIGINT`.
    pub fn new(value: u64) -> Option<Self> {
        (value > 0 && value <= i64::MAX as u64).then_some(Self(NonZeroU64::new(value)?))
    }

    /// Returns the exact persistence revision scalar.
    pub fn value(self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Display for CurriculumImportRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.value())
    }
}

impl FromStr for CurriculumImportRevision {
    type Err = CurriculumImportRevisionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(CurriculumImportRevisionError);
        }
        value
            .parse()
            .ok()
            .and_then(Self::new)
            .ok_or(CurriculumImportRevisionError)
    }
}

impl TryFrom<String> for CurriculumImportRevision {
    type Error = CurriculumImportRevisionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<CurriculumImportRevision> for String {
    fn from(value: CurriculumImportRevision) -> Self {
        value.to_string()
    }
}

/// An import revision was not one canonical positive PostgreSQL-bigint decimal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumImportRevisionError;

impl std::fmt::Display for CurriculumImportRevisionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum import revision must be a canonical positive decimal")
    }
}

impl std::error::Error for CurriculumImportRevisionError {}

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
    #[serde(deserialize_with = "deserialize_assignment_witnesses")]
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
    /// Builds a bounded deterministic witness, rejecting duplicate assignment bindings.
    pub fn new(
        course: CourseReference,
        schedule_revision: CourseScheduleRevision,
        mut assignment_revisions: Vec<ObservedAssignmentRevision>,
    ) -> Result<Self, CourseScheduleWitnessError> {
        if assignment_revisions.len() > MAX_ASSIGNMENT_ORDERED_ENTRIES {
            return Err(CourseScheduleWitnessError::TooManyAssignments);
        }
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
    /// The witness exceeded the shared bound for one course operation.
    TooManyAssignments,
    /// One assignment had more than one revision in the same preview witness.
    DuplicateAssignment,
}

impl std::fmt::Display for CourseScheduleWitnessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyAssignments => {
                formatter.write_str("course schedule witness has too many assignments")
            }
            Self::DuplicateAssignment => {
                formatter.write_str("course schedule witness repeats an assignment")
            }
        }
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

/// Preview request for an independent Alpha fork. It has no teaching-course fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForkAlphaPreviewRequest {
    /// Public source observed under approved-Instructor authority.
    pub source: ObservedAlphaSource,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// Preview request for one Blueprint instantiation into an existing teaching course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintInstantiationPreviewRequest {
    /// Owner-scoped source and exact revision.
    pub source: ObservedBlueprintSource,
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// Target term used to resolve reusable schedule defaults.
    pub target_term: CourseTerm,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// Preview request for one Alpha instantiation into a new teaching course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlphaInstantiationPreviewRequest {
    /// Public source and exact revision.
    pub source: ObservedAlphaSource,
    /// Validated title proposed for the new ordinary teaching course.
    pub title: CurriculumAdoptionTitle,
    /// Explicit target term for every new course schedule.
    pub target_term: CourseTerm,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// Preview request for an ordinary teaching-course rollover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseRolloverPreviewRequest {
    /// Source-course schedule and assignment revision witness.
    pub witness: CourseScheduleWitness,
    /// Validated title proposed for the new ordinary teaching course.
    pub title: CurriculumAdoptionTitle,
    /// Explicit target term for the new course.
    pub target_term: CourseTerm,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// Preview request for an atomic whole-course term shift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseTermShiftPreviewRequest {
    /// Current course and assignment revisions that apply must repeat.
    pub witness: CourseScheduleWitness,
    /// Replacement term for the existing unissued course.
    pub target_term: CourseTerm,
}

/// Preview request for an imported assignment fast-forward decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentFastForwardPreviewRequest {
    /// Destination course that owns the import.
    pub course: CourseReference,
    /// Destination assignment revision observed by the Instructor.
    pub assignment: ObservedAssignmentRevision,
    /// Durable import baseline revision observed by the Instructor.
    pub import_revision: CurriculumImportRevision,
    /// Re-readable source revision selected for comparison.
    pub source: AssignmentDefinitionSourceView,
}

/// Preview request for a separate source-derived assignment after divergence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDerivedAssignmentPreviewRequest {
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// Source definition selected for the new independent draft.
    pub source: AssignmentDefinitionSourceView,
    /// Explicit public-question substitutions accumulated through preview corrections.
    pub replacements: CurriculumPinReplacements,
}

/// One answer-free assignment schedule row for an already existing destination assignment.
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

/// One answer-free row prepared before a destination assignment has a route reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedCurriculumAssignmentView {
    /// Instructor-visible title that will be copied into the new assignment.
    pub title: CurriculumAdoptionTitle,
    /// Server-resolved target-term schedule projection.
    pub schedule: ResolvedRelativeAssignmentSchedule,
}

/// One answer-free course preview prepared before a destination course has a route reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedCurriculumCourseView {
    /// Instructor-visible course title that will be copied into the new course.
    pub title: CurriculumAdoptionTitle,
    /// New ordinary assignment definitions in source order.
    pub assignments: Vec<PreparedCurriculumAssignmentView>,
}

/// Fork preview naming the source and the resulting independent Alpha exactly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForkAlphaPreviewView {
    /// Public Alpha source selected for the fork.
    pub source: ObservedAlphaSource,
    /// Resulting independent Alpha title copied exactly from the source.
    pub resulting_alpha_title: CurriculumAdoptionTitle,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}

/// Blueprint-instantiation preview for one existing teaching course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlueprintInstantiationPreviewView {
    /// Selected source and observed revision.
    pub source: ObservedBlueprintSource,
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// Target term that resolved the prepared schedule.
    pub target_term: CourseTerm,
    /// Current course/assignment revisions required by apply.
    pub witness: CourseScheduleWitness,
    /// Prepared assignment before it has a destination reference.
    pub assignment: PreparedCurriculumAssignmentView,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}

/// Alpha-instantiation preview for one new teaching course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlphaInstantiationPreviewView {
    /// Selected public source and observed revision.
    pub source: ObservedAlphaSource,
    /// Explicit target term.
    pub target_term: CourseTerm,
    /// Prepared course before it has a destination reference.
    pub course: PreparedCurriculumCourseView,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}

/// Rollover preview for a new ordinary course.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseRolloverPreviewView {
    /// Source course and all revisions that apply must repeat.
    pub witness: CourseScheduleWitness,
    /// Explicit target term.
    pub target_term: CourseTerm,
    /// Prepared new course before it has a destination reference.
    pub course: PreparedCurriculumCourseView,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
}

/// Whole-course term-shift preview for existing destination assignments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CourseTermShiftPreviewView {
    /// Course and assignment revisions required by apply.
    pub witness: CourseScheduleWitness,
    /// Target term selected for the existing course.
    pub target_term: CourseTerm,
    /// Existing assignments with resolved target-term schedules.
    pub assignments: Vec<CurriculumAssignmentView>,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
}

/// Exact bounded semantic position of one replaceable source pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "CurriculumPinPositionParts")]
pub struct CurriculumPinPosition {
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    candidate_index: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CurriculumPinPositionParts {
    module_index: Option<u16>,
    assignment_index: u16,
    entry_index: u16,
    candidate_index: Option<u16>,
}

impl TryFrom<CurriculumPinPositionParts> for CurriculumPinPosition {
    type Error = CurriculumPinPositionError;

    fn try_from(value: CurriculumPinPositionParts) -> Result<Self, Self::Error> {
        Self::new(
            value.module_index,
            value.assignment_index,
            value.entry_index,
            value.candidate_index,
        )
    }
}

impl CurriculumPinPosition {
    /// Validates zero-based module, assignment, entry, and optional pool-candidate coordinates.
    pub fn new(
        module_index: Option<u16>,
        assignment_index: u16,
        entry_index: u16,
        candidate_index: Option<u16>,
    ) -> Result<Self, CurriculumPinPositionError> {
        let bound = u16::try_from(MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .expect("assignment position bound fits u16");
        let candidate_bound = u16::try_from(MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP)
            .expect("candidate position bound fits u16");
        if assignment_index >= bound
            || entry_index >= bound
            || module_index.is_some_and(|index| index >= bound)
            || candidate_index.is_some_and(|index| index >= candidate_bound)
        {
            return Err(CurriculumPinPositionError);
        }
        Ok(Self {
            module_index,
            assignment_index,
            entry_index,
            candidate_index,
        })
    }

    /// Returns the optional zero-based Alpha module position.
    pub fn module_index(self) -> Option<u16> {
        self.module_index
    }

    /// Returns the zero-based assignment position within its source scope.
    pub fn assignment_index(self) -> u16 {
        self.assignment_index
    }

    /// Returns the zero-based fixed-item or pool entry position.
    pub fn entry_index(self) -> u16 {
        self.entry_index
    }

    /// Returns the zero-based pool candidate position, or `None` for one fixed item.
    pub fn candidate_index(self) -> Option<u16> {
        self.candidate_index
    }
}

/// A pin position exceeded a reusable ordering or pool-candidate bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumPinPositionError;

impl std::fmt::Display for CurriculumPinPositionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum pin position is outside the reusable ordering bound")
    }
}

impl std::error::Error for CurriculumPinPositionError {}

/// One explicit public-question substitution for an exact semantic pin position.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurriculumPinReplacement {
    /// Exact fixed-item or pool-candidate coordinate selected by the server preview.
    pub position: CurriculumPinPosition,
    /// Public Question ID selected through the shared ProblemPicker.
    pub question: QuestionId,
}

/// Bounded unique substitutions accumulated while correcting one adoption preview.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<CurriculumPinReplacement>")]
pub struct CurriculumPinReplacements(Vec<CurriculumPinReplacement>);

impl<'de> Deserialize<'de> for CurriculumPinReplacements {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = deserialize_pin_replacements(deserializer)?;
        Self::new(values).map_err(de::Error::custom)
    }
}

impl CurriculumPinReplacements {
    /// Validates unique exact positions within the total source-selection bound.
    pub fn new(
        mut values: Vec<CurriculumPinReplacement>,
    ) -> Result<Self, CurriculumPinReplacementsError> {
        if values.len() > MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES {
            return Err(CurriculumPinReplacementsError);
        }
        values.sort_unstable_by_key(|value| value.position);
        if values
            .windows(2)
            .any(|pair| pair[0].position == pair[1].position)
        {
            return Err(CurriculumPinReplacementsError);
        }
        Ok(Self(values))
    }

    /// Returns substitutions in the Instructor-confirmed order echoed by preview.
    pub fn as_slice(&self) -> &[CurriculumPinReplacement] {
        &self.0
    }
}

impl TryFrom<Vec<CurriculumPinReplacement>> for CurriculumPinReplacements {
    type Error = CurriculumPinReplacementsError;

    fn try_from(value: Vec<CurriculumPinReplacement>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CurriculumPinReplacements> for Vec<CurriculumPinReplacement> {
    fn from(value: CurriculumPinReplacements) -> Self {
        value.0
    }
}

/// Pin substitutions exceeded the bound or repeated one exact semantic position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurriculumPinReplacementsError;

impl std::fmt::Display for CurriculumPinReplacementsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("curriculum pin replacements are invalid")
    }
}

impl std::error::Error for CurriculumPinReplacementsError {}

/// Validated answer-free candidate question IDs for one explicit replacement action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "Vec<QuestionId>")]
pub struct ReplacementQuestionChoices(Vec<QuestionId>);

impl<'de> Deserialize<'de> for ReplacementQuestionChoices {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values = deserialize_replacement_questions(deserializer)?;
        Self::new(values).map_err(de::Error::custom)
    }
}

impl ReplacementQuestionChoices {
    /// Validates nonempty unique public candidate IDs within the existing pool bound.
    pub fn new(values: Vec<QuestionId>) -> Result<Self, ReplacementQuestionChoicesError> {
        if values.is_empty() || values.len() > MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP {
            return Err(ReplacementQuestionChoicesError);
        }
        if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err(ReplacementQuestionChoicesError);
        }
        Ok(Self(values))
    }

    /// Returns public candidate question IDs in server-selected recovery order.
    pub fn as_slice(&self) -> &[QuestionId] {
        &self.0
    }
}

impl TryFrom<Vec<QuestionId>> for ReplacementQuestionChoices {
    type Error = ReplacementQuestionChoicesError;

    fn try_from(value: Vec<QuestionId>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ReplacementQuestionChoices> for Vec<QuestionId> {
    fn from(value: ReplacementQuestionChoices) -> Self {
        value.0
    }
}

/// Replacement candidates were empty, duplicated, or above the pool bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplacementQuestionChoicesError;

impl std::fmt::Display for ReplacementQuestionChoicesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("replacement question choices are invalid")
    }
}

impl std::error::Error for ReplacementQuestionChoicesError {}

/// Explicit recovery that preserves an assignment whose reusable meaning or evidence is fixed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PreservedAssignmentRecoveryAction {
    /// Preserve the divergent assignment and create a new source-derived draft.
    CreateSourceDerivedAssignment,
}

/// Explicit replacement action for one source pin unavailable to the destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum UnavailablePinRecoveryAction {
    /// Choose one public replacement question for a pin that cannot be reauthorized.
    SelectReplacementQuestion {
        /// Bounded reusable source position containing the unavailable pin.
        position: CurriculumPinPosition,
        /// Public catalog question IDs suitable for the explicit replacement flow.
        candidates: ReplacementQuestionChoices,
    },
}

/// Structured outcome of an assignment fast-forward preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum AssignmentFastForwardDecision {
    /// All source, baseline, issued-work, and exact-pin checks permit a fast-forward.
    Eligible,
    /// Destination reusable meaning changed; preserve it and create a separate draft.
    Divergent {
        /// Preserve the current assignment and create an independent source-derived draft.
        recovery: PreservedAssignmentRecoveryAction,
    },
    /// A required source pin cannot be reauthorized for new destination use.
    UnavailablePin {
        /// Choose an authorized public replacement for the exact unavailable position.
        recovery: UnavailablePinRecoveryAction,
    },
    /// The source changed or the observed source revision no longer matches.
    SourceRevisionDrift {
        /// Current exact assignment-definition source returned for a corrected preview.
        source: AssignmentDefinitionSourceView,
    },
    /// Learner work was issued, so the existing assignment retains its immutable evidence context.
    IssuedWork {
        /// Preserve the issued assignment and create an independent source-derived draft.
        recovery: PreservedAssignmentRecoveryAction,
    },
}

/// Fast-forward preview with one structured, recoverable decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssignmentFastForwardPreviewView {
    /// Destination course.
    pub course: CourseReference,
    /// Destination assignment and observed definition revision.
    pub assignment: ObservedAssignmentRevision,
    /// Import baseline revision.
    pub import_revision: CurriculumImportRevision,
    /// Source selected for re-read and comparison.
    pub source: AssignmentDefinitionSourceView,
    /// Schedule/assignment witness preserved through an eligible apply.
    pub witness: CourseScheduleWitness,
    /// Explicit result and available recovery action.
    pub decision: AssignmentFastForwardDecision,
}

/// Preview for a new independent source-derived assignment before it has a reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDerivedAssignmentPreviewView {
    /// Existing teaching-course destination.
    pub course: CourseReference,
    /// Selected re-readable source.
    pub source: AssignmentDefinitionSourceView,
    /// Current course and assignment revisions required by apply.
    pub witness: CourseScheduleWitness,
    /// Prepared independent assignment before it has a destination reference.
    pub assignment: PreparedCurriculumAssignmentView,
    /// Validated substitutions already applied to the prepared semantic snapshot.
    pub replacements: CurriculumPinReplacements,
    /// Field-specific schedule corrections, if any.
    pub corrections: Vec<CurriculumScheduleCorrection>,
    /// First exact source pin requiring an explicit authorized replacement.
    pub pin_correction: Option<UnavailablePinRecoveryAction>,
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
    pub source: AssignmentDefinitionSourceView,
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
