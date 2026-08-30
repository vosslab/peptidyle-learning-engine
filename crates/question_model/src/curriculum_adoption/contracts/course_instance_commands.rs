//! Non-Serde CourseInstance apply commands built only from server-held records.

use crate::{CourseTerm, ResolvedRelativeAssignmentSchedule, UserId};

use super::{
    AssignmentDefinitionSourceView, BoundedResolvedScheduleSet, CourseInstanceBlueprintApplication,
    CourseInstanceCreationWitness, CourseInstanceImportWitness, CourseInstanceReceiptTarget,
    CourseInstanceWitness, CurriculumAdoptionIdempotencyKey, CurriculumPinReplacements,
    RolloverCourseInstanceManifest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolloverCourseInstanceCommand {
    source_course_instance: CourseInstanceWitness,
    blueprint_application: CourseInstanceBlueprintApplication,
    target_term: CourseTerm,
    manifest: RolloverCourseInstanceManifest,
    creation: CourseInstanceCreationWitness,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl RolloverCourseInstanceCommand {
    pub fn from_server_record(record: super::RolloverCourseInstanceApplyRecord) -> Self {
        let idempotency_key = record.creation().idempotency_key().clone();
        Self {
            source_course_instance: record.source_course_instance().clone(),
            blueprint_application: record.blueprint_application(),
            target_term: record.target_term().clone(),
            manifest: record.manifest().clone(),
            creation: record.creation().clone(),
            idempotency_key,
        }
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
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
        &self.idempotency_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftCourseInstanceTermCommand {
    destination: CourseInstanceWitness,
    blueprint_application: CourseInstanceBlueprintApplication,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl ShiftCourseInstanceTermCommand {
    pub fn from_server_record(record: super::ShiftCourseInstanceTermApplyRecord) -> Self {
        Self {
            destination: record.destination().clone(),
            blueprint_application: record.blueprint_application(),
            target_term: record.target_term().clone(),
            schedules: record.schedules().clone(),
            authorized_actor: record.authorized_actor(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
    }

    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.blueprint_application
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn schedules(&self) -> &[ResolvedRelativeAssignmentSchedule] {
        self.schedules.as_slice()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledUpdateBlueprintAssignmentCommand {
    source: AssignmentDefinitionSourceView,
    import: CourseInstanceImportWitness,
    destination: CourseInstanceWitness,
    blueprint_application: CourseInstanceBlueprintApplication,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl ControlledUpdateBlueprintAssignmentCommand {
    pub fn from_server_record(
        record: super::ControlledUpdateBlueprintAssignmentApplyRecord,
    ) -> Self {
        Self {
            source: record.source(),
            import: record.import().clone(),
            destination: record.destination().clone(),
            blueprint_application: record.blueprint_application(),
            authorized_actor: record.authorized_actor(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
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
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.blueprint_application
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSelectedBlueprintAssignmentCommand {
    source: AssignmentDefinitionSourceView,
    destination: CourseInstanceWitness,
    blueprint_application: CourseInstanceBlueprintApplication,
    schedule: ResolvedRelativeAssignmentSchedule,
    replacements: CurriculumPinReplacements,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl CreateSelectedBlueprintAssignmentCommand {
    pub fn from_server_record(record: super::CreateSelectedBlueprintAssignmentApplyRecord) -> Self {
        Self {
            source: record.source(),
            destination: record.destination().clone(),
            blueprint_application: record.blueprint_application(),
            schedule: record.schedule().clone(),
            replacements: record.replacements().clone(),
            authorized_actor: record.authorized_actor(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
    }

    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.blueprint_application
    }
    pub fn schedule(&self) -> &ResolvedRelativeAssignmentSchedule {
        &self.schedule
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionCommand {
    receipt: CourseInstanceReceiptTarget,
    blueprint_application: CourseInstanceBlueprintApplication,
    authorized_actor: UserId,
    request_digest: [u8; 32],
    idempotency_key: CurriculumAdoptionIdempotencyKey,
}

impl ReconcileCourseInstanceAdoptionCommand {
    pub fn from_server_record(record: super::ReconcileCourseInstanceAdoptionApplyRecord) -> Self {
        Self {
            receipt: record.receipt().clone(),
            blueprint_application: record.blueprint_application(),
            authorized_actor: record.authorized_actor(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
    }

    pub fn receipt(&self) -> &CourseInstanceReceiptTarget {
        &self.receipt
    }
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.blueprint_application
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
