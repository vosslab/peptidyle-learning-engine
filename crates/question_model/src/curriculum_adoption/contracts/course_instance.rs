//! CourseInstance browser previews and exact server witnesses.
//!
//! This module owns browser-safe inspection and preview values. Server-held apply records,
//! commands, and immutable receipt evidence live in focused sibling modules so a preview can
//! never also appear to be mutation authority.

use serde::{Deserialize, Serialize};

use super::{
    BlueprintAssignmentRevisionReference, BlueprintOperationRetryToken, BlueprintQuestionPosition,
    BlueprintRevisionReference, CourseInstanceOperationReceipt, CurriculumImportRevision,
    QuestionRevisionSubstitutions, ReplacementQuestionRevisionChoices,
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
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
    reserved_course: CourseInstanceReference,
}

impl CourseInstanceCreationReservation {
    pub fn for_blueprint(
        source: BlueprintRevisionReference,
        target_term: CourseTerm,
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: BlueprintOperationRetryToken,
        reserved_course: CourseInstanceReference,
    ) -> Self {
        Self {
            origin: CourseInstanceCreationOrigin::Blueprint(source),
            target_term,
            authorized_account,
            request_digest,
            idempotency_key,
            reserved_course,
        }
    }

    pub fn for_rollover(
        source: CourseInstanceSnapshot,
        target_term: CourseTerm,
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: BlueprintOperationRetryToken,
        reserved_course: CourseInstanceReference,
    ) -> Self {
        Self {
            origin: CourseInstanceCreationOrigin::Rollover(source),
            target_term,
            authorized_account,
            request_digest,
            idempotency_key,
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
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
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
    blueprint_content_digest: super::super::BlueprintContentDigest,
    assignment: AssignmentRevisionReference,
    import_revision: CurriculumImportRevision,
}

impl AssignmentSourceRecord {
    pub fn new(
        source: BlueprintAssignmentRevisionReference,
        replacements: QuestionRevisionSubstitutions,
        blueprint_content_digest: super::super::BlueprintContentDigest,
        assignment: AssignmentRevisionReference,
        import_revision: CurriculumImportRevision,
    ) -> Self {
        Self {
            source,
            replacements,
            blueprint_content_digest,
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
    pub fn blueprint_content_digest(&self) -> super::super::BlueprintContentDigest {
        self.blueprint_content_digest
    }
    pub fn assignment(&self) -> AssignmentRevisionReference {
        self.assignment
    }
    pub fn import_revision(&self) -> CurriculumImportRevision {
        self.import_revision
    }
}

/// Delivery effect of one controlled update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlledUpdateEffect {
    MeaningChanged,
    SourceRevisionOnly,
}

/// Exact server-only locator for an immutable assignment-import receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentImportReceiptTarget {
    receipt_account: AccountId,
    receipt_key: BlueprintOperationRetryToken,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    import_revision: CurriculumImportRevision,
}

impl AssignmentImportReceiptTarget {
    pub fn new(
        receipt_account: AccountId,
        receipt_key: BlueprintOperationRetryToken,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        import_revision: CurriculumImportRevision,
    ) -> Self {
        Self {
            receipt_account,
            receipt_key,
            course,
            assignment,
            import_revision,
        }
    }
    pub fn receipt_account(&self) -> AccountId {
        self.receipt_account
    }
    pub fn receipt_key(&self) -> &BlueprintOperationRetryToken {
        &self.receipt_key
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
pub enum CourseInstanceOperationBlocker {
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
    Blocked {
        blocker: CourseInstanceOperationBlocker,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ShiftCourseDatesReadiness {
    Ready,
    Blocked {
        blocker: CourseInstanceOperationBlocker,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ApplyBlueprintUpdateReadiness {
    Ready,
    Blocked {
        blocker: CourseInstanceOperationBlocker,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CopyAssignmentFromBlueprintReadiness {
    Ready,
    Blocked {
        blocker: CourseInstanceOperationBlocker,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintOperationReconciliationReadiness {
    Ready,
    Blocked {
        blocker: CourseInstanceOperationBlocker,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RolloverCourseInstancePreviewRequest {
    pub source_course: CourseInstanceReference,
    pub target_term: CourseTerm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ShiftCourseInstanceTermPreviewRequest {
    pub course: CourseInstanceReference,
    pub target_term: CourseTerm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ControlledUpdateBlueprintAssignmentPreviewRequest {
    pub course: CourseInstanceReference,
    pub source: BlueprintAssignmentRevisionReference,
    pub assignment: AssignmentReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateSelectedBlueprintAssignmentPreviewRequest {
    pub course: CourseInstanceReference,
    pub source: BlueprintAssignmentRevisionReference,
    pub replacements: QuestionRevisionSubstitutions,
}

/// Server-only repair intent for one retained CourseInstance receipt.
///
/// A repair has an independent audit and retry identity. Callers reuse the
/// key for a retry and issue a new key for a later repair action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionIntent {
    pub target: CourseInstanceOperationReceipt,
    pub idempotency_key: BlueprintOperationRetryToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RolloverReusableStateManifest {
    pub source: BlueprintRevisionReference,
    assignments: BoundedAssignmentDefinitionSources,
    schedules: BoundedResolvedScheduleSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RolloverReusableStateManifestError;

impl RolloverReusableStateManifest {
    /// Validates the complete reusable-only state copied into a rollover.
    pub fn new(
        source: BlueprintRevisionReference,
        assignments: Vec<BlueprintAssignmentRevisionReference>,
        schedules: Vec<ResolvedAssignmentSchedule>,
    ) -> Result<Self, RolloverReusableStateManifestError> {
        Ok(Self {
            source,
            assignments: BoundedAssignmentDefinitionSources::new(assignments)
                .map_err(|_| RolloverReusableStateManifestError)?,
            schedules: BoundedResolvedScheduleSet::new(schedules)
                .map_err(|_| RolloverReusableStateManifestError)?,
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

impl std::fmt::Display for RolloverReusableStateManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("rollover reusable evidence exceeds the course assignment bound")
    }
}
impl std::error::Error for RolloverReusableStateManifestError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RolloverExclusionPolicy {
    ExcludeAllStudentAndDeliveryRecords,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseRolloverManifest {
    pub copied: RolloverReusableStateManifest,
    pub exclusion_policy: RolloverExclusionPolicy,
}

impl CourseRolloverManifest {
    pub fn new(copied: RolloverReusableStateManifest) -> Self {
        Self {
            copied,
            exclusion_policy: RolloverExclusionPolicy::ExcludeAllStudentAndDeliveryRecords,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RolloverCourseInstancePreview {
    pub witness: CourseInstanceSnapshot,
    pub target_term: CourseTerm,
    pub manifest: CourseRolloverManifest,
    pub readiness: CopyCourseForNewTermReadiness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ShiftCourseInstanceTermPreview {
    pub witness: CourseInstanceSnapshot,
    pub target_term: CourseTerm,
    pub schedules: BoundedResolvedScheduleSet,
    pub readiness: ShiftCourseDatesReadiness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ControlledUpdateBlueprintAssignmentPreview {
    pub import: AssignmentSourceSnapshot,
    pub witness: CourseInstanceSnapshot,
    pub readiness: ApplyBlueprintUpdateReadiness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateSelectedBlueprintAssignmentPreview {
    pub source: BlueprintAssignmentRevisionReference,
    pub witness: CourseInstanceSnapshot,
    pub schedule: ResolvedAssignmentSchedule,
    pub readiness: CopyAssignmentFromBlueprintReadiness,
}

/// Server-only reconciliation projection bound to one immutable completed receipt.
///
/// This is not a browser DTO: Store code uses it to retain the receipt target while it
/// rechecks current derived projections and consumes the matching reconciliation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionPreview {
    receipt: super::CourseInstanceOperationReceipt,
    readiness: BlueprintOperationReconciliationReadiness,
}

impl ReconcileCourseInstanceAdoptionPreview {
    /// Creates a server-held reconciliation projection for exactly one receipt target.
    pub fn new(
        receipt: super::CourseInstanceOperationReceipt,
        readiness: BlueprintOperationReconciliationReadiness,
    ) -> Self {
        Self { receipt, readiness }
    }

    /// Returns the immutable completed operation selected for reconciliation.
    pub fn receipt(&self) -> &super::CourseInstanceOperationReceipt {
        &self.receipt
    }

    /// Returns the server-computed reconciliation readiness.
    pub fn readiness(&self) -> &BlueprintOperationReconciliationReadiness {
        &self.readiness
    }
}

/// Browser-safe completion for one next-term CourseInstance rollover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RolloverCourseInstanceCompleted {
    pub course: CourseInstanceReference,
}

/// Browser-safe completion for one atomic CourseInstance term shift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ShiftCourseInstanceTermCompleted {
    pub course: CourseInstanceReference,
}

/// Browser-safe completion for one controlled Blueprint assignment update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ControlledUpdateBlueprintAssignmentCompleted {
    pub course: CourseInstanceReference,
    pub assignment: AssignmentReference,
}

/// Browser-safe completion for one selected Blueprint assignment copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateSelectedBlueprintAssignmentCompleted {
    pub course: CourseInstanceReference,
    pub assignment: AssignmentReference,
}

/// Browser-safe completion for rebuilding derived projections from one receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReconcileCourseInstanceAdoptionCompleted {
    pub course: CourseInstanceReference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseInstanceCommandError {
    Blocked(CourseInstanceOperationBlocker),
    CreationWitnessMismatch,
    ScheduleEvidence(BoundedResolvedScheduleSetError),
    ControlledUpdateLineageMismatch,
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
            Self::Blocked { blocker } => Err(CourseInstanceCommandError::Blocked(blocker.clone())),
        }
    }
}

impl ShiftCourseDatesReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { blocker } => Err(CourseInstanceCommandError::Blocked(blocker.clone())),
        }
    }
}

impl ApplyBlueprintUpdateReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { blocker } => Err(CourseInstanceCommandError::Blocked(blocker.clone())),
        }
    }
}

impl CopyAssignmentFromBlueprintReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { blocker } => Err(CourseInstanceCommandError::Blocked(blocker.clone())),
        }
    }
}

impl BlueprintOperationReconciliationReadiness {
    pub(super) fn require_ready(&self) -> Result<(), CourseInstanceCommandError> {
        match self {
            Self::Ready => Ok(()),
            Self::Blocked { blocker } => Err(CourseInstanceCommandError::Blocked(blocker.clone())),
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
pub struct BoundedAssignmentDefinitionSources(Vec<BlueprintAssignmentRevisionReference>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedAssignmentDefinitionSourcesError;

impl BoundedAssignmentDefinitionSources {
    pub fn new(
        assignments: Vec<BlueprintAssignmentRevisionReference>,
    ) -> Result<Self, BoundedAssignmentDefinitionSourcesError> {
        (assignments.len() <= crate::MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .then_some(Self(assignments))
            .ok_or(BoundedAssignmentDefinitionSourcesError)
    }

    pub fn as_slice(&self) -> &[BlueprintAssignmentRevisionReference] {
        &self.0
    }
}

impl std::fmt::Display for BoundedAssignmentDefinitionSourcesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("rollover assignment-source evidence exceeds the course assignment bound")
    }
}
impl std::error::Error for BoundedAssignmentDefinitionSourcesError {}

impl Serialize for BoundedAssignmentDefinitionSources {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BoundedAssignmentDefinitionSources {
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
        formatter.write_str("resolved schedule collection exceeds the course assignment bound")
    }
}
impl std::error::Error for BoundedResolvedScheduleSetError {}
