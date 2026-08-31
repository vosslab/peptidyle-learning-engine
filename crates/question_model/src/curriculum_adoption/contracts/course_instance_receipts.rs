//! Immutable server receipt evidence for CourseInstance operations.

use crate::{AccountId, ActivityTimestamp, CourseTerm, ResolvedAssignmentSchedule};

use super::{
    AssignmentImportReceiptTarget, AssignmentSourceRecord, AssignmentSourceSnapshot,
    BlueprintOperationRetryToken, BoundedResolvedScheduleSet, ControlledUpdateEffect,
    CourseInstanceCreationReservation, CourseInstanceSnapshot, CourseOrigin,
    CourseRolloverManifest, CurriculumImportRevision,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseInstanceOperationKind {
    Rollover,
    ShiftTerm,
    ControlledUpdate,
    SelectedCopy,
    Reconcile,
}

/// Immutable evidence shared by each CourseInstance operation receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CourseInstanceReceiptBinding {
    operation: CourseInstanceOperationKind,
    precondition: CourseInstanceSnapshot,
    outcome: CourseInstanceSnapshot,
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    idempotency_key: BlueprintOperationRetryToken,
    request_digest: [u8; 32],
    server_time: ActivityTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CourseInstanceReceiptAuthority {
    course_origin: CourseOrigin,
    authorized_account: AccountId,
    idempotency_key: BlueprintOperationRetryToken,
    request_digest: [u8; 32],
    server_time: ActivityTimestamp,
}

impl CourseInstanceReceiptBinding {
    fn new(
        operation: CourseInstanceOperationKind,
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
            idempotency_key: authority.idempotency_key,
            request_digest: authority.request_digest,
            server_time: authority.server_time,
        }
    }

    pub fn operation(&self) -> CourseInstanceOperationKind {
        self.operation
    }
    pub fn destination(&self) -> &CourseInstanceSnapshot {
        &self.outcome
    }
    /// Returns the exact current witness consumed by the server-held command.
    pub fn precondition(&self) -> &CourseInstanceSnapshot {
        &self.precondition
    }
    /// Returns the exact resulting witness after the mutation completed.
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
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        &self.idempotency_key
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.request_digest
    }
    pub fn server_time(&self) -> ActivityTimestamp {
        self.server_time
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolloverCourseInstanceReceipt {
    source_course_instance: CourseInstanceSnapshot,
    source_course_origin: CourseOrigin,
    created_course_instance: CourseInstanceSnapshot,
    created_course_origin: CourseOrigin,
    created_from: CourseInstanceCreationReservation,
    target_term: CourseTerm,
    manifest: CourseRolloverManifest,
    server_time: ActivityTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShiftCourseInstanceTermReceipt {
    binding: CourseInstanceReceiptBinding,
    target_term: CourseTerm,
    schedules: BoundedResolvedScheduleSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlledUpdateBlueprintAssignmentReceipt {
    binding: CourseInstanceReceiptBinding,
    consumed_import: AssignmentSourceSnapshot,
    applied: AssignmentSourceRecord,
    effect: ControlledUpdateEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyAssignmentFromBlueprintReceipt {
    binding: CourseInstanceReceiptBinding,
    applied: AssignmentSourceRecord,
    schedule: ResolvedAssignmentSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionReceipt {
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
pub enum RolloverCourseInstanceReceiptError {
    CreationSourceMismatch,
    CreationTermMismatch,
    CreatedCourseMismatch,
    BlueprintApplicationMismatch,
}

/// A term-shift receipt did not retain the exact committed delivery delta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftCourseInstanceTermReceiptError {
    CourseMismatch,
    AssignmentShapeMismatch,
    AssignmentRevisionDidNotAdvance,
    ScheduleRevisionDidNotAdvance,
}

impl RolloverCourseInstanceReceipt {
    /// Builds immutable rollover evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::RolloverCourseInstanceApplyRecord,
        created_course_instance: CourseInstanceSnapshot,
        server_time: ActivityTimestamp,
    ) -> Result<Self, RolloverCourseInstanceReceiptError> {
        let (source_course_instance, source_course_origin, target_term, manifest, created_from) =
            record.into_receipt_parts();
        if !created_from.matches_rollover_source(&source_course_instance) {
            return Err(RolloverCourseInstanceReceiptError::CreationSourceMismatch);
        }
        if created_from.target_term() != &target_term {
            return Err(RolloverCourseInstanceReceiptError::CreationTermMismatch);
        }
        if created_from.reserved_course() != created_course_instance.course {
            return Err(RolloverCourseInstanceReceiptError::CreatedCourseMismatch);
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
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        self.created_from.idempotency_key()
    }
    pub fn request_digest(&self) -> [u8; 32] {
        self.created_from.request_digest()
    }
    pub fn server_time(&self) -> ActivityTimestamp {
        self.server_time
    }
    /// Returns the authenticated account bound by the rollover creation witness.
    pub fn authorized_account(&self) -> AccountId {
        self.created_from.authorized_account()
    }
}

impl ShiftCourseInstanceTermReceipt {
    /// Builds immutable term-shift evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::ShiftCourseInstanceTermApplyRecord,
        outcome: CourseInstanceSnapshot,
        server_time: ActivityTimestamp,
    ) -> Result<Self, ShiftCourseInstanceTermReceiptError> {
        let (
            destination,
            course_origin,
            target_term,
            schedules,
            authorized_account,
            request_digest,
            idempotency_key,
        ) = record.into_receipt_parts();
        if destination.course != outcome.course {
            return Err(ShiftCourseInstanceTermReceiptError::CourseMismatch);
        }
        if destination.assignment_revisions().len() != outcome.assignment_revisions().len()
            || !destination
                .assignment_revisions()
                .iter()
                .zip(outcome.assignment_revisions())
                .all(|(before, after)| before.assignment == after.assignment)
        {
            return Err(ShiftCourseInstanceTermReceiptError::AssignmentShapeMismatch);
        }
        if !destination
            .assignment_revisions()
            .iter()
            .zip(outcome.assignment_revisions())
            .all(|(before, after)| before.revision_number < after.revision_number)
        {
            return Err(ShiftCourseInstanceTermReceiptError::AssignmentRevisionDidNotAdvance);
        }
        if destination.schedule_revision >= outcome.schedule_revision {
            return Err(ShiftCourseInstanceTermReceiptError::ScheduleRevisionDidNotAdvance);
        }
        Ok(Self {
            binding: CourseInstanceReceiptBinding::new(
                CourseInstanceOperationKind::ShiftTerm,
                destination,
                outcome,
                CourseInstanceReceiptAuthority {
                    course_origin,
                    authorized_account,
                    idempotency_key,
                    request_digest,
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

impl ControlledUpdateBlueprintAssignmentReceipt {
    /// Builds immutable controlled-update evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::ControlledUpdateBlueprintAssignmentApplyRecord,
        outcome: CourseInstanceSnapshot,
        applied: AssignmentSourceRecord,
        effect: ControlledUpdateEffect,
        server_time: ActivityTimestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (
            source,
            import,
            destination,
            course_origin,
            authorized_account,
            request_digest,
            idempotency_key,
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
                CourseInstanceOperationKind::ControlledUpdate,
                destination,
                outcome,
                CourseInstanceReceiptAuthority {
                    course_origin,
                    authorized_account,
                    idempotency_key,
                    request_digest,
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
    pub fn effect(&self) -> ControlledUpdateEffect {
        self.effect
    }
}

impl CopyAssignmentFromBlueprintReceipt {
    /// Builds immutable selected-copy evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::CopyAssignmentFromBlueprintApplyRecord,
        outcome: CourseInstanceSnapshot,
        applied: AssignmentSourceRecord,
        server_time: ActivityTimestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (
            source,
            destination,
            course_origin,
            schedule,
            replacements,
            authorized_account,
            request_digest,
            idempotency_key,
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
                CourseInstanceOperationKind::SelectedCopy,
                destination,
                outcome,
                CourseInstanceReceiptAuthority {
                    course_origin,
                    authorized_account,
                    idempotency_key,
                    request_digest,
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
    effect: ControlledUpdateEffect,
) -> bool {
    match effect {
        ControlledUpdateEffect::SourceRevisionOnly => precondition == outcome,
        ControlledUpdateEffect::MeaningChanged => {
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

impl ReconcileCourseInstanceAdoptionReceipt {
    /// Builds immutable reconciliation evidence from its consumed receipt-targeted record.
    pub fn from_server_record(
        record: super::ReconcileCourseInstanceAdoptionApplyRecord,
        server_time: ActivityTimestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (receipt, course_origin, authorized_account, request_digest, idempotency_key) =
            record.into_receipt_parts();
        let original_import_target = receipt
            .assignment_import_target()
            .ok_or(AssignmentReceiptError::AssignmentMismatch)?;
        Ok(Self {
            binding: CourseInstanceReceiptBinding::new(
                CourseInstanceOperationKind::Reconcile,
                receipt.destination().clone(),
                receipt.destination().clone(),
                CourseInstanceReceiptAuthority {
                    course_origin,
                    authorized_account,
                    idempotency_key,
                    request_digest,
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

/// Server/operator-only reconciliation target. It cannot deserialize from browser input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CourseInstanceOperationReceipt {
    Rollover(Box<RolloverCourseInstanceReceipt>),
    ShiftTerm(ShiftCourseInstanceTermReceipt),
    ControlledUpdate(ControlledUpdateBlueprintAssignmentReceipt),
    SelectedCopy(CopyAssignmentFromBlueprintReceipt),
    Reconcile(ReconcileCourseInstanceAdoptionReceipt),
}

impl CourseInstanceOperationReceipt {
    /// Returns the operation recorded by this immutable receipt target.
    pub fn operation(&self) -> CourseInstanceOperationKind {
        match self {
            Self::Rollover(_) => CourseInstanceOperationKind::Rollover,
            Self::ShiftTerm(receipt) => receipt.binding().operation(),
            Self::ControlledUpdate(receipt) => receipt.binding().operation(),
            Self::SelectedCopy(receipt) => receipt.binding().operation(),
            Self::Reconcile(receipt) => receipt.binding().operation(),
        }
    }

    /// Returns the exact committed destination retained by this receipt target.
    pub fn destination(&self) -> &CourseInstanceSnapshot {
        match self {
            Self::Rollover(receipt) => receipt.created_course_instance(),
            Self::ShiftTerm(receipt) => receipt.binding().destination(),
            Self::ControlledUpdate(receipt) => receipt.binding().destination(),
            Self::SelectedCopy(receipt) => receipt.binding().destination(),
            Self::Reconcile(receipt) => receipt.binding().destination(),
        }
    }

    /// Returns the immutable Blueprint application bound to this destination.
    pub fn course_origin(&self) -> CourseOrigin {
        match self {
            Self::Rollover(receipt) => receipt.created_course_origin(),
            Self::ShiftTerm(receipt) => receipt.binding().course_origin(),
            Self::ControlledUpdate(receipt) => receipt.binding().course_origin(),
            Self::SelectedCopy(receipt) => receipt.binding().course_origin(),
            Self::Reconcile(receipt) => receipt.binding().course_origin(),
        }
    }

    /// Returns the exact idempotency binding for every operation receipt.
    pub fn idempotency_key(&self) -> &BlueprintOperationRetryToken {
        match self {
            Self::Rollover(receipt) => receipt.idempotency_key(),
            Self::ShiftTerm(receipt) => receipt.binding().idempotency_key(),
            Self::ControlledUpdate(receipt) => receipt.binding().idempotency_key(),
            Self::SelectedCopy(receipt) => receipt.binding().idempotency_key(),
            Self::Reconcile(receipt) => receipt.binding().idempotency_key(),
        }
    }

    /// Returns the canonical request binding for every operation receipt.
    pub fn request_digest(&self) -> [u8; 32] {
        match self {
            Self::Rollover(receipt) => receipt.request_digest(),
            Self::ShiftTerm(receipt) => receipt.binding().request_digest(),
            Self::ControlledUpdate(receipt) => receipt.binding().request_digest(),
            Self::SelectedCopy(receipt) => receipt.binding().request_digest(),
            Self::Reconcile(receipt) => receipt.binding().request_digest(),
        }
    }

    /// Returns the authenticated account retained by every receipt target.
    pub fn authorized_account(&self) -> AccountId {
        match self {
            Self::Rollover(receipt) => receipt.authorized_account(),
            Self::ShiftTerm(receipt) => receipt.binding().authorized_account(),
            Self::ControlledUpdate(receipt) => receipt.binding().authorized_account(),
            Self::SelectedCopy(receipt) => receipt.binding().authorized_account(),
            Self::Reconcile(receipt) => receipt.binding().authorized_account(),
        }
    }

    /// Returns an exact immutable assignment-import locator when this receipt
    /// owns one assignment-derived projection.
    pub fn assignment_import_target(&self) -> Option<AssignmentImportReceiptTarget> {
        match self {
            Self::ControlledUpdate(receipt) => Some(AssignmentImportReceiptTarget::new(
                receipt.binding().authorized_account(),
                receipt.binding().idempotency_key().clone(),
                receipt.binding().outcome().course,
                receipt.applied().assignment().assignment,
                receipt.applied().import_revision(),
            )),
            Self::SelectedCopy(receipt) => Some(AssignmentImportReceiptTarget::new(
                receipt.binding().authorized_account(),
                receipt.binding().idempotency_key().clone(),
                receipt.binding().outcome().course,
                receipt.applied().assignment().assignment,
                receipt.applied().import_revision(),
            )),
            Self::Rollover(_) | Self::ShiftTerm(_) | Self::Reconcile(_) => None,
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
    RolloverCourseInstanceReceiptError,
    "rollover receipt creation evidence is invalid"
);

impl std::fmt::Display for AssignmentReceiptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("assignment receipt creation evidence is invalid")
    }
}
impl std::error::Error for AssignmentReceiptError {}
