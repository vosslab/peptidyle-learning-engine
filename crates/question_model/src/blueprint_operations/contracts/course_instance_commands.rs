//! Non-Serde CourseInstance apply commands built only from server-held records.

use crate::{AccountId, CourseTerm, ResolvedAssignmentSchedule};

use super::{
    AssignmentImportReceipt, AssignmentSourceSnapshot, BlueprintAssignmentRevisionReference,
    BoundedResolvedScheduleSet, CourseInstanceCreationReservation, CourseInstanceSnapshot,
    CourseOrigin, CourseRolloverManifest, QuestionRevisionSubstitutions, RequestChecksum,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyCourseForNewTermCommand {
    source_course_instance: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    target_term: CourseTerm,
    manifest: CourseRolloverManifest,
    creation: CourseInstanceCreationReservation,
}

impl CopyCourseForNewTermCommand {
    pub fn from_server_record(record: super::CopyCourseForNewTermApplyRecord) -> Self {
        Self {
            source_course_instance: record.source_course_instance().clone(),
            course_origin: record.course_origin(),
            target_term: record.target_term().clone(),
            manifest: record.manifest().clone(),
            creation: record.creation().clone(),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftCourseDatesCommand {
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
    authorized_account: AccountId,
    request_checksum: RequestChecksum,
}

impl ShiftCourseDatesCommand {
    pub fn from_server_record(record: super::ShiftCourseDatesApplyRecord) -> Self {
        Self {
            destination: record.destination().clone(),
            course_origin: record.course_origin(),
            target_term: record.target_term().clone(),
            schedules: record.schedules().clone(),
            authorized_account: record.authorized_account(),
            request_checksum: record.request_checksum(),
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
    pub fn request_checksum(&self) -> RequestChecksum {
        self.request_checksum
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyBlueprintUpdateCommand {
    source: BlueprintAssignmentRevisionReference,
    import: AssignmentSourceSnapshot,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    request_checksum: RequestChecksum,
}

impl ApplyBlueprintUpdateCommand {
    pub fn from_server_record(record: super::ApplyBlueprintUpdateApplyRecord) -> Self {
        Self {
            source: record.source(),
            import: record.import().clone(),
            destination: record.destination().clone(),
            course_origin: record.course_origin(),
            authorized_account: record.authorized_account(),
            request_checksum: record.request_checksum(),
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
    pub fn request_checksum(&self) -> RequestChecksum {
        self.request_checksum
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyAssignmentFromBlueprintCommand {
    source: BlueprintAssignmentRevisionReference,
    destination: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    schedule: ResolvedAssignmentSchedule,
    replacements: QuestionRevisionSubstitutions,
    authorized_account: AccountId,
    request_checksum: RequestChecksum,
}

impl CopyAssignmentFromBlueprintCommand {
    pub fn from_server_record(record: super::CopyAssignmentFromBlueprintApplyRecord) -> Self {
        Self {
            source: record.source(),
            destination: record.destination().clone(),
            course_origin: record.course_origin(),
            schedule: record.schedule().clone(),
            replacements: record.replacements().clone(),
            authorized_account: record.authorized_account(),
            request_checksum: record.request_checksum(),
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
    pub fn request_checksum(&self) -> RequestChecksum {
        self.request_checksum
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentImportRepairCommand {
    original_import_receipt: AssignmentImportReceipt,
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    request_checksum: RequestChecksum,
}

impl AssignmentImportRepairCommand {
    pub fn from_server_record(record: super::AssignmentImportRepairApplyRecord) -> Self {
        Self {
            original_import_receipt: record.original_import_receipt().clone(),
            course_origin: record.course_origin(),
            authorized_account: record.authorized_account(),
            request_checksum: record.request_checksum(),
        }
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
    pub fn request_checksum(&self) -> RequestChecksum {
        self.request_checksum
    }
}
