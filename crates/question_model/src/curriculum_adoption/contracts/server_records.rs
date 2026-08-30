//! Server-held authority records consumed to construct curriculum apply commands.
//!
//! Browser previews are explanatory JSON. These records are created after authorization from
//! canonical Store reads and are intentionally non-Serde so a browser round-trip cannot become
//! an apply authority boundary.

use super::{
    AssignmentDefinitionSourceView, BlueprintAdoptionEligibility, BlueprintCourseCreationWitness,
    BoundedResolvedScheduleSet, CourseInstanceBlueprintApplication, CourseInstanceCommandError,
    CourseInstanceCreationWitness, CourseInstanceEligibility, CourseInstanceImportWitness,
    CourseInstanceReceiptTarget, CourseInstanceWitness, CurriculumAdoptionCommandError,
    CurriculumAdoptionIdempotencyKey, CurriculumPinReplacements, ObservedBlueprintSource,
    RolloverCourseInstanceManifest,
};
use crate::{AccountId, CourseTerm, ResolvedRelativeAssignmentSchedule};

/// Exact immutable parentage and mutable destination witness for one existing
/// CourseInstance operation. This server-only binding keeps origin evidence
/// coupled to the course it constrains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInstanceApplicationBinding {
    destination: CourseInstanceWitness,
    blueprint_application: CourseInstanceBlueprintApplication,
}

impl CourseInstanceApplicationBinding {
    pub fn new(
        destination: CourseInstanceWitness,
        blueprint_application: CourseInstanceBlueprintApplication,
    ) -> Self {
        Self {
            destination,
            blueprint_application,
        }
    }

    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }

    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.blueprint_application
    }

    pub(super) fn into_parts(self) -> (CourseInstanceWitness, CourseInstanceBlueprintApplication) {
        (self.destination, self.blueprint_application)
    }
}

/// Exact authenticated request identity observed by a server apply record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurriculumAdoptionRequestBinding {
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl CurriculumAdoptionRequestBinding {
    pub fn new(
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
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

    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }

    pub(super) fn into_parts(self) -> (AccountId, [u8; 32], CurriculumAdoptionIdempotencyKey) {
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
    source: AssignmentDefinitionSourceView,
    destination: CourseInstanceApplicationBinding,
    replacements: CurriculumPinReplacements,
    request: CurriculumAdoptionRequestBinding,
}

impl AdoptBlueprintAssignmentApplyRecord {
    /// Captures every server-resolved fact required to atomically apply one adoption.
    pub fn new(
        source: AssignmentDefinitionSourceView,
        destination: CourseInstanceApplicationBinding,
        replacements: CurriculumPinReplacements,
        request: CurriculumAdoptionRequestBinding,
        eligibility: BlueprintAdoptionEligibility,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        super::require_blueprint_eligible(&eligibility)?;
        Ok(Self {
            source,
            destination,
            replacements,
            request,
        })
    }

    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        self.destination.destination()
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.destination.blueprint_application()
    }
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }
    pub fn authorized_account(&self) -> AccountId {
        self.request.authorized_account()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request.request_digest()
    }
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        self.request.idempotency_key()
    }
}

/// Exact server-resolved authority for one BlueprintCourse fork reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForkBlueprintCourseApplyRecord {
    source: ObservedBlueprintSource,
    replacements: CurriculumPinReplacements,
    creation: BlueprintCourseCreationWitness,
}

impl ForkBlueprintCourseApplyRecord {
    /// Binds a validated fork intent to its non-Serde creation reservation.
    pub fn new(
        source: ObservedBlueprintSource,
        replacements: CurriculumPinReplacements,
        creation: BlueprintCourseCreationWitness,
        eligibility: BlueprintAdoptionEligibility,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        super::require_blueprint_eligible(&eligibility)?;
        if creation.source() != &source {
            return Err(CurriculumAdoptionCommandError::CreationWitnessMismatch);
        }
        Ok(Self {
            source,
            replacements,
            creation,
        })
    }

    pub fn source(&self) -> &ObservedBlueprintSource {
        &self.source
    }
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }
    pub fn creation(&self) -> &BlueprintCourseCreationWitness {
        &self.creation
    }
}

/// Exact server-resolved authority for one CourseInstance instantiation reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstantiateBlueprintCourseApplyRecord {
    source: ObservedBlueprintSource,
    target_term: CourseTerm,
    replacements: CurriculumPinReplacements,
    creation: CourseInstanceCreationWitness,
}

impl InstantiateBlueprintCourseApplyRecord {
    /// Binds validated source, term, substitutions, and a reserved CourseInstance identity.
    pub fn new(
        source: ObservedBlueprintSource,
        target_term: CourseTerm,
        replacements: CurriculumPinReplacements,
        creation: CourseInstanceCreationWitness,
        eligibility: BlueprintAdoptionEligibility,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        super::require_blueprint_eligible(&eligibility)?;
        if !creation.matches_blueprint_source(&source) || creation.target_term() != &target_term {
            return Err(CurriculumAdoptionCommandError::CreationWitnessMismatch);
        }
        Ok(Self {
            source,
            target_term,
            replacements,
            creation,
        })
    }

    pub fn source(&self) -> &ObservedBlueprintSource {
        &self.source
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }
    pub fn creation(&self) -> &CourseInstanceCreationWitness {
        &self.creation
    }
}

/// Exact server-resolved authority for a rollover that reserves its destination before mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolloverCourseInstanceApplyRecord {
    source_course_instance: CourseInstanceWitness,
    blueprint_application: CourseInstanceBlueprintApplication,
    target_term: CourseTerm,
    manifest: RolloverCourseInstanceManifest,
    creation: CourseInstanceCreationWitness,
}

impl RolloverCourseInstanceApplyRecord {
    /// Binds a validated rollover to the exact source, manifest, term, and creation reservation.
    pub fn new(
        source_course_instance: CourseInstanceWitness,
        blueprint_application: CourseInstanceBlueprintApplication,
        target_term: CourseTerm,
        manifest: RolloverCourseInstanceManifest,
        creation: CourseInstanceCreationWitness,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, super::CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        if !creation.matches_rollover_source(&source_course_instance)
            || creation.target_term() != &target_term
        {
            return Err(super::CourseInstanceCommandError::CreationWitnessMismatch);
        }
        Ok(Self {
            source_course_instance,
            blueprint_application,
            target_term,
            manifest,
            creation,
        })
    }

    pub fn source_course_instance(&self) -> &CourseInstanceWitness {
        &self.source_course_instance
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.blueprint_application
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn manifest(&self) -> &RolloverCourseInstanceManifest {
        &self.manifest
    }
    pub fn creation(&self) -> &CourseInstanceCreationWitness {
        &self.creation
    }

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        CourseInstanceWitness,
        CourseInstanceBlueprintApplication,
        CourseTerm,
        RolloverCourseInstanceManifest,
        CourseInstanceCreationWitness,
    ) {
        (
            self.source_course_instance,
            self.blueprint_application,
            self.target_term,
            self.manifest,
            self.creation,
        )
    }
}

/// Exact server-resolved authority for a term shift on an existing CourseInstance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftCourseInstanceTermApplyRecord {
    destination: CourseInstanceApplicationBinding,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
    request: CurriculumAdoptionRequestBinding,
}

impl ShiftCourseInstanceTermApplyRecord {
    /// Captures the exact current destination and resolved schedule state for one shift.
    pub fn new(
        destination: CourseInstanceApplicationBinding,
        target_term: CourseTerm,
        schedules: Vec<ResolvedRelativeAssignmentSchedule>,
        request: CurriculumAdoptionRequestBinding,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        let schedules = BoundedResolvedScheduleSet::new(schedules)
            .map_err(CourseInstanceCommandError::ScheduleEvidence)?;
        Ok(Self {
            destination,
            target_term,
            schedules,
            request,
        })
    }

    pub fn destination(&self) -> &CourseInstanceWitness {
        self.destination.destination()
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.destination.blueprint_application()
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
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        self.request.idempotency_key()
    }

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        CourseInstanceWitness,
        CourseInstanceBlueprintApplication,
        CourseTerm,
        BoundedResolvedScheduleSet,
        AccountId,
        [u8; 32],
        CurriculumAdoptionIdempotencyKey,
    ) {
        let (destination, blueprint_application) = self.destination.into_parts();
        let (authorized_account, request_digest, idempotency_key) = self.request.into_parts();
        (
            destination,
            blueprint_application,
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
    source: AssignmentDefinitionSourceView,
    import: CourseInstanceImportWitness,
    destination: CourseInstanceApplicationBinding,
    request: CurriculumAdoptionRequestBinding,
}

impl ControlledUpdateBlueprintAssignmentApplyRecord {
    /// Captures exact source/import/destination evidence for one approved controlled update.
    pub fn new(
        source: AssignmentDefinitionSourceView,
        import: CourseInstanceImportWitness,
        destination: CourseInstanceApplicationBinding,
        request: CurriculumAdoptionRequestBinding,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        if !source.is_strictly_newer_revision_of(import.source) {
            return Err(CourseInstanceCommandError::ControlledUpdateLineageMismatch);
        }
        if !destination
            .destination()
            .assignments()
            .contains(&import.destination)
        {
            return Err(CourseInstanceCommandError::DestinationAssignmentMissing);
        }
        Ok(Self {
            source,
            import,
            destination,
            request,
        })
    }

    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn import(&self) -> &CourseInstanceImportWitness {
        &self.import
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        self.destination.destination()
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.destination.blueprint_application()
    }
    pub fn authorized_account(&self) -> AccountId {
        self.request.authorized_account()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request.request_digest()
    }
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        self.request.idempotency_key()
    }

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        AssignmentDefinitionSourceView,
        CourseInstanceImportWitness,
        CourseInstanceWitness,
        CourseInstanceBlueprintApplication,
        AccountId,
        [u8; 32],
        CurriculumAdoptionIdempotencyKey,
    ) {
        let (destination, blueprint_application) = self.destination.into_parts();
        let (authorized_account, request_digest, idempotency_key) = self.request.into_parts();
        (
            self.source,
            self.import,
            destination,
            blueprint_application,
            authorized_account,
            request_digest,
            idempotency_key,
        )
    }
}

/// Exact server-resolved authority for one selected Blueprint assignment copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSelectedBlueprintAssignmentApplyRecord {
    source: AssignmentDefinitionSourceView,
    destination: CourseInstanceApplicationBinding,
    schedule: ResolvedRelativeAssignmentSchedule,
    replacements: CurriculumPinReplacements,
    request: CurriculumAdoptionRequestBinding,
}

impl CreateSelectedBlueprintAssignmentApplyRecord {
    /// Captures the source, exact destination, and resolved schedule chosen by the server.
    pub fn new(
        source: AssignmentDefinitionSourceView,
        destination: CourseInstanceApplicationBinding,
        schedule: ResolvedRelativeAssignmentSchedule,
        replacements: CurriculumPinReplacements,
        request: CurriculumAdoptionRequestBinding,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        Ok(Self {
            source,
            destination,
            schedule,
            replacements,
            request,
        })
    }

    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        self.destination.destination()
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.destination.blueprint_application()
    }
    pub fn schedule(&self) -> &ResolvedRelativeAssignmentSchedule {
        &self.schedule
    }
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }
    pub fn authorized_account(&self) -> AccountId {
        self.request.authorized_account()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request.request_digest()
    }
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        self.request.idempotency_key()
    }

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        AssignmentDefinitionSourceView,
        CourseInstanceWitness,
        CourseInstanceBlueprintApplication,
        ResolvedRelativeAssignmentSchedule,
        CurriculumPinReplacements,
        AccountId,
        [u8; 32],
        CurriculumAdoptionIdempotencyKey,
    ) {
        let (destination, blueprint_application) = self.destination.into_parts();
        let (authorized_account, request_digest, idempotency_key) = self.request.into_parts();
        (
            self.source,
            destination,
            blueprint_application,
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
    receipt: CourseInstanceReceiptTarget,
    blueprint_application: CourseInstanceBlueprintApplication,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl ReconcileCourseInstanceAdoptionApplyRecord {
    /// Binds reconciliation to immutable receipt evidence rather than a browser preview.
    pub fn new(
        receipt: CourseInstanceReceiptTarget,
        blueprint_application: CourseInstanceBlueprintApplication,
        authorized_account: AccountId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        let Some(original_import_target) = receipt.assignment_import_target() else {
            return Err(CourseInstanceCommandError::ReceiptBindingMismatch);
        };
        if blueprint_application != receipt.blueprint_application()
            || original_import_target.course() != receipt.destination().course
            || (authorized_account == receipt.authorized_account()
                && idempotency_key == *receipt.idempotency_key())
        {
            return Err(CourseInstanceCommandError::ReceiptBindingMismatch);
        }
        Ok(Self {
            receipt,
            blueprint_application,
            authorized_account,
            request_digest,
            idempotency_key,
        })
    }

    pub fn receipt(&self) -> &CourseInstanceReceiptTarget {
        &self.receipt
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.blueprint_application
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

    pub(super) fn into_receipt_parts(
        self,
    ) -> (
        CourseInstanceReceiptTarget,
        CourseInstanceBlueprintApplication,
        AccountId,
        [u8; 32],
        CurriculumAdoptionIdempotencyKey,
    ) {
        (
            self.receipt,
            self.blueprint_application,
            self.authorized_account,
            self.request_digest,
            self.idempotency_key,
        )
    }
}
