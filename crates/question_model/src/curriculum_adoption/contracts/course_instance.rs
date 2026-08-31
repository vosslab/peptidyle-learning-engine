//! CourseInstance browser previews and exact server witnesses.
//!
//! This module owns browser-safe inspection and preview values. Server-held apply records,
//! commands, and immutable receipt evidence live in focused sibling modules so a preview can
//! never also appear to be mutation authority.

use serde::{Deserialize, Serialize};

use super::{
    AssignmentDefinitionSourceView, CourseInstanceReceiptTarget, CurriculumAdoptionIdempotencyKey,
    CurriculumImportRevision, CurriculumPinPosition, CurriculumPinReplacements,
    CurriculumReplayStatus, ObservedBlueprintSource, ReplacementQuestionChoices,
};
use crate::{
    AccountId, AssignmentReference, AssignmentRevision, CourseInstanceReference, CourseScheduleRevision,
    CourseTerm, QuestionVersionReference, ResolvedRelativeAssignmentSchedule,
};

use super::bounded::{
    deserialize_course_instance_corrections, deserialize_course_instance_provenance,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ObservedCourseInstanceAssignment {
    pub assignment: AssignmentReference,
    pub revision: AssignmentRevision,
}

/// Exact destination scope and revision evidence observed by the server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseInstanceWitness {
    pub course: CourseInstanceReference,
    pub schedule_revision: CourseScheduleRevision,
    assignments: BoundedCourseInstanceAssignments,
}

/// Immutable Blueprint application that established a CourseInstance.
///
/// This is the CourseInstance's parent and initial applied revision.  It is
/// deliberately separate from assignment-level provenance: later controlled
/// updates and selected copies may have different source revisions without
/// changing the instance's origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseInstanceBlueprintApplication {
    pub source: ObservedBlueprintSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CourseInstanceWitnessError;

impl CourseInstanceWitness {
    /// Creates exact bounded destination evidence retained by commands and receipts.
    pub fn new(
        course: CourseInstanceReference,
        schedule_revision: CourseScheduleRevision,
        assignments: Vec<ObservedCourseInstanceAssignment>,
    ) -> Result<Self, CourseInstanceWitnessError> {
        Ok(Self {
            course,
            schedule_revision,
            assignments: BoundedCourseInstanceAssignments::new(assignments)
                .map_err(|_| CourseInstanceWitnessError)?,
        })
    }

    /// Returns the observed assignment revisions in their server-observed order.
    pub fn assignments(&self) -> &[ObservedCourseInstanceAssignment] {
        self.assignments.as_slice()
    }
}

impl std::fmt::Display for CourseInstanceWitnessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("course instance assignment evidence exceeds the course assignment bound")
    }
}
impl std::error::Error for CourseInstanceWitnessError {}

/// Server-only origin evidence for one reserved CourseInstance creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseInstanceCreationOrigin {
    Blueprint(ObservedBlueprintSource),
    Rollover(CourseInstanceWitness),
}

/// One server-reserved CourseInstance creation bound to an authenticated Instructor operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInstanceCreationWitness {
    origin: CourseInstanceCreationOrigin,
    target_term: CourseTerm,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
    reserved_course: CourseInstanceReference,
}

impl CourseInstanceCreationWitness {
    pub fn for_blueprint(
        source: ObservedBlueprintSource,
        target_term: CourseTerm,
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
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
        source: CourseInstanceWitness,
        target_term: CourseTerm,
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
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
    pub fn matches_blueprint_source(&self, source: &ObservedBlueprintSource) -> bool {
        matches!(&self.origin, CourseInstanceCreationOrigin::Blueprint(value) if value == source)
    }
    pub fn matches_rollover_source(&self, source: &CourseInstanceWitness) -> bool {
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
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
    pub fn reserved_course(&self) -> CourseInstanceReference {
        self.reserved_course
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseInstanceImportWitness {
    pub source: AssignmentDefinitionSourceView,
    pub destination: ObservedCourseInstanceAssignment,
    pub import_revision: CurriculumImportRevision,
}

/// Server-only immutable evidence for one applied assignment import.
///
/// No Serde implementation is provided: a browser may request an update, but
/// cannot manufacture the post-mutation evidence that proves its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedAssignmentImportEvidence {
    source: AssignmentDefinitionSourceView,
    replacements: CurriculumPinReplacements,
    semantic_digest: super::super::CurriculumSemanticDigest,
    assignment: ObservedCourseInstanceAssignment,
    import_revision: CurriculumImportRevision,
}

impl AppliedAssignmentImportEvidence {
    pub fn new(
        source: AssignmentDefinitionSourceView,
        replacements: CurriculumPinReplacements,
        semantic_digest: super::super::CurriculumSemanticDigest,
        assignment: ObservedCourseInstanceAssignment,
        import_revision: CurriculumImportRevision,
    ) -> Self {
        Self {
            source,
            replacements,
            semantic_digest,
            assignment,
            import_revision,
        }
    }
    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }
    pub fn semantic_digest(&self) -> super::super::CurriculumSemanticDigest {
        self.semantic_digest
    }
    pub fn assignment(&self) -> ObservedCourseInstanceAssignment {
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
    receipt_key: CurriculumAdoptionIdempotencyKey,
    course: CourseInstanceReference,
    assignment: AssignmentReference,
    import_revision: CurriculumImportRevision,
}

impl AssignmentImportReceiptTarget {
    pub fn new(
        receipt_account: AccountId,
        receipt_key: CurriculumAdoptionIdempotencyKey,
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
    pub fn receipt_key(&self) -> &CurriculumAdoptionIdempotencyKey {
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
pub struct UnavailableCurriculumPinRecovery {
    pub source: AssignmentDefinitionSourceView,
    pub position: CurriculumPinPosition,
    pub unavailable: QuestionVersionReference,
    pub choices: ReplacementQuestionChoices,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct BlueprintAssignmentProvenance {
    pub source: AssignmentDefinitionSourceView,
    pub import_revision: CurriculumImportRevision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CourseInstanceBlueprintInspectionView {
    pub initial_blueprint_application: CourseInstanceBlueprintApplication,
    pub witness: CourseInstanceWitness,
    #[serde(deserialize_with = "deserialize_course_instance_provenance")]
    pub assignments: Vec<BlueprintAssignmentProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CourseInstanceRefusal {
    IssuedWork {
        course: CourseInstanceReference,
    },
    Divergent {
        assignment: AssignmentReference,
    },
    SourceRevisionDrift {
        source: ObservedBlueprintSource,
    },
    ScheduleCorrectionsRequired {
        #[serde(deserialize_with = "deserialize_course_instance_corrections")]
        corrections: Vec<CourseInstanceScheduleCorrection>,
    },
    UnavailablePin {
        recovery: UnavailableCurriculumPinRecovery,
    },
    ReceiptUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CourseInstanceEligibility {
    Eligible,
    Refused { refusal: CourseInstanceRefusal },
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
    pub source: AssignmentDefinitionSourceView,
    pub assignment: AssignmentReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateSelectedBlueprintAssignmentPreviewRequest {
    pub course: CourseInstanceReference,
    pub source: AssignmentDefinitionSourceView,
    pub replacements: CurriculumPinReplacements,
}

/// Server-only repair intent for one retained CourseInstance receipt.
///
/// A repair has an independent audit and retry identity. Callers reuse the
/// key for a retry and issue a new key for a later repair action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionIntent {
    pub target: CourseInstanceReceiptTarget,
    pub idempotency_key: CurriculumAdoptionIdempotencyKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RolloverReusableStateManifest {
    pub source: ObservedBlueprintSource,
    assignments: BoundedAssignmentDefinitionSources,
    schedules: BoundedResolvedScheduleSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RolloverReusableStateManifestError;

impl RolloverReusableStateManifest {
    /// Validates the complete reusable-only state copied into a rollover.
    pub fn new(
        source: ObservedBlueprintSource,
        assignments: Vec<AssignmentDefinitionSourceView>,
        schedules: Vec<ResolvedRelativeAssignmentSchedule>,
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
    pub fn assignments(&self) -> &[AssignmentDefinitionSourceView] {
        self.assignments.as_slice()
    }

    /// Returns resolved reusable schedules in their original reusable-course order.
    pub fn schedules(&self) -> &[ResolvedRelativeAssignmentSchedule] {
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
pub struct RolloverCourseInstanceManifest {
    pub copied: RolloverReusableStateManifest,
    pub exclusion_policy: RolloverExclusionPolicy,
}

impl RolloverCourseInstanceManifest {
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
    pub witness: CourseInstanceWitness,
    pub target_term: CourseTerm,
    pub manifest: RolloverCourseInstanceManifest,
    pub eligibility: CourseInstanceEligibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ShiftCourseInstanceTermPreview {
    pub witness: CourseInstanceWitness,
    pub target_term: CourseTerm,
    pub schedules: BoundedResolvedScheduleSet,
    pub eligibility: CourseInstanceEligibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ControlledUpdateBlueprintAssignmentPreview {
    pub import: CourseInstanceImportWitness,
    pub witness: CourseInstanceWitness,
    pub eligibility: CourseInstanceEligibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateSelectedBlueprintAssignmentPreview {
    pub source: AssignmentDefinitionSourceView,
    pub witness: CourseInstanceWitness,
    pub schedule: ResolvedRelativeAssignmentSchedule,
    pub eligibility: CourseInstanceEligibility,
}

/// Server-only reconciliation projection bound to one immutable completed receipt.
///
/// This is not a browser DTO: Store code uses it to retain the receipt target while it
/// rechecks current derived projections and consumes the matching reconciliation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionPreview {
    receipt: super::CourseInstanceReceiptTarget,
    eligibility: CourseInstanceEligibility,
}

impl ReconcileCourseInstanceAdoptionPreview {
    /// Creates a server-held reconciliation projection for exactly one receipt target.
    pub fn new(
        receipt: super::CourseInstanceReceiptTarget,
        eligibility: CourseInstanceEligibility,
    ) -> Self {
        Self {
            receipt,
            eligibility,
        }
    }

    /// Returns the immutable completed operation selected for reconciliation.
    pub fn receipt(&self) -> &super::CourseInstanceReceiptTarget {
        &self.receipt
    }

    /// Returns the server-computed eligibility outcome.
    pub fn eligibility(&self) -> &CourseInstanceEligibility {
        &self.eligibility
    }
}

/// Browser-safe completion for one next-term CourseInstance rollover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RolloverCourseInstanceCompleted {
    pub course: CourseInstanceReference,
    pub replay: CurriculumReplayStatus,
}

/// Browser-safe completion for one atomic CourseInstance term shift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ShiftCourseInstanceTermCompleted {
    pub course: CourseInstanceReference,
    pub replay: CurriculumReplayStatus,
}

/// Browser-safe completion for one controlled Blueprint assignment update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ControlledUpdateBlueprintAssignmentCompleted {
    pub course: CourseInstanceReference,
    pub assignment: AssignmentReference,
    pub replay: CurriculumReplayStatus,
}

/// Browser-safe completion for one selected Blueprint assignment copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CreateSelectedBlueprintAssignmentCompleted {
    pub course: CourseInstanceReference,
    pub assignment: AssignmentReference,
    pub replay: CurriculumReplayStatus,
}

/// Browser-safe completion for rebuilding derived projections from one receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReconcileCourseInstanceAdoptionCompleted {
    pub course: CourseInstanceReference,
    pub replay: CurriculumReplayStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseInstanceCommandError {
    Refused(CourseInstanceRefusal),
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

pub(super) fn require_course_instance_eligible(
    value: &CourseInstanceEligibility,
) -> Result<(), CourseInstanceCommandError> {
    match value {
        CourseInstanceEligibility::Eligible => Ok(()),
        CourseInstanceEligibility::Refused { refusal } => {
            Err(CourseInstanceCommandError::Refused(refusal.clone()))
        }
    }
}

/// Checked, immutable schedule evidence retained by server apply records and receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedResolvedScheduleSet(Vec<ResolvedRelativeAssignmentSchedule>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedResolvedScheduleSetError;

impl BoundedResolvedScheduleSet {
    pub fn new(
        schedules: Vec<ResolvedRelativeAssignmentSchedule>,
    ) -> Result<Self, BoundedResolvedScheduleSetError> {
        (schedules.len() <= crate::MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .then_some(Self(schedules))
            .ok_or(BoundedResolvedScheduleSetError)
    }
    pub fn as_slice(&self) -> &[ResolvedRelativeAssignmentSchedule] {
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
pub struct BoundedCourseInstanceAssignments(Vec<ObservedCourseInstanceAssignment>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedCourseInstanceAssignmentsError;

impl BoundedCourseInstanceAssignments {
    pub fn new(
        assignments: Vec<ObservedCourseInstanceAssignment>,
    ) -> Result<Self, BoundedCourseInstanceAssignmentsError> {
        if assignments.len() > crate::MAX_ASSIGNMENT_ORDERED_ENTRIES
            || assignments.iter().enumerate().any(|(index, assignment)| {
                assignments[..index]
                    .iter()
                    .any(|prior| prior.assignment == assignment.assignment)
            })
        {
            return Err(BoundedCourseInstanceAssignmentsError);
        }
        Ok(Self(assignments))
    }

    pub fn as_slice(&self) -> &[ObservedCourseInstanceAssignment] {
        &self.0
    }
}

impl std::fmt::Display for BoundedCourseInstanceAssignmentsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("course instance assignment evidence exceeds the course assignment bound")
    }
}
impl std::error::Error for BoundedCourseInstanceAssignmentsError {}

impl Serialize for BoundedCourseInstanceAssignments {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BoundedCourseInstanceAssignments {
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
pub struct BoundedAssignmentDefinitionSources(Vec<AssignmentDefinitionSourceView>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedAssignmentDefinitionSourcesError;

impl BoundedAssignmentDefinitionSources {
    pub fn new(
        assignments: Vec<AssignmentDefinitionSourceView>,
    ) -> Result<Self, BoundedAssignmentDefinitionSourcesError> {
        (assignments.len() <= crate::MAX_ASSIGNMENT_ORDERED_ENTRIES)
            .then_some(Self(assignments))
            .ok_or(BoundedAssignmentDefinitionSourcesError)
    }

    pub fn as_slice(&self) -> &[AssignmentDefinitionSourceView] {
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
