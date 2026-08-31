//! Non-Serde CourseInstance apply commands built only from server-held records.

use crate::{AccountId, CourseTerm, ResolvedAssignmentSchedule};

use super::{
    AssignmentSourceSnapshot, BlueprintAssignmentRevisionReference, BlueprintOperationRetryToken,
    BoundedResolvedScheduleSet, CourseInstanceCreationReservation, CourseInstanceOperationReceipt,
    CourseInstanceSnapshot, CourseOrigin, CourseRolloverManifest, QuestionRevisionSubstitutions,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolloverCourseInstanceCommand {
    source_course_instance: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    target_term: CourseTerm,
    manifest: CourseRolloverManifest,
    creation: CourseInstanceCreationReservation,
    idempotency_key: BlueprintOperationRetryToken,
}

impl RolloverCourseInstanceCommand {
    pub fn from_server_record(record: super::RolloverCourseInstanceApplyRecord) -> Self {
        let idempotency_key = record.creation().idempotency_key().clone();
        Self {
            source_course_instance: record.source_course_instance().clone(),
            course_origin: record.course_origin(),
            target_term: record.target_term().clone(),
            manifest: record.manifest().clone(),
            creation: record.creation().clone(),
            idempotency_key,
        }
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
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftCourseInstanceTermCommand {
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

impl ShiftCourseInstanceTermCommand {
    pub fn from_server_record(record: super::ShiftCourseInstanceTermApplyRecord) -> Self {
        Self {
            destination: record.destination().clone(),
            course_origin: record.course_origin(),
            target_term: record.target_term().clone(),
            schedules: record.schedules().clone(),
            authorized_account: record.authorized_account(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
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
    pub fn schedules(&self) -> &[ResolvedAssignmentSchedule] {
        self.schedules.as_slice()
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledUpdateBlueprintAssignmentCommand {
    source: BlueprintAssignmentRevisionReference,
    import: AssignmentSourceSnapshot,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

impl ControlledUpdateBlueprintAssignmentCommand {
    pub fn from_server_record(
        record: super::ControlledUpdateBlueprintAssignmentApplyRecord,
    ) -> Self {
        Self {
            source: record.source(),
            import: record.import().clone(),
            destination: record.destination().clone(),
            course_origin: record.course_origin(),
            authorized_account: record.authorized_account(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
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
        self.authorized_account
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSelectedBlueprintAssignmentCommand {
    source: BlueprintAssignmentRevisionReference,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    schedule: ResolvedAssignmentSchedule,
    replacements: QuestionRevisionSubstitutions,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

impl CreateSelectedBlueprintAssignmentCommand {
    pub fn from_server_record(record: super::CopyAssignmentFromBlueprintApplyRecord) -> Self {
        Self {
            source: record.source(),
            destination: record.destination().clone(),
            course_origin: record.course_origin(),
            schedule: record.schedule().clone(),
            replacements: record.replacements().clone(),
            authorized_account: record.authorized_account(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
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
        self.authorized_account
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionCommand {
    receipt: CourseInstanceOperationReceipt,
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    request_digest: [u8; 32],
    idempotency_key: BlueprintOperationRetryToken,
}

impl ReconcileCourseInstanceAdoptionCommand {
    pub fn from_server_record(record: super::ReconcileCourseInstanceAdoptionApplyRecord) -> Self {
        Self {
            receipt: record.receipt().clone(),
            course_origin: record.course_origin(),
            authorized_account: record.authorized_account(),
            request_digest: record.request_digest(),
            idempotency_key: record.idempotency_key().clone(),
        }
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
}
