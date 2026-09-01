//! Server-held authority records consumed to construct curriculum apply commands.
//!
//! Browser previews are explanatory JSON. These records are created after authorization from
//! canonical Store reads and are intentionally non-Serde so a browser round-trip cannot become
//! an apply authority boundary.

use super::{
    ApplyBlueprintUpdateReadiness, AssignmentImportReceipt, AssignmentSourceSnapshot,
    BlueprintAssignmentRevisionReference, BlueprintForkReservation, BlueprintOperationRetryToken,
    BlueprintRevisionReference, BoundedResolvedScheduleSet, CopyAssignmentFromBlueprintReadiness,
    CopyCourseForNewTermReadiness, CourseInstanceCommandError, CourseInstanceCreationReservation,
    CourseInstanceSnapshot, CourseOrigin, CourseRolloverManifest,
    CreateCourseFromBlueprintCommandError, CreateCourseFromBlueprintReadiness,
    ForkBlueprintCourseCommandError, ForkBlueprintCourseReadiness, QuestionRevisionSubstitutions,
    ReconcileCourseInstanceReadiness, ShiftCourseDatesReadiness,
};
use crate::{AccountId, CourseTerm, ResolvedAssignmentSchedule};

/// Exact authenticated request identity observed by a server apply record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintOperationRequestBinding {
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

impl BlueprintOperationRequestBinding {
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

/// Exact server-resolved authority for one Create Course from Blueprint reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCourseFromBlueprintApplyRecord {
    source: BlueprintRevisionReference,
    target_term: CourseTerm,
    replacements: QuestionRevisionSubstitutions,
    creation: CourseInstanceCreationReservation,
}

impl CreateCourseFromBlueprintApplyRecord {
    /// Binds validated source, term, substitutions, and a reserved CourseInstance identity.
    pub fn new(
        source: BlueprintRevisionReference,
        target_term: CourseTerm,
        replacements: QuestionRevisionSubstitutions,
        creation: CourseInstanceCreationReservation,
        readiness: CreateCourseFromBlueprintReadiness,
    ) -> Result<Self, CreateCourseFromBlueprintCommandError> {
        readiness.require_ready()?;
        if !creation.matches_blueprint_source(&source) || creation.target_term() != &target_term {
            return Err(CreateCourseFromBlueprintCommandError::CreationReservationMismatch);
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

/// Exact server-resolved authority for Copy Course for New Term, with a reserved destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyCourseForNewTermApplyRecord {
    source_course_instance: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    target_term: CourseTerm,
    manifest: CourseRolloverManifest,
    creation: CourseInstanceCreationReservation,
}

impl CopyCourseForNewTermApplyRecord {
    /// Binds Copy Course for New Term to its exact source, manifest, term, and creation reservation.
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
pub struct ShiftCourseDatesApplyRecord {
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
    request: BlueprintOperationRequestBinding,
}

impl ShiftCourseDatesApplyRecord {
    /// Captures the exact current destination and resolved schedule state for Shift Course Dates.
    pub fn new(
        destination: CourseInstanceSnapshot,
        course_origin: CourseOrigin,
        target_term: CourseTerm,
        schedules: Vec<ResolvedAssignmentSchedule>,
        request: BlueprintOperationRequestBinding,
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

/// Exact server-resolved authority for applying a Blueprint update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyBlueprintUpdateApplyRecord {
    source: BlueprintAssignmentRevisionReference,
    import: AssignmentSourceSnapshot,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    request: BlueprintOperationRequestBinding,
}

impl ApplyBlueprintUpdateApplyRecord {
    /// Captures exact source/import/destination evidence for one approved Blueprint update.
    pub fn new(
        source: BlueprintAssignmentRevisionReference,
        import: AssignmentSourceSnapshot,
        destination: CourseInstanceSnapshot,
        course_origin: CourseOrigin,
        request: BlueprintOperationRequestBinding,
        readiness: ApplyBlueprintUpdateReadiness,
    ) -> Result<Self, CourseInstanceCommandError> {
        readiness.require_ready()?;
        if !source.is_strictly_newer_revision_of(import.source) {
            return Err(CourseInstanceCommandError::BlueprintUpdateLineageMismatch);
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

/// Exact server-resolved authority for copying one Blueprint Assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyAssignmentFromBlueprintApplyRecord {
    source: BlueprintAssignmentRevisionReference,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    schedule: ResolvedAssignmentSchedule,
    replacements: QuestionRevisionSubstitutions,
    request: BlueprintOperationRequestBinding,
}

impl CopyAssignmentFromBlueprintApplyRecord {
    /// Captures the source, exact destination, and resolved schedule chosen by the server.
    pub fn new(
        source: BlueprintAssignmentRevisionReference,
        destination: CourseInstanceSnapshot,
        course_origin: CourseOrigin,
        schedule: ResolvedAssignmentSchedule,
        replacements: QuestionRevisionSubstitutions,
        request: BlueprintOperationRequestBinding,
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

/// Server-only reconciliation authority bound to one retained Assignment import receipt.
///
/// `original_import_receipt` identifies the immutable source evidence. `authorized_account`,
/// request digest, and idempotency key identify this new repair action, so a
/// repair has its own audit identity and never collides with the original
/// completed operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceApplyRecord {
    original_import_receipt: AssignmentImportReceipt,
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

impl ReconcileCourseInstanceApplyRecord {
    /// Binds reconciliation to immutable receipt evidence rather than a browser preview.
    pub fn new(
        original_import_receipt: AssignmentImportReceipt,
        course_origin: CourseOrigin,
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: BlueprintOperationRetryToken,
        readiness: ReconcileCourseInstanceReadiness,
    ) -> Result<Self, CourseInstanceCommandError> {
        readiness.require_ready()?;
        let original_import_target = original_import_receipt.target();
        if course_origin != original_import_receipt.course_origin()
            || original_import_target.course() != original_import_receipt.destination().course
            || (authorized_account == original_import_receipt.authorized_account()
                && idempotency_key == *original_import_receipt.idempotency_key())
        {
            return Err(CourseInstanceCommandError::ReceiptBindingMismatch);
        }
        Ok(Self {
            original_import_receipt,
            course_origin,
            authorized_account,
            request_digest,
            idempotency_key,
        })
    }

    pub fn original_import_receipt(&self) -> &AssignmentImportReceipt {
        &self.original_import_receipt
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
        AssignmentImportReceipt,
        CourseOrigin,
        AccountId,
        [u8; 32],
        BlueprintOperationRetryToken,
    ) {
        (
            self.original_import_receipt,
            self.course_origin,
            self.authorized_account,
            self.request_digest,
            self.idempotency_key,
        )
    }
}
