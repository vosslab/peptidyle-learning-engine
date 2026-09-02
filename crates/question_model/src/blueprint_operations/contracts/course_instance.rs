//! CourseInstance browser previews and exact server witnesses.
//!
//! This module owns browser-safe inspection and preview values. Server-held apply records,
//! commands, and immutable receipt evidence live in focused sibling modules so a preview can
//! never also appear to be mutation authority.

use serde::{Deserialize, Serialize};

use super::{
    AssignmentImportReceipt, BlueprintAssignmentRevisionReference, BlueprintQuestionPosition,
    BlueprintRevisionReference, CurriculumImportRevision, QuestionRevisionSubstitutions,
    ReplacementQuestionRevisionChoices, RequestChecksum, RequestRetryToken,
};
use crate::{
    AccountId, AssignmentReference, AssignmentRevisionNumber, CourseInstanceReference,
    CourseScheduleRevisionReference, CourseTerm, QuestionRevisionReference,
    ResolvedAssignmentSchedule,
};

use super::bounded::{deserialize_assignment_sources, deserialize_course_instance_corrections};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AssignmentRevisionReference {
    pub assignment: AssignmentReference,
    pub revision_number: AssignmentRevisionNumber,
}

/// Exact destination scope and revision evidence observed by the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseInstanceSnapshot {
    pub course: CourseInstanceReference,
    pub schedule_revision: CourseScheduleRevisionReference,
    assignment_revisions: BoundedAssignmentRevisionReferences,
}

/// Immutable source history that established one Course Instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseOrigin {
    pub blueprint_revision: BlueprintRevisionReference,
    pub source_course: Option<CourseInstanceReference>,
}

impl CourseOrigin {
    /// Records direct creation from one exact Blueprint Revision.
    pub const fn from_blueprint(blueprint_revision: BlueprintRevisionReference) -> Self {
        Self {
            blueprint_revision,
            source_course: None,
        }
    }

    /// Records rollover from one exact Course Instance while retaining its Blueprint Revision.
    pub const fn from_rollover(
        blueprint_revision: BlueprintRevisionReference,
        source_course: CourseInstanceReference,
    ) -> Self {
        Self {
            blueprint_revision,
            source_course: Some(source_course),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInstanceSnapshotError {
    ScheduleRevisionCourseMismatch,
    TooManyOrRepeatedAssignmentRevisions,
}

impl CourseInstanceSnapshot {
    /// Creates exact bounded destination evidence retained by commands and receipts.
    pub fn new(
        course: CourseInstanceReference,
        schedule_revision: CourseScheduleRevisionReference,
        assignment_revisions: Vec<AssignmentRevisionReference>,
    ) -> Result<Self, CourseInstanceSnapshotError> {
        if schedule_revision.course != course {
            return Err(CourseInstanceSnapshotError::ScheduleRevisionCourseMismatch);
        }
        Ok(Self {
            course,
            schedule_revision,
            assignment_revisions: BoundedAssignmentRevisionReferences::new(assignment_revisions)
                .map_err(|_| CourseInstanceSnapshotError::TooManyOrRepeatedAssignmentRevisions)?,
        })
    }

    /// Returns the observed assignment revisions in their server-observed order.
    pub fn assignment_revisions(&self) -> &[AssignmentRevisionReference] {
        self.assignment_revisions.as_slice()
    }
}

impl std::fmt::Display for CourseInstanceSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScheduleRevisionCourseMismatch => formatter.write_str(
                "Course Schedule Revision Reference belongs to a different Course Instance",
            ),
            Self::TooManyOrRepeatedAssignmentRevisions => formatter.write_str(
                "course instance assignment evidence exceeds the course assignment bound",
            ),
        }
    }
}
impl std::error::Error for CourseInstanceSnapshotError {}

/// Server-only origin evidence for one reserved CourseInstance creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseInstanceCreationOrigin {
    Blueprint(BlueprintRevisionReference),
    Rollover(CourseInstanceSnapshot),
}

/// One server-reserved CourseInstance creation bound to an authenticated Instructor operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInstanceCreationReservation {
    origin: CourseInstanceCreationOrigin,
    target_term: CourseTerm,
    authorized_account: AccountId,
    request_checksum: RequestChecksum,
    retry_token: RequestRetryToken,
    reserved_course: CourseInstanceReference,
}

impl CourseInstanceCreationReservation {
    pub fn for_blueprint(
        source: BlueprintRevisionReference,
        target_term: CourseTerm,
        authorized_account: AccountId,
        request_checksum: RequestChecksum,
        retry_token: RequestRetryToken,
        reserved_course: CourseInstanceReference,
    ) -> Self {
        Self {
            origin: CourseInstanceCreationOrigin::Blueprint(source),
            target_term,
            authorized_account,
            request_checksum,
            retry_token,
            reserved_course,
        }
    }

    pub fn for_rollover(
        source: CourseInstanceSnapshot,
        target_term: CourseTerm,
        authorized_account: AccountId,
        request_checksum: RequestChecksum,
        retry_token: RequestRetryToken,
        reserved_course: CourseInstanceReference,
    ) -> Self {
        Self {
            origin: CourseInstanceCreationOrigin::Rollover(source),
            target_term,
            authorized_account,
            request_checksum,
            retry_token,
            reserved_course,
        }
    }

    pub fn origin(&self) -> &CourseInstanceCreationOrigin {
        &self.origin
    }
    pub fn matches_blueprint_source(&self, source: &BlueprintRevisionReference) -> bool {
        matches!(&self.origin, CourseInstanceCreationOrigin::Blueprint(value) if value == source)
    }
    pub fn matches_rollover_source(&self, source: &CourseInstanceSnapshot) -> bool {
        matches!(&self.origin, CourseInstanceCreationOrigin::Rollover(value) if value == source)
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn authorized_account(&self) -> AccountId {
        self.authorized_account
    }
    pub fn request_checksum(&self) -> RequestChecksum {
        self.request_checksum
    }
    pub fn retry_token(&self) -> &RequestRetryToken {
        &self.retry_token
    }
    pub fn reserved_course(&self) -> CourseInstanceReference {
        self.reserved_course
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AssignmentSourceSnapshot {
    pub source: BlueprintAssignmentRevisionReference,
    pub destination: AssignmentRevisionReference,
    pub import_revision: CurriculumImportRevision,
}

/// Server-only immutable evidence for one applied assignment import.
///
/// No Serde implementation is provided: a browser may request an update, but
/// cannot manufacture the post-mutation evidence that proves its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentSourceRecord {
    source: BlueprintAssignmentRevisionReference,
    replacements: QuestionRevisionSubstitutions,
    blueprint_content_checksum: super::super::BlueprintContentChecksum,
    assignment: AssignmentRevisionReference,
    import_revision: CurriculumImportRevision,
}

impl AssignmentSourceRecord {
    pub fn new(
        source: BlueprintAssignmentRevisionReference,
        replacements: QuestionRevisionSubstitutions,
        blueprint_content_checksum: super::super::BlueprintContentChecksum,
        assignment: AssignmentRevisionReference,
        import_revision: CurriculumImportRevision,
    ) -> Self {
        Self {
            source,
            replacements,
            blueprint_content_checksum,
            assignment,
            import_revision,
        }
    }
    pub fn source(&self) -> BlueprintAssignmentRevisionReference {
        self.source
    }
    pub fn replacements(&self) -> &QuestionRevisionSubstitutions {
        &self.replacements
    }
    pub fn blueprint_content_checksum(&self) -> super::super::BlueprintContentChecksum {
        self.blueprint_content_checksum
    }
    pub fn assignment(&self) -> AssignmentRevisionReference {
        self.assignment
    }
    pub fn import_revision(&self) -> CurriculumImportRevision {
        self.import_revision
    }
}

/// Delivery effect of one applied Blueprint update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyBlueprintUpdateEffect {
    MeaningChanged,
    SourceRevisionOnly,
}

/// Exact server-only locator for an immutable assignment-import receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentImportReceiptTarget {
    receipt_account: AccountId,
    retry_token: RequestRetryToken,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    import_revision: CurriculumImportRevision,
}

impl AssignmentImportReceiptTarget {
    pub fn new(
        receipt_account: AccountId,
        retry_token: RequestRetryToken,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        import_revision: CurriculumImportRevision,
    ) -> Self {
        Self {
            receipt_account,
            retry_token,
            course,
            assignment,
            import_revision,
        }
    }
    pub fn receipt_account(&self) -> AccountId {
        self.receipt_account
    }
    pub fn retry_token(&self) -> &RequestRetryToken {
        &self.retry_token
    }
    pub fn course(&self) -> CourseInstanceReference {
        self.course
    }
    pub fn assignment(&self) -> AssignmentReference {
        self.assignment
    }
    pub fn import_revision(&self) -> CurriculumImportRevision {
        self.import_revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseInstanceScheduleField {
    AvailableAt,
    DueAt,
    ClosesAt,
    Schedule,
    TargetTerm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseInstanceScheduleReason {
    OutsideTargetTerm,
    NonexistentLocalTime,
    AmbiguousLocalTime,
    OutOfOrder,
    TimestampOutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseInstanceScheduleCorrection {
    pub field: CourseInstanceScheduleField,
    pub reason: CourseInstanceScheduleReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct UnavailableQuestionRevisionRecovery {
    pub source: BlueprintAssignmentRevisionReference,
    pub position: BlueprintQuestionPosition,
    pub unavailable: QuestionRevisionReference,
    pub choices: ReplacementQuestionRevisionChoices,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct AssignmentSource {
    pub source: BlueprintAssignmentRevisionReference,
    pub import_revision: CurriculumImportRevision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseInstanceBlueprintInspectionView {
    pub initial_course_origin: CourseOrigin,
    pub witness: CourseInstanceSnapshot,
    #[serde(deserialize_with = "deserialize_assignment_sources")]
    pub assignment_sources: Vec<AssignmentSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CopyCourseForNewTermIssue {
    IssuedWork {
        course: CourseInstanceReference,
    },
    Divergent {
        assignment: AssignmentReference,
    },
    SourceRevisionDrift {
        source: BlueprintRevisionReference,
    },
    ScheduleCorrectionsRequired {
        #[serde(deserialize_with = "deserialize_course_instance_corrections")]
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailableQuestionRevision {
        recovery: UnavailableQuestionRevisionRecovery,
    },
    ReceiptUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CopyCourseForNewTermReadiness {
    Ready,
    Blocked { issue: CopyCourseForNewTermIssue },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShiftCourseDatesIssue {
    IssuedWork {
        course: CourseInstanceReference,
    },
    Divergent {
        assignment: AssignmentReference,
    },
    SourceRevisionDrift {
        source: BlueprintRevisionReference,
    },
    ScheduleCorrectionsRequired {
        #[serde(deserialize_with = "deserialize_course_instance_corrections")]
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailableQuestionRevision {
        recovery: UnavailableQuestionRevisionRecovery,
    },
    ReceiptUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShiftCourseDatesReadiness {
    Ready,
    Blocked { issue: ShiftCourseDatesIssue },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ApplyBlueprintUpdateIssue {
    IssuedWork {
        course: CourseInstanceReference,
    },
    Divergent {
        assignment: AssignmentReference,
    },
    SourceRevisionDrift {
        source: BlueprintRevisionReference,
    },
    ScheduleCorrectionsRequired {
        #[serde(deserialize_with = "deserialize_course_instance_corrections")]
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailableQuestionRevision {
        recovery: UnavailableQuestionRevisionRecovery,
    },
    ReceiptUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ApplyBlueprintUpdateReadiness {
    Ready,
    Blocked { issue: ApplyBlueprintUpdateIssue },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CopyAssignmentFromBlueprintIssue {
    IssuedWork {
        course: CourseInstanceReference,
    },
    Divergent {
        assignment: AssignmentReference,
    },
    SourceRevisionDrift {
        source: BlueprintRevisionReference,
    },
    ScheduleCorrectionsRequired {
        #[serde(deserialize_with = "deserialize_course_instance_corrections")]
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailableQuestionRevision {
        recovery: UnavailableQuestionRevisionRecovery,
    },
    ReceiptUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CopyAssignmentFromBlueprintReadiness {
    Ready,
    Blocked {
        issue: CopyAssignmentFromBlueprintIssue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentImportRepairReadiness {
    Ready,
    Blocked { issue: AssignmentImportRepairIssue },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentImportRepairIssue {
    IssuedWork {
        course: CourseInstanceReference,
    },
    Divergent {
        assignment: AssignmentReference,
    },
    SourceRevisionDrift {
        source: BlueprintRevisionReference,
    },
    ScheduleCorrectionsRequired {
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailableQuestionRevision {
        recovery: UnavailableQuestionRevisionRecovery,
    },
    ReceiptUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CopyCourseForNewTermPreviewRequest {
    pub source_course: CourseInstanceReference,
    pub target_term: CourseTerm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ShiftCourseDatesPreviewRequest {
    pub course: CourseInstanceReference,
    pub target_term: CourseTerm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ApplyBlueprintUpdatePreviewRequest {
    pub course: CourseInstanceReference,
    pub source: BlueprintAssignmentRevisionReference,
    pub assignment: AssignmentReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CopyAssignmentFromBlueprintPreviewRequest {
    pub course: CourseInstanceReference,
    pub source: BlueprintAssignmentRevisionReference,
    pub replacements: QuestionRevisionSubstitutions,
}

/// Server-only repair intent for one retained Assignment import receipt.
///
/// A repair has an independent audit and retry identity. Callers reuse the
/// key for a retry and issue a new key for a later repair action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentImportRepairIntent {
    pub original_import_receipt: AssignmentImportReceipt,
    pub retry_token: RequestRetryToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseRolloverCopiedState {
    pub source: BlueprintRevisionReference,
    assignments: BoundedAssignmentContentSources,
    schedules: BoundedResolvedScheduleSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseRolloverCopiedStateError;

impl CourseRolloverCopiedState {
    /// Validates the complete reusable-only state copied into a rollover.
    pub fn new(
        source: BlueprintRevisionReference,
        assignments: Vec<BlueprintAssignmentRevisionReference>,
        schedules: Vec<ResolvedAssignmentSchedule>,
    ) -> Result<Self, CourseRolloverCopiedStateError> {
        Ok(Self {
            source,
            assignments: BoundedAssignmentContentSources::new(assignments)
                .map_err(|_| CourseRolloverCopiedStateError)?,
            schedules: BoundedResolvedScheduleSet::new(schedules)
                .map_err(|_| CourseRolloverCopiedStateError)?,
        })
    }

    /// Returns copied source locations in their original reusable-course order.
    pub fn assignments(&self) -> &[BlueprintAssignmentRevisionReference] {
        self.assignments.as_slice()
    }

    /// Returns resolved reusable schedules in their original reusable-course order.
    pub fn schedules(&self) -> &[ResolvedAssignmentSchedule] {
        self.schedules.as_slice()
    }
}

impl std::fmt::Display for CourseRolloverCopiedStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("rollover reusable evidence exceeds the course assignment bound")
    }
}
impl std::error::Error for CourseRolloverCopiedStateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RolloverExclusionPolicy {
    ExcludeAllStudentAndDeliveryRecords,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseRolloverManifest {
    pub copied: CourseRolloverCopiedState,
    pub exclusion_policy: RolloverExclusionPolicy,
}

impl CourseRolloverManifest {
    pub fn new(copied: CourseRolloverCopiedState) -> Self {
        Self {
            copied,
            exclusion_policy: RolloverExclusionPolicy::ExcludeAllStudentAndDeliveryRecords,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CopyCourseForNewTermPreview {
    pub witness: CourseInstanceSnapshot,
    pub target_term: CourseTerm,
    pub manifest: CourseRolloverManifest,
    pub readiness: CopyCourseForNewTermReadiness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ShiftCourseDatesPreview {
    pub witness: CourseInstanceSnapshot,
    pub target_term: CourseTerm,
    pub schedules: BoundedResolvedScheduleSet,
    pub readiness: ShiftCourseDatesReadiness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ApplyBlueprintUpdatePreview {
    pub import: AssignmentSourceSnapshot,
    pub witness: CourseInstanceSnapshot,
    pub readiness: ApplyBlueprintUpdateReadiness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CopyAssignmentFromBlueprintPreview {
    pub source: BlueprintAssignmentRevisionReference,
    pub witness: CourseInstanceSnapshot,
    pub schedule: ResolvedAssignmentSchedule,
    pub readiness: CopyAssignmentFromBlueprintReadiness,
}

/// Server-only Assignment import repair projection bound to one immutable receipt.
///
/// This is not a browser DTO: Store code retains the original import receipt while it
/// rechecks current derived projections and consumes the matching repair record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentImportRepairPreview {
    original_import_receipt: AssignmentImportReceipt,
    readiness: AssignmentImportRepairReadiness,
}

impl AssignmentImportRepairPreview {
    /// Creates a server-held repair projection for exactly one Assignment import.
    pub fn new(
        original_import_receipt: AssignmentImportReceipt,
        readiness: AssignmentImportRepairReadiness,
    ) -> Self {
        Self {
            original_import_receipt,
            readiness,
        }
    }

    /// Returns the immutable Assignment import selected for repair.
    pub fn original_import_receipt(&self) -> &AssignmentImportReceipt {
        &self.original_import_receipt
    }

    /// Returns the server-computed repair readiness.
    pub fn readiness(&self) -> &AssignmentImportRepairReadiness {
        &self.readiness
    }
}

/// Browser-safe completion for one next-term CourseInstance rollover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CopyCourseForNewTermCompleted {
    pub course: CourseInstanceReference,
}

/// Browser-safe completion for one atomic CourseInstance term shift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ShiftCourseDatesCompleted {
    pub course: CourseInstanceReference,
}

/// Browser-safe completion for one applied Blueprint update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ApplyBlueprintUpdateCompleted {
    pub course: CourseInstanceReference,
    pub assignment: AssignmentReference,
}

/// Browser-safe completion for copying one Blueprint Assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CopyAssignmentFromBlueprintCompleted {
    pub course: CourseInstanceReference,
    pub assignment: AssignmentReference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseInstanceCommandError {
    Blocked,
    CreationWitnessMismatch,
    ScheduleEvidence(BoundedResolvedScheduleSetError),
    BlueprintUpdateLineageMismatch,
    DestinationAssignmentMissing,
    ReceiptBindingMismatch,
}

impl std::fmt::Display for CourseInstanceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("course instance operation is not eligible for apply")
    }
}
impl std::error::Error for CourseInstanceCommandError {}

impl CopyCourseForNewTermReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { .. } => Err(CourseInstanceCommandError::Blocked),
        }
    }
}

impl ShiftCourseDatesReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { .. } => Err(CourseInstanceCommandError::Blocked),
        }
    }
}

impl ApplyBlueprintUpdateReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { .. } => Err(CourseInstanceCommandError::Blocked),
        }
    }
}

impl CopyAssignmentFromBlueprintReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { .. } => Err(CourseInstanceCommandError::Blocked),
        }
    }
}

impl AssignmentImportRepairReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { .. } => Err(CourseInstanceCommandError::Blocked),
        }
    }
}

/// Checked, immutable schedule evidence retained by server apply records and receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedResolvedScheduleSet(Vec<ResolvedAssignmentSchedule>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedResolvedScheduleSetError;

impl BoundedResolvedScheduleSet {
    pub fn new(
        schedules: Vec<ResolvedAssignmentSchedule>,
    ) -> Result<Self, BoundedResolvedScheduleSetError> {
        (schedules.len() <= crate::MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .then_some(Self(schedules))
            .ok_or(BoundedResolvedScheduleSetError)
    }
    pub fn as_slice(&self) -> &[ResolvedAssignmentSchedule] {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Serialize for BoundedResolvedScheduleSet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BoundedResolvedScheduleSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        super::bounded::deserialize_bounded_vec::<D, _, { crate::MAX_ASSIGNMENT_ORDERED_ENTRIES }>(
            deserializer,
        )
        .map(Self)
    }
}

/// Checked assignment-revision evidence retained by CourseInstance records and receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedAssignmentRevisionReferences(Vec<AssignmentRevisionReference>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedAssignmentRevisionReferencesError;

impl BoundedAssignmentRevisionReferences {
    pub fn new(
        assignment_revisions: Vec<AssignmentRevisionReference>,
    ) -> Result<Self, BoundedAssignmentRevisionReferencesError> {
        if assignment_revisions.len() > crate::MAX_ASSIGNMENT_ORDERED_ENTRIES
            || assignment_revisions
                .iter()
                .enumerate()
                .any(|(index, assignment_revision)| {
                    assignment_revisions[..index]
                        .iter()
                        .any(|prior| prior.assignment == assignment_revision.assignment)
                })
        {
            return Err(BoundedAssignmentRevisionReferencesError);
        }
        Ok(Self(assignment_revisions))
    }

    pub fn as_slice(&self) -> &[AssignmentRevisionReference] {
        &self.0
    }
}

impl std::fmt::Display for BoundedAssignmentRevisionReferencesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("course instance assignment evidence exceeds the course assignment bound")
    }
}
impl std::error::Error for BoundedAssignmentRevisionReferencesError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CourseScheduleRevisionNumber, CourseScheduleRevisionReference};

    #[test]
    fn assignment_revision_reference_keeps_assignment_and_immutable_revision_number_together() {
        let reference = AssignmentRevisionReference {
            assignment: "A-7".parse().expect("Assignment reference"),
            revision_number: AssignmentRevisionNumber::new(3).expect("revision number"),
        };
        assert_eq!(
            serde_json::to_value(reference).expect("portable reference"),
            serde_json::json!({"assignment": "A-7", "revision_number": "3"})
        );
    }

    #[test]
    fn course_instance_snapshot_refuses_a_schedule_revision_from_another_course() {
        let course = CourseInstanceReference::new(7).expect("course");
        let other_course = CourseInstanceReference::new(8).expect("other course");
        let revision = CourseScheduleRevisionReference::new(
            other_course,
            CourseScheduleRevisionNumber::new(1).expect("positive revision"),
        );

        assert_eq!(
            CourseInstanceSnapshot::new(course, revision, vec![]),
            Err(CourseInstanceSnapshotError::ScheduleRevisionCourseMismatch)
        );
    }
}

impl Serialize for BoundedAssignmentRevisionReferences {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BoundedAssignmentRevisionReferences {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        super::bounded::deserialize_bounded_vec::<D, _, { crate::MAX_ASSIGNMENT_ORDERED_ENTRIES }>(
            deserializer,
        )
        .and_then(|assignments| Self::new(assignments).map_err(serde::de::Error::custom))
    }
}

/// Checked reusable source-location evidence retained by rollover records and receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedAssignmentContentSources(Vec<BlueprintAssignmentRevisionReference>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedAssignmentContentSourcesError;

impl BoundedAssignmentContentSources {
    pub fn new(
        assignments: Vec<BlueprintAssignmentRevisionReference>,
    ) -> Result<Self, BoundedAssignmentContentSourcesError> {
        (assignments.len() <= crate::MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .then_some(Self(assignments))
            .ok_or(BoundedAssignmentContentSourcesError)
    }

    pub fn as_slice(&self) -> &[BlueprintAssignmentRevisionReference] {
        &self.0
    }
}

impl std::fmt::Display for BoundedAssignmentContentSourcesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("rollover assignment-source evidence exceeds the course assignment bound")
    }
}
impl std::error::Error for BoundedAssignmentContentSourcesError {}

impl Serialize for BoundedAssignmentContentSources {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BoundedAssignmentContentSources {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        super::bounded::deserialize_bounded_vec::<D, _, { crate::MAX_ASSIGNMENT_ORDERED_ENTRIES }>(
            deserializer,
        )
        .map(Self)
    }
}

impl std::fmt::Display for BoundedResolvedScheduleSetError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("resolved schedule set exceeds the course assignment bound")
    }
}
impl std::error::Error for BoundedResolvedScheduleSetError {}
