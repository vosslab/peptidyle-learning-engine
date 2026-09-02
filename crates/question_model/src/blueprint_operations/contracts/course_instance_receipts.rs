//! Immutable server receipt evidence for CourseInstance operations.

use crate::{AccountId, CourseTerm, ResolvedAssignmentSchedule, Timestamp};

use super::{
    ApplyBlueprintUpdateEffect, AssignmentImportReceiptTarget, AssignmentSourceRecord,
    AssignmentSourceSnapshot, BoundedResolvedScheduleSet, CourseInstanceCreationReservation,
    CourseInstanceSnapshot, CourseOrigin, CourseRolloverManifest, CurriculumImportRevision,
    RequestChecksum, RequestRetryToken,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintOperationKind {
    CopyCourseForNewTerm,
    ShiftCourseDates,
    ApplyBlueprintUpdate,
    CopyAssignmentFromBlueprint,
    RepairAssignmentImport,
}

/// Immutable evidence shared by each CourseInstance operation receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInstanceReceiptBinding {
    operation: BlueprintOperationKind,
    precondition: CourseInstanceSnapshot,
    outcome: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    retry_token: RequestRetryToken,
    request_checksum: RequestChecksum,
    server_time: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CourseInstanceReceiptAuthority {
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    retry_token: RequestRetryToken,
    request_checksum: RequestChecksum,
    server_time: Timestamp,
}

impl CourseInstanceReceiptBinding {
    fn new(
        operation: BlueprintOperationKind,
        precondition: CourseInstanceSnapshot,
        outcome: CourseInstanceSnapshot,
        authority: CourseInstanceReceiptAuthority,
    ) -> Self {
        Self {
            operation,
            precondition,
            outcome,
            course_origin: authority.course_origin,
            authorized_account: authority.authorized_account,
            retry_token: authority.retry_token,
            request_checksum: authority.request_checksum,
            server_time: authority.server_time,
        }
    }

    pub fn operation(&self) -> BlueprintOperationKind {
        self.operation
    }
    pub fn destination(&self) -> &CourseInstanceSnapshot {
        &self.outcome
    }
    /// Returns the exact Course Instance Snapshot precondition consumed by the server-held command.
    pub fn precondition(&self) -> &CourseInstanceSnapshot {
        &self.precondition
    }
    /// Returns the resulting Course Instance Snapshot after the mutation completed.
    pub fn outcome(&self) -> &CourseInstanceSnapshot {
        &self.outcome
    }

    pub fn course_origin(&self) -> CourseOrigin {
        self.course_origin
    }
    /// Returns the authenticated account bound by the consumed server apply record.
    pub fn authorized_account(&self) -> AccountId {
        self.authorized_account
    }
    pub fn retry_token(&self) -> &RequestRetryToken {
        &self.retry_token
    }
    pub fn request_checksum(&self) -> RequestChecksum {
        self.request_checksum
    }
    pub fn server_time(&self) -> Timestamp {
        self.server_time
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyCourseForNewTermReceipt {
    source_course_instance: CourseInstanceSnapshot,
    source_course_origin: CourseOrigin,
    created_course_instance: CourseInstanceSnapshot,
    created_course_origin: CourseOrigin,
    created_from: CourseInstanceCreationReservation,
    target_term: CourseTerm,
    manifest: CourseRolloverManifest,
    server_time: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftCourseDatesReceipt {
    binding: CourseInstanceReceiptBinding,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyBlueprintUpdateReceipt {
    binding: CourseInstanceReceiptBinding,
    consumed_import: AssignmentSourceSnapshot,
    applied: AssignmentSourceRecord,
    effect: ApplyBlueprintUpdateEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyAssignmentFromBlueprintReceipt {
    binding: CourseInstanceReceiptBinding,
    applied: AssignmentSourceRecord,
    schedule: ResolvedAssignmentSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentImportRepairReceipt {
    binding: CourseInstanceReceiptBinding,
    original_import_target: AssignmentImportReceiptTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentReceiptError {
    CourseMismatch,
    OutcomeAssignmentMissing,
    ControlledImportMissing,
    AssignmentMismatch,
    ImportRevisionMismatch,
    SourceMismatch,
    ReplacementsMismatch,
    SelectedAssignmentAlreadyPresent,
    EffectWitnessMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyCourseForNewTermReceiptError {
    CreationSourceMismatch,
    CreationTermMismatch,
    CreatedCourseMismatch,
    BlueprintApplicationMismatch,
}

/// A Shift Course Dates receipt did not retain the exact committed delivery delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftCourseDatesReceiptError {
    CourseMismatch,
    AssignmentShapeMismatch,
    AssignmentRevisionDidNotAdvance,
    ScheduleRevisionDidNotAdvance,
}

impl CopyCourseForNewTermReceipt {
    /// Builds immutable Copy Course for New Term evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::CopyCourseForNewTermApplyRecord,
        created_course_instance: CourseInstanceSnapshot,
        server_time: Timestamp,
    ) -> Result<Self, CopyCourseForNewTermReceiptError> {
        let (source_course_instance, source_course_origin, target_term, manifest, created_from) =
            record.into_receipt_parts();
        if !created_from.matches_rollover_source(&source_course_instance) {
            return Err(CopyCourseForNewTermReceiptError::CreationSourceMismatch);
        }
        if created_from.target_term() != &target_term {
            return Err(CopyCourseForNewTermReceiptError::CreationTermMismatch);
        }
        if created_from.reserved_course() != created_course_instance.course {
            return Err(CopyCourseForNewTermReceiptError::CreatedCourseMismatch);
        }
        Ok(Self {
            source_course_instance,
            source_course_origin,
            created_course_instance,
            created_course_origin: source_course_origin,
            created_from,
            target_term,
            manifest,
            server_time,
        })
    }

    pub fn source_course_instance(&self) -> &CourseInstanceSnapshot {
        &self.source_course_instance
    }
    pub fn source_course_origin(&self) -> CourseOrigin {
        self.source_course_origin
    }
    pub fn created_course_instance(&self) -> &CourseInstanceSnapshot {
        &self.created_course_instance
    }
    pub fn created_course_origin(&self) -> CourseOrigin {
        self.created_course_origin
    }
    pub fn created_from(&self) -> &CourseInstanceCreationReservation {
        &self.created_from
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn manifest(&self) -> &CourseRolloverManifest {
        &self.manifest
    }
    pub fn retry_token(&self) -> &RequestRetryToken {
        self.created_from.retry_token()
    }
    pub fn request_checksum(&self) -> RequestChecksum {
        self.created_from.request_checksum()
    }
    pub fn server_time(&self) -> Timestamp {
        self.server_time
    }
    /// Returns the authenticated account bound by the Course Instance Creation Reservation.
    pub fn authorized_account(&self) -> AccountId {
        self.created_from.authorized_account()
    }
}

impl ShiftCourseDatesReceipt {
    /// Builds immutable Shift Course Dates evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::ShiftCourseDatesApplyRecord,
        outcome: CourseInstanceSnapshot,
        server_time: Timestamp,
    ) -> Result<Self, ShiftCourseDatesReceiptError> {
        let (
            destination,
            course_origin,
            target_term,
            schedules,
            authorized_account,
            request_checksum,
            retry_token,
        ) = record.into_receipt_parts();
        if destination.course != outcome.course {
            return Err(ShiftCourseDatesReceiptError::CourseMismatch);
        }
        if destination.assignment_revisions().len() != outcome.assignment_revisions().len()
            || !destination
                .assignment_revisions()
                .iter()
                .zip(outcome.assignment_revisions())
                .all(|(before, after)| before.assignment == after.assignment)
        {
            return Err(ShiftCourseDatesReceiptError::AssignmentShapeMismatch);
        }
        if !destination
            .assignment_revisions()
            .iter()
            .zip(outcome.assignment_revisions())
            .all(|(before, after)| before.revision_number < after.revision_number)
        {
            return Err(ShiftCourseDatesReceiptError::AssignmentRevisionDidNotAdvance);
        }
        if destination.schedule_revision >= outcome.schedule_revision {
            return Err(ShiftCourseDatesReceiptError::ScheduleRevisionDidNotAdvance);
        }
        Ok(Self {
            binding: CourseInstanceReceiptBinding::new(
                BlueprintOperationKind::ShiftCourseDates,
                destination,
                outcome,
                CourseInstanceReceiptAuthority {
                    course_origin,
                    authorized_account,
                    retry_token,
                    request_checksum,
                    server_time,
                },
            ),
            target_term,
            schedules,
        })
    }

    pub fn binding(&self) -> &CourseInstanceReceiptBinding {
        &self.binding
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn schedules(&self) -> &[ResolvedAssignmentSchedule] {
        self.schedules.as_slice()
    }
}

impl ApplyBlueprintUpdateReceipt {
    /// Builds immutable Blueprint-update evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::ApplyBlueprintUpdateApplyRecord,
        outcome: CourseInstanceSnapshot,
        applied: AssignmentSourceRecord,
        effect: ApplyBlueprintUpdateEffect,
        server_time: Timestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (
            source,
            import,
            destination,
            course_origin,
            authorized_account,
            request_checksum,
            retry_token,
        ) = record.into_receipt_parts();
        if source != applied.source()
            || import.destination.assignment != applied.assignment().assignment
        {
            return Err(AssignmentReceiptError::SourceMismatch);
        }
        if destination.course != outcome.course {
            return Err(AssignmentReceiptError::CourseMismatch);
        }
        if !destination
            .assignment_revisions()
            .contains(&import.destination)
        {
            return Err(AssignmentReceiptError::ControlledImportMissing);
        }
        if !outcome
            .assignment_revisions()
            .contains(&applied.assignment())
        {
            return Err(AssignmentReceiptError::OutcomeAssignmentMissing);
        }
        let expected_revision = import
            .import_revision
            .value()
            .checked_add(1)
            .and_then(CurriculumImportRevision::new)
            .ok_or(AssignmentReceiptError::ImportRevisionMismatch)?;
        if applied.import_revision() != expected_revision {
            return Err(AssignmentReceiptError::ImportRevisionMismatch);
        }
        if !applied.replacements().as_slice().is_empty() {
            return Err(AssignmentReceiptError::ReplacementsMismatch);
        }
        if !controlled_update_effect_matches(
            &destination,
            &outcome,
            import.destination,
            applied.assignment(),
            effect,
        ) {
            return Err(AssignmentReceiptError::EffectWitnessMismatch);
        }
        Ok(Self {
            binding: CourseInstanceReceiptBinding::new(
                BlueprintOperationKind::ApplyBlueprintUpdate,
                destination,
                outcome,
                CourseInstanceReceiptAuthority {
                    course_origin,
                    authorized_account,
                    retry_token,
                    request_checksum,
                    server_time,
                },
            ),
            consumed_import: import,
            applied,
            effect,
        })
    }

    pub fn binding(&self) -> &CourseInstanceReceiptBinding {
        &self.binding
    }
    pub fn consumed_import(&self) -> &AssignmentSourceSnapshot {
        &self.consumed_import
    }
    pub fn applied(&self) -> &AssignmentSourceRecord {
        &self.applied
    }
    pub fn effect(&self) -> ApplyBlueprintUpdateEffect {
        self.effect
    }
}

impl CopyAssignmentFromBlueprintReceipt {
    /// Builds immutable assignment-copy evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::CopyAssignmentFromBlueprintApplyRecord,
        outcome: CourseInstanceSnapshot,
        applied: AssignmentSourceRecord,
        server_time: Timestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (
            source,
            destination,
            course_origin,
            schedule,
            replacements,
            authorized_account,
            request_checksum,
            retry_token,
        ) = record.into_receipt_parts();
        if source != applied.source() {
            return Err(AssignmentReceiptError::SourceMismatch);
        }
        if replacements != *applied.replacements() {
            return Err(AssignmentReceiptError::ReplacementsMismatch);
        }
        if destination.course != outcome.course {
            return Err(AssignmentReceiptError::CourseMismatch);
        }
        if destination
            .assignment_revisions()
            .iter()
            .any(|observed| observed.assignment == applied.assignment().assignment)
        {
            return Err(AssignmentReceiptError::SelectedAssignmentAlreadyPresent);
        }
        if applied.import_revision()
            != CurriculumImportRevision::new(1).expect("bounded initial import revision")
        {
            return Err(AssignmentReceiptError::ImportRevisionMismatch);
        }
        if !selected_copy_outcome_matches(&destination, &outcome, applied.assignment()) {
            return Err(AssignmentReceiptError::OutcomeAssignmentMissing);
        }
        Ok(Self {
            binding: CourseInstanceReceiptBinding::new(
                BlueprintOperationKind::CopyAssignmentFromBlueprint,
                destination,
                outcome,
                CourseInstanceReceiptAuthority {
                    course_origin,
                    authorized_account,
                    retry_token,
                    request_checksum,
                    server_time,
                },
            ),
            applied,
            schedule,
        })
    }

    pub fn binding(&self) -> &CourseInstanceReceiptBinding {
        &self.binding
    }
    pub fn applied(&self) -> &AssignmentSourceRecord {
        &self.applied
    }
    pub fn schedule(&self) -> &ResolvedAssignmentSchedule {
        &self.schedule
    }
}

fn controlled_update_effect_matches(
    precondition: &CourseInstanceSnapshot,
    outcome: &CourseInstanceSnapshot,
    consumed: super::AssignmentRevisionReference,
    applied: super::AssignmentRevisionReference,
    effect: ApplyBlueprintUpdateEffect,
) -> bool {
    match effect {
        ApplyBlueprintUpdateEffect::SourceRevisionOnly => precondition == outcome,
        ApplyBlueprintUpdateEffect::MeaningChanged => {
            precondition.schedule_revision < outcome.schedule_revision
                && precondition.assignment_revisions().len() == outcome.assignment_revisions().len()
                && precondition
                    .assignment_revisions()
                    .iter()
                    .zip(outcome.assignment_revisions())
                    .all(|(before, after)| {
                        if before.assignment == consumed.assignment {
                            *before == consumed
                                && after.assignment == applied.assignment
                                && *after == applied
                                && before.revision_number < after.revision_number
                        } else {
                            before == after
                        }
                    })
        }
    }
}

fn selected_copy_outcome_matches(
    precondition: &CourseInstanceSnapshot,
    outcome: &CourseInstanceSnapshot,
    applied: super::AssignmentRevisionReference,
) -> bool {
    outcome.schedule_revision > precondition.schedule_revision
        && outcome.assignment_revisions().len() == precondition.assignment_revisions().len() + 1
        && outcome
            .assignment_revisions()
            .starts_with(precondition.assignment_revisions())
        && outcome.assignment_revisions().last() == Some(&applied)
}

impl AssignmentImportRepairReceipt {
    /// Builds immutable repair evidence from its consumed receipt-targeted record.
    pub fn from_server_record(
        record: super::AssignmentImportRepairApplyRecord,
        server_time: Timestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (
            original_import_receipt,
            course_origin,
            authorized_account,
            request_checksum,
            retry_token,
        ) = record.into_receipt_parts();
        let original_import_target = original_import_receipt.target();
        Ok(Self {
            binding: CourseInstanceReceiptBinding::new(
                BlueprintOperationKind::RepairAssignmentImport,
                original_import_receipt.destination().clone(),
                original_import_receipt.destination().clone(),
                CourseInstanceReceiptAuthority {
                    course_origin,
                    authorized_account,
                    retry_token,
                    request_checksum,
                    server_time,
                },
            ),
            original_import_target,
        })
    }

    pub fn binding(&self) -> &CourseInstanceReceiptBinding {
        &self.binding
    }
    /// Returns the exact original immutable import evidence repaired by this action.
    pub fn original_import_target(&self) -> &AssignmentImportReceiptTarget {
        &self.original_import_target
    }
}

/// Completed Assignment import evidence eligible for repair.
///
/// A repair rebuilds projections derived from one Assignment import. It cannot
/// target a course rollover, a term shift, or an earlier repair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssignmentImportReceipt {
    ApplyBlueprintUpdate(ApplyBlueprintUpdateReceipt),
    CopyAssignmentFromBlueprint(CopyAssignmentFromBlueprintReceipt),
}

impl AssignmentImportReceipt {
    /// Returns the Course Instance snapshot committed by this Assignment import.
    pub fn destination(&self) -> &CourseInstanceSnapshot {
        match self {
            Self::ApplyBlueprintUpdate(receipt) => receipt.binding().destination(),
            Self::CopyAssignmentFromBlueprint(receipt) => receipt.binding().destination(),
        }
    }

    /// Returns the immutable Blueprint source that established the Course Instance.
    pub fn course_origin(&self) -> CourseOrigin {
        match self {
            Self::ApplyBlueprintUpdate(receipt) => receipt.binding().course_origin(),
            Self::CopyAssignmentFromBlueprint(receipt) => receipt.binding().course_origin(),
        }
    }

    /// Returns the original import request's exact retry identity.
    pub fn retry_token(&self) -> &RequestRetryToken {
        match self {
            Self::ApplyBlueprintUpdate(receipt) => receipt.binding().retry_token(),
            Self::CopyAssignmentFromBlueprint(receipt) => receipt.binding().retry_token(),
        }
    }

    /// Returns the account that authorized the original Assignment import.
    pub fn authorized_account(&self) -> AccountId {
        match self {
            Self::ApplyBlueprintUpdate(receipt) => receipt.binding().authorized_account(),
            Self::CopyAssignmentFromBlueprint(receipt) => receipt.binding().authorized_account(),
        }
    }

    /// Returns the exact immutable import receipt that this repair targets.
    pub fn target(&self) -> AssignmentImportReceiptTarget {
        match self {
            Self::ApplyBlueprintUpdate(receipt) => AssignmentImportReceiptTarget::new(
                receipt.binding().authorized_account(),
                receipt.binding().retry_token().clone(),
                receipt.binding().outcome().course,
                receipt.applied().assignment().assignment,
                receipt.applied().import_revision(),
            ),
            Self::CopyAssignmentFromBlueprint(receipt) => AssignmentImportReceiptTarget::new(
                receipt.binding().authorized_account(),
                receipt.binding().retry_token().clone(),
                receipt.binding().outcome().course,
                receipt.applied().assignment().assignment,
                receipt.applied().import_revision(),
            ),
        }
    }
}

macro_rules! receipt_error {
    ($error:ident, $message:literal) => {
        impl std::fmt::Display for $error {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str($message)
            }
        }
        impl std::error::Error for $error {}
    };
}

receipt_error!(
    CopyCourseForNewTermReceiptError,
    "Copy Course for New Term receipt creation evidence is invalid"
);

impl std::fmt::Display for AssignmentReceiptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("assignment receipt creation evidence is invalid")
    }
}
impl std::error::Error for AssignmentReceiptError {}
