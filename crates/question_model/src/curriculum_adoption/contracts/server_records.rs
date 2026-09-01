//! Server-held authority records consumed to construct curriculum apply commands.
//!
//! Browser previews are explanatory JSON. These records are created after authorization from
//! canonical Store reads and are intentionally non-Serde so a browser round-trip cannot become
//! an apply authority boundary.

use super::{
    AdoptBlueprintAssignmentCommandError, AdoptBlueprintAssignmentReadiness,
    ApplyBlueprintUpdateReadiness, AssignmentSourceSnapshot, BlueprintAssignmentRevisionReference,
    BlueprintForkReservation, BlueprintOperationReconciliationReadiness,
    BlueprintOperationRetryToken, BlueprintRevisionReference, BoundedResolvedScheduleSet,
    CopyAssignmentFromBlueprintReadiness, CopyCourseForNewTermReadiness,
    CourseInstanceCommandError, CourseInstanceCreationReservation, CourseInstanceOperationReceipt,
    CourseInstanceSnapshot, CourseOrigin, CourseRolloverManifest, ForkBlueprintCourseCommandError,
    ForkBlueprintCourseReadiness, InstantiateBlueprintCourseCommandError,
    InstantiateBlueprintCourseReadiness, QuestionRevisionSubstitutions, ShiftCourseDatesReadiness,
};
use crate::{AccountId, CourseTerm, ResolvedAssignmentSchedule};

/// Exact authenticated request identity observed by a server apply record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurriculumAdoptionRequestBinding {
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

impl CurriculumAdoptionRequestBinding {
    pub fn new(
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: BlueprintOperationRetryToken,
    ) -> Self {
        Self {
            authorized_account,
            request_digest,
            idempotency_key,
        }
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

    pub(super) fn into_parts(self) -> (AccountId, [u8; 32], BlueprintOperationRetryToken) {
        (
            self.authorized_account,
            self.request_digest,
            self.idempotency_key,
        )
    }
}

/// Exact server-resolved authority for an existing-CourseInstance assignment adoption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptBlueprintAssignmentApplyRecord {
    source: BlueprintAssignmentRevisionReference,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    replacements: QuestionRevisionSubstitutions,
    request: CurriculumAdoptionRequestBinding,
}

impl AdoptBlueprintAssignmentApplyRecord {
    /// Captures every server-resolved fact required to atomically apply one adoption.
    pub fn new(
        source: BlueprintAssignmentRevisionReference,
        destination: CourseInstanceSnapshot,
        course_origin: CourseOrigin,
        replacements: QuestionRevisionSubstitutions,
        request: CurriculumAdoptionRequestBinding,
        readiness: AdoptBlueprintAssignmentReadiness,
    ) -> Result<Self, AdoptBlueprintAssignmentCommandError> {
        readiness.require_ready()?;
        Ok(Self {
            source,
            destination,
            course_origin,
            replacements,
            request,
        })
    }

    pub fn source(&self) -> BlueprintAssignmentRevisionReference {
        self.source
    }
    pub fn destination(&self) -> &CourseInstanceSnapshot {
        &self.destination
    }
    pub fn course_origin(&self) -> CourseOrigin {
        self.course_origin
    }
    pub fn replacements(&self) -> &QuestionRevisionSubstitutions {
        &self.replacements
    }
    pub fn authorized_account(&self) -> AccountId {
        self.request.authorized_account()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request.request_digest()
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        self.request.idempotency_key()
    }
}

/// Exact server-resolved authority for one BlueprintCourse fork reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBlueprintCourseApplyRecord {
    source: BlueprintRevisionReference,
    replacements: QuestionRevisionSubstitutions,
    creation: BlueprintForkReservation,
}

impl ForkBlueprintCourseApplyRecord {
    /// Binds a validated fork intent to its non-Serde creation reservation.
    pub fn new(
        source: BlueprintRevisionReference,
        replacements: QuestionRevisionSubstitutions,
        creation: BlueprintForkReservation,
        readiness: ForkBlueprintCourseReadiness,
    ) -> Result<Self, ForkBlueprintCourseCommandError> {
        readiness.require_ready()?;
        if creation.source() != &source {
            return Err(ForkBlueprintCourseCommandError::CreationReservationMismatch);
        }
        Ok(Self {
            source,
            replacements,
            creation,
        })
    }

    pub fn source(&self) -> &BlueprintRevisionReference {
        &self.source
    }
    pub fn replacements(&self) -> &QuestionRevisionSubstitutions {
        &self.replacements
    }
    pub fn creation(&self) -> &BlueprintForkReservation {
        &self.creation
    }
}

/// Exact server-resolved authority for one CourseInstance instantiation reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstantiateBlueprintCourseApplyRecord {
    source: BlueprintRevisionReference,
    target_term: CourseTerm,
    replacements: QuestionRevisionSubstitutions,
    creation: CourseInstanceCreationReservation,
}

impl InstantiateBlueprintCourseApplyRecord {
    /// Binds validated source, term, substitutions, and a reserved CourseInstance identity.
    pub fn new(
        source: BlueprintRevisionReference,
        target_term: CourseTerm,
        replacements: QuestionRevisionSubstitutions,
        creation: CourseInstanceCreationReservation,
        readiness: InstantiateBlueprintCourseReadiness,
    ) -> Result<Self, InstantiateBlueprintCourseCommandError> {
        readiness.require_ready()?;
        if !creation.matches_blueprint_source(&source) || creation.target_term() != &target_term {
            return Err(InstantiateBlueprintCourseCommandError::CreationReservationMismatch);
        }
        Ok(Self {
            source,
            target_term,
            replacements,
            creation,
        })
    }

    pub fn source(&self) -> &BlueprintRevisionReference {
        &self.source
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn replacements(&self) -> &QuestionRevisionSubstitutions {
        &self.replacements
    }
    pub fn creation(&self) -> &CourseInstanceCreationReservation {
        &self.creation
    }
}

/// Exact server-resolved authority for a rollover that reserves its destination before mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolloverCourseInstanceApplyRecord {
    source_course_instance: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    target_term: CourseTerm,
    manifest: CourseRolloverManifest,
    creation: CourseInstanceCreationReservation,
}

impl RolloverCourseInstanceApplyRecord {
    /// Binds a validated rollover to the exact source, manifest, term, and creation reservation.
    pub fn new(
        source_course_instance: CourseInstanceSnapshot,
        course_origin: CourseOrigin,
        target_term: CourseTerm,
        manifest: CourseRolloverManifest,
        creation: CourseInstanceCreationReservation,
        readiness: CopyCourseForNewTermReadiness,
    ) -> Result<Self, super::CourseInstanceCommandError> {
        readiness.require_ready()?;
        if !creation.matches_rollover_source(&source_course_instance)
            || creation.target_term() != &target_term
            || course_origin.source_course != Some(source_course_instance.course)
        {
            return Err(super::CourseInstanceCommandError::CreationWitnessMismatch);
        }
        Ok(Self {
            source_course_instance,
            course_origin,
            target_term,
            manifest,
            creation,
        })
    }

    pub fn source_course_instance(&self) -> &CourseInstanceSnapshot {
        &self.source_course_instance
    }
    pub fn course_origin(&self) -> CourseOrigin {
        self.course_origin
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn manifest(&self) -> &CourseRolloverManifest {
        &self.manifest
    }
    pub fn creation(&self) -> &CourseInstanceCreationReservation {
        &self.creation
    }

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        CourseInstanceSnapshot,
        CourseOrigin,
        CourseTerm,
        CourseRolloverManifest,
        CourseInstanceCreationReservation,
    ) {
        (
            self.source_course_instance,
            self.course_origin,
            self.target_term,
            self.manifest,
            self.creation,
        )
    }
}

/// Exact server-resolved authority for a term shift on an existing CourseInstance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftCourseInstanceTermApplyRecord {
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
    request: CurriculumAdoptionRequestBinding,
}

impl ShiftCourseInstanceTermApplyRecord {
    /// Captures the exact current destination and resolved schedule state for one shift.
    pub fn new(
        destination: CourseInstanceSnapshot,
        course_origin: CourseOrigin,
        target_term: CourseTerm,
        schedules: Vec<ResolvedAssignmentSchedule>,
        request: CurriculumAdoptionRequestBinding,
        readiness: ShiftCourseDatesReadiness,
    ) -> Result<Self, CourseInstanceCommandError> {
        readiness.require_ready()?;
        let schedules = BoundedResolvedScheduleSet::new(schedules)
            .map_err(CourseInstanceCommandError::ScheduleEvidence)?;
        Ok(Self {
            destination,
            course_origin,
            target_term,
            schedules,
            request,
        })
    }

    pub fn destination(&self) -> &CourseInstanceSnapshot {
        &self.destination
    }
    pub fn course_origin(&self) -> CourseOrigin {
        self.course_origin
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn schedules(&self) -> &BoundedResolvedScheduleSet {
        &self.schedules
    }
    pub fn authorized_account(&self) -> AccountId {
        self.request.authorized_account()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request.request_digest()
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        self.request.idempotency_key()
    }

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        CourseInstanceSnapshot,
        CourseOrigin,
        CourseTerm,
        BoundedResolvedScheduleSet,
        AccountId,
        [u8; 32],
        BlueprintOperationRetryToken,
    ) {
        let (authorized_account, request_digest, idempotency_key) = self.request.into_parts();
        (
            self.destination,
            self.course_origin,
            self.target_term,
            self.schedules,
            authorized_account,
            request_digest,
            idempotency_key,
        )
    }
}

/// Exact server-resolved authority for a controlled assignment update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledUpdateBlueprintAssignmentApplyRecord {
    source: BlueprintAssignmentRevisionReference,
    import: AssignmentSourceSnapshot,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    request: CurriculumAdoptionRequestBinding,
}

impl ControlledUpdateBlueprintAssignmentApplyRecord {
    /// Captures exact source/import/destination evidence for one approved controlled update.
    pub fn new(
        source: BlueprintAssignmentRevisionReference,
        import: AssignmentSourceSnapshot,
        destination: CourseInstanceSnapshot,
        course_origin: CourseOrigin,
        request: CurriculumAdoptionRequestBinding,
        readiness: ApplyBlueprintUpdateReadiness,
    ) -> Result<Self, CourseInstanceCommandError> {
        readiness.require_ready()?;
        if !source.is_strictly_newer_revision_of(import.source) {
            return Err(CourseInstanceCommandError::ControlledUpdateLineageMismatch);
        }
        if !destination
            .assignment_revisions()
            .contains(&import.destination)
        {
            return Err(CourseInstanceCommandError::DestinationAssignmentMissing);
        }
        Ok(Self {
            source,
            import,
            destination,
            course_origin,
            request,
        })
    }

    pub fn source(&self) -> BlueprintAssignmentRevisionReference {
        self.source
    }
    pub fn import(&self) -> &AssignmentSourceSnapshot {
        &self.import
    }
    pub fn destination(&self) -> &CourseInstanceSnapshot {
        &self.destination
    }
    pub fn course_origin(&self) -> CourseOrigin {
        self.course_origin
    }
    pub fn authorized_account(&self) -> AccountId {
        self.request.authorized_account()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request.request_digest()
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        self.request.idempotency_key()
    }

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        BlueprintAssignmentRevisionReference,
        AssignmentSourceSnapshot,
        CourseInstanceSnapshot,
        CourseOrigin,
        AccountId,
        [u8; 32],
        BlueprintOperationRetryToken,
    ) {
        let (authorized_account, request_digest, idempotency_key) = self.request.into_parts();
        (
            self.source,
            self.import,
            self.destination,
            self.course_origin,
            authorized_account,
            request_digest,
            idempotency_key,
        )
    }
}

/// Exact server-resolved authority for one selected Blueprint assignment copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyAssignmentFromBlueprintApplyRecord {
    source: BlueprintAssignmentRevisionReference,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    schedule: ResolvedAssignmentSchedule,
    replacements: QuestionRevisionSubstitutions,
    request: CurriculumAdoptionRequestBinding,
}

impl CopyAssignmentFromBlueprintApplyRecord {
    /// Captures the source, exact destination, and resolved schedule chosen by the server.
    pub fn new(
        source: BlueprintAssignmentRevisionReference,
        destination: CourseInstanceSnapshot,
        course_origin: CourseOrigin,
        schedule: ResolvedAssignmentSchedule,
        replacements: QuestionRevisionSubstitutions,
        request: CurriculumAdoptionRequestBinding,
        readiness: CopyAssignmentFromBlueprintReadiness,
    ) -> Result<Self, CourseInstanceCommandError> {
        readiness.require_ready()?;
        Ok(Self {
            source,
            destination,
            course_origin,
            schedule,
            replacements,
            request,
        })
    }

    pub fn source(&self) -> BlueprintAssignmentRevisionReference {
        self.source
    }
    pub fn destination(&self) -> &CourseInstanceSnapshot {
        &self.destination
    }
    pub fn course_origin(&self) -> CourseOrigin {
        self.course_origin
    }
    pub fn schedule(&self) -> &ResolvedAssignmentSchedule {
        &self.schedule
    }
    pub fn replacements(&self) -> &QuestionRevisionSubstitutions {
        &self.replacements
    }
    pub fn authorized_account(&self) -> AccountId {
        self.request.authorized_account()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request.request_digest()
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        self.request.idempotency_key()
    }

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        BlueprintAssignmentRevisionReference,
        CourseInstanceSnapshot,
        CourseOrigin,
        ResolvedAssignmentSchedule,
        QuestionRevisionSubstitutions,
        AccountId,
        [u8; 32],
        BlueprintOperationRetryToken,
    ) {
        let (authorized_account, request_digest, idempotency_key) = self.request.into_parts();
        (
            self.source,
            self.destination,
            self.course_origin,
            self.schedule,
            self.replacements,
            authorized_account,
            request_digest,
            idempotency_key,
        )
    }
}

/// Server-only reconciliation authority bound to one retained original receipt.
///
/// `receipt` identifies the immutable source evidence. `authorized_account`,
/// request digest, and idempotency key identify this new repair action, so a
/// repair has its own audit identity and never collides with the original
/// completed operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionApplyRecord {
    receipt: CourseInstanceOperationReceipt,
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

impl ReconcileCourseInstanceAdoptionApplyRecord {
    /// Binds reconciliation to immutable receipt evidence rather than a browser preview.
    pub fn new(
        receipt: CourseInstanceOperationReceipt,
        course_origin: CourseOrigin,
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: BlueprintOperationRetryToken,
        readiness: BlueprintOperationReconciliationReadiness,
    ) -> Result<Self, CourseInstanceCommandError> {
        readiness.require_ready()?;
        let Some(original_import_target) = receipt.assignment_import_target() else {
            return Err(CourseInstanceCommandError::ReceiptBindingMismatch);
        };
        if course_origin != receipt.course_origin()
            || original_import_target.course() != receipt.destination().course
            || (authorized_account == receipt.authorized_account()
                && idempotency_key == *receipt.idempotency_key())
        {
            return Err(CourseInstanceCommandError::ReceiptBindingMismatch);
        }
        Ok(Self {
            receipt,
            course_origin,
            authorized_account,
            request_digest,
            idempotency_key,
        })
    }

    pub fn receipt(&self) -> &CourseInstanceOperationReceipt {
        &self.receipt
    }
    pub fn course_origin(&self) -> CourseOrigin {
        self.course_origin
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

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        CourseInstanceOperationReceipt,
        CourseOrigin,
        AccountId,
        [u8; 32],
        BlueprintOperationRetryToken,
    ) {
        (
            self.receipt,
            self.course_origin,
            self.authorized_account,
            self.request_digest,
            self.idempotency_key,
        )
    }
}
