//! Server-held authority records consumed to construct curriculum apply commands.
//!
//! Browser previews are explanatory JSON. These records are created after authorization from
//! canonical Store reads and are intentionally non-Serde so a browser round-trip cannot become
//! an apply authority boundary.

use super::{
    AssignmentDefinitionSourceView, BlueprintAdoptionEligibility, BlueprintCourseCreationWitness,
    BoundedResolvedScheduleSet, CourseInstanceCommandError, CourseInstanceCreationWitness,
    CourseInstanceEligibility, CourseInstanceImportWitness, CourseInstanceReceiptTarget,
    CourseInstanceWitness, CurriculumAdoptionCommandError, CurriculumAdoptionIdempotencyKey,
    CurriculumPinReplacements, ObservedBlueprintSource, RolloverCourseInstanceManifest,
};
use crate::{CourseTerm, ResolvedRelativeAssignmentSchedule, UserId};

/// Exact server-resolved authority for an existing-CourseInstance assignment adoption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptBlueprintAssignmentApplyRecord {
    source: AssignmentDefinitionSourceView,
    destination: CourseInstanceWitness,
    replacements: CurriculumPinReplacements,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl AdoptBlueprintAssignmentApplyRecord {
    /// Captures every server-resolved fact required to atomically apply one adoption.
    pub fn new(
        source: AssignmentDefinitionSourceView,
        destination: CourseInstanceWitness,
        replacements: CurriculumPinReplacements,
        authorized_actor: UserId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
        eligibility: BlueprintAdoptionEligibility,
    ) -> Result<Self, CurriculumAdoptionCommandError> {
        super::require_blueprint_eligible(&eligibility)?;
        Ok(Self {
            source,
            destination,
            replacements,
            authorized_actor,
            request_digest,
            idempotency_key,
        })
    }

    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }
    pub fn replacements(&self) -> &CurriculumPinReplacements {
        &self.replacements
    }
    pub fn authorized_actor(&self) -> UserId {
        self.authorized_actor
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
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
    target_term: CourseTerm,
    manifest: RolloverCourseInstanceManifest,
    creation: CourseInstanceCreationWitness,
}

impl RolloverCourseInstanceApplyRecord {
    /// Binds a validated rollover to the exact source, manifest, term, and creation reservation.
    pub fn new(
        source_course_instance: CourseInstanceWitness,
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
            target_term,
            manifest,
            creation,
        })
    }

    pub fn source_course_instance(&self) -> &CourseInstanceWitness {
        &self.source_course_instance
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
        CourseTerm,
        RolloverCourseInstanceManifest,
        CourseInstanceCreationWitness,
    ) {
        (
            self.source_course_instance,
            self.target_term,
            self.manifest,
            self.creation,
        )
    }
}

/// Exact server-resolved authority for a term shift on an existing CourseInstance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftCourseInstanceTermApplyRecord {
    destination: CourseInstanceWitness,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl ShiftCourseInstanceTermApplyRecord {
    /// Captures the exact current destination and resolved schedule state for one shift.
    pub fn new(
        destination: CourseInstanceWitness,
        target_term: CourseTerm,
        schedules: Vec<ResolvedRelativeAssignmentSchedule>,
        authorized_actor: UserId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        let schedules = BoundedResolvedScheduleSet::new(schedules)
            .map_err(CourseInstanceCommandError::ScheduleEvidence)?;
        Ok(Self {
            destination,
            target_term,
            schedules,
            authorized_actor,
            request_digest,
            idempotency_key,
        })
    }

    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn schedules(&self) -> &BoundedResolvedScheduleSet {
        &self.schedules
    }
    pub fn authorized_actor(&self) -> UserId {
        self.authorized_actor
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
        CourseInstanceWitness,
        CourseTerm,
        BoundedResolvedScheduleSet,
        UserId,
        [u8; 32],
        CurriculumAdoptionIdempotencyKey,
    ) {
        (
            self.destination,
            self.target_term,
            self.schedules,
            self.authorized_actor,
            self.request_digest,
            self.idempotency_key,
        )
    }
}

/// Exact server-resolved authority for a controlled assignment update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledUpdateBlueprintAssignmentApplyRecord {
    source: AssignmentDefinitionSourceView,
    import: CourseInstanceImportWitness,
    destination: CourseInstanceWitness,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl ControlledUpdateBlueprintAssignmentApplyRecord {
    /// Captures exact source/import/destination evidence for one approved controlled update.
    pub fn new(
        source: AssignmentDefinitionSourceView,
        import: CourseInstanceImportWitness,
        destination: CourseInstanceWitness,
        authorized_actor: UserId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        if source != import.source {
            return Err(CourseInstanceCommandError::ImportSourceMismatch);
        }
        if !destination.assignments().contains(&import.destination) {
            return Err(CourseInstanceCommandError::DestinationAssignmentMissing);
        }
        Ok(Self {
            source,
            import,
            destination,
            authorized_actor,
            request_digest,
            idempotency_key,
        })
    }

    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn import(&self) -> &CourseInstanceImportWitness {
        &self.import
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }
    pub fn authorized_actor(&self) -> UserId {
        self.authorized_actor
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
        AssignmentDefinitionSourceView,
        CourseInstanceImportWitness,
        CourseInstanceWitness,
        UserId,
        [u8; 32],
        CurriculumAdoptionIdempotencyKey,
    ) {
        (
            self.source,
            self.import,
            self.destination,
            self.authorized_actor,
            self.request_digest,
            self.idempotency_key,
        )
    }
}

/// Exact server-resolved authority for one selected Blueprint assignment copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSelectedBlueprintAssignmentApplyRecord {
    source: AssignmentDefinitionSourceView,
    destination: CourseInstanceWitness,
    schedule: ResolvedRelativeAssignmentSchedule,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl CreateSelectedBlueprintAssignmentApplyRecord {
    /// Captures the source, exact destination, and resolved schedule chosen by the server.
    pub fn new(
        source: AssignmentDefinitionSourceView,
        destination: CourseInstanceWitness,
        schedule: ResolvedRelativeAssignmentSchedule,
        authorized_actor: UserId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        Ok(Self {
            source,
            destination,
            schedule,
            authorized_actor,
            request_digest,
            idempotency_key,
        })
    }

    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }
    pub fn schedule(&self) -> &ResolvedRelativeAssignmentSchedule {
        &self.schedule
    }
    pub fn authorized_actor(&self) -> UserId {
        self.authorized_actor
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
        AssignmentDefinitionSourceView,
        CourseInstanceWitness,
        ResolvedRelativeAssignmentSchedule,
        UserId,
        [u8; 32],
        CurriculumAdoptionIdempotencyKey,
    ) {
        (
            self.source,
            self.destination,
            self.schedule,
            self.authorized_actor,
            self.request_digest,
            self.idempotency_key,
        )
    }
}

/// Server-only receipt-targeted reconciliation authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionApplyRecord {
    receipt: CourseInstanceReceiptTarget,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl ReconcileCourseInstanceAdoptionApplyRecord {
    /// Binds reconciliation to immutable receipt evidence rather than a browser preview.
    pub fn new(
        receipt: CourseInstanceReceiptTarget,
        authorized_actor: UserId,
        request_digest: [u8; 32],
        idempotency_key: CurriculumAdoptionIdempotencyKey,
        eligibility: CourseInstanceEligibility,
    ) -> Result<Self, CourseInstanceCommandError> {
        super::require_course_instance_eligible(&eligibility)?;
        if receipt.idempotency_key() != &idempotency_key
            || receipt.request_digest() != request_digest
        {
            return Err(CourseInstanceCommandError::ReceiptBindingMismatch);
        }
        Ok(Self {
            receipt,
            authorized_actor,
            request_digest,
            idempotency_key,
        })
    }

    pub fn receipt(&self) -> &CourseInstanceReceiptTarget {
        &self.receipt
    }
    pub fn authorized_actor(&self) -> UserId {
        self.authorized_actor
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
        UserId,
        [u8; 32],
        CurriculumAdoptionIdempotencyKey,
    ) {
        (
            self.receipt,
            self.authorized_actor,
            self.request_digest,
            self.idempotency_key,
        )
    }
}
