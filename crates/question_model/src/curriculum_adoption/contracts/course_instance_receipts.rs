//! Immutable server receipt evidence for CourseInstance operations.

use crate::{AccountId, ActivityTimestamp, CourseTerm, ResolvedRelativeAssignmentSchedule};

use super::{
    AppliedAssignmentImportEvidence, AssignmentImportReceiptTarget, BoundedResolvedScheduleSet,
    ControlledUpdateEffect, CourseInstanceBlueprintApplication, CourseInstanceCreationWitness,
    CourseInstanceImportWitness, CourseInstanceWitness, CurriculumAdoptionIdempotencyKey,
    CurriculumImportRevision, RolloverCourseInstanceManifest,
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
    precondition: CourseInstanceWitness,
    outcome: CourseInstanceWitness,
    blueprint_application: CourseInstanceBlueprintApplication,
    authorized_account: AccountId,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
    request_digest: [u8; 32],
    server_time: ActivityTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CourseInstanceReceiptAuthority {
    blueprint_application: CourseInstanceBlueprintApplication,
    authorized_account: AccountId,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
    request_digest: [u8; 32],
    server_time: ActivityTimestamp,
}

impl CourseInstanceReceiptBinding {
    fn new(
        operation: CourseInstanceOperationKind,
        precondition: CourseInstanceWitness,
        outcome: CourseInstanceWitness,
        authority: CourseInstanceReceiptAuthority,
    ) -> Self {
        Self {
            operation,
            precondition,
            outcome,
            blueprint_application: authority.blueprint_application,
            authorized_account: authority.authorized_account,
            idempotency_key: authority.idempotency_key,
            request_digest: authority.request_digest,
            server_time: authority.server_time,
        }
    }

    pub fn operation(&self) -> CourseInstanceOperationKind {
        self.operation
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.outcome
    }
    /// Returns the exact current witness consumed by the server-held command.
    pub fn precondition(&self) -> &CourseInstanceWitness {
        &self.precondition
    }
    /// Returns the exact resulting witness after the mutation completed.
    pub fn outcome(&self) -> &CourseInstanceWitness {
        &self.outcome
    }

    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.blueprint_application
    }
    /// Returns the authenticated account bound by the consumed server apply record.
    pub fn authorized_account(&self) -> AccountId {
        self.authorized_account
    }
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
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
    source_course_instance: CourseInstanceWitness,
    source_blueprint_application: CourseInstanceBlueprintApplication,
    created_course_instance: CourseInstanceWitness,
    created_blueprint_application: CourseInstanceBlueprintApplication,
    created_from: CourseInstanceCreationWitness,
    target_term: CourseTerm,
    manifest: RolloverCourseInstanceManifest,
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
    consumed_import: CourseInstanceImportWitness,
    applied: AppliedAssignmentImportEvidence,
    effect: ControlledUpdateEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSelectedBlueprintAssignmentReceipt {
    binding: CourseInstanceReceiptBinding,
    applied: AppliedAssignmentImportEvidence,
    schedule: ResolvedRelativeAssignmentSchedule,
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
        created_course_instance: CourseInstanceWitness,
        server_time: ActivityTimestamp,
    ) -> Result<Self, RolloverCourseInstanceReceiptError> {
        let (
            source_course_instance,
            source_blueprint_application,
            target_term,
            manifest,
            created_from,
        ) = record.into_receipt_parts();
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
            source_blueprint_application,
            created_course_instance,
            created_blueprint_application: source_blueprint_application,
            created_from,
            target_term,
            manifest,
            server_time,
        })
    }

    pub fn source_course_instance(&self) -> &CourseInstanceWitness {
        &self.source_course_instance
    }
    pub fn source_blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.source_blueprint_application
    }
    pub fn created_course_instance(&self) -> &CourseInstanceWitness {
        &self.created_course_instance
    }
    pub fn created_blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        self.created_blueprint_application
    }
    pub fn created_from(&self) -> &CourseInstanceCreationWitness {
        &self.created_from
    }
    pub fn target_term(&self) -> &CourseTerm {
        &self.target_term
    }
    pub fn manifest(&self) -> &RolloverCourseInstanceManifest {
        &self.manifest
    }
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
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
        outcome: CourseInstanceWitness,
        server_time: ActivityTimestamp,
    ) -> Result<Self, ShiftCourseInstanceTermReceiptError> {
        let (
            destination,
            blueprint_application,
            target_term,
            schedules,
            authorized_account,
            request_digest,
            idempotency_key,
        ) = record.into_receipt_parts();
        if destination.course != outcome.course {
            return Err(ShiftCourseInstanceTermReceiptError::CourseMismatch);
        }
        if destination.assignments().len() != outcome.assignments().len()
            || !destination
                .assignments()
                .iter()
                .zip(outcome.assignments())
                .all(|(before, after)| before.assignment == after.assignment)
        {
            return Err(ShiftCourseInstanceTermReceiptError::AssignmentShapeMismatch);
        }
        if !destination
            .assignments()
            .iter()
            .zip(outcome.assignments())
            .all(|(before, after)| before.revision < after.revision)
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
                    blueprint_application,
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
    pub fn schedules(&self) -> &[ResolvedRelativeAssignmentSchedule] {
        self.schedules.as_slice()
    }
}

impl ControlledUpdateBlueprintAssignmentReceipt {
    /// Builds immutable controlled-update evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::ControlledUpdateBlueprintAssignmentApplyRecord,
        outcome: CourseInstanceWitness,
        applied: AppliedAssignmentImportEvidence,
        effect: ControlledUpdateEffect,
        server_time: ActivityTimestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (
            source,
            import,
            destination,
            blueprint_application,
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
        if !destination.assignments().contains(&import.destination) {
            return Err(AssignmentReceiptError::ControlledImportMissing);
        }
        if !outcome.assignments().contains(&applied.assignment()) {
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
                    blueprint_application,
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
    pub fn consumed_import(&self) -> &CourseInstanceImportWitness {
        &self.consumed_import
    }
    pub fn applied(&self) -> &AppliedAssignmentImportEvidence {
        &self.applied
    }
    pub fn effect(&self) -> ControlledUpdateEffect {
        self.effect
    }
}

impl CreateSelectedBlueprintAssignmentReceipt {
    /// Builds immutable selected-copy evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::CreateSelectedBlueprintAssignmentApplyRecord,
        outcome: CourseInstanceWitness,
        applied: AppliedAssignmentImportEvidence,
        server_time: ActivityTimestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (
            source,
            destination,
            blueprint_application,
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
            .assignments()
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
                    blueprint_application,
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
    pub fn applied(&self) -> &AppliedAssignmentImportEvidence {
        &self.applied
    }
    pub fn schedule(&self) -> &ResolvedRelativeAssignmentSchedule {
        &self.schedule
    }
}

fn controlled_update_effect_matches(
    precondition: &CourseInstanceWitness,
    outcome: &CourseInstanceWitness,
    consumed: super::ObservedCourseInstanceAssignment,
    applied: super::ObservedCourseInstanceAssignment,
    effect: ControlledUpdateEffect,
) -> bool {
    match effect {
        ControlledUpdateEffect::SourceRevisionOnly => precondition == outcome,
        ControlledUpdateEffect::MeaningChanged => {
            precondition.schedule_revision < outcome.schedule_revision
                && precondition.assignments().len() == outcome.assignments().len()
                && precondition
                    .assignments()
                    .iter()
                    .zip(outcome.assignments())
                    .all(|(before, after)| {
                        if before.assignment == consumed.assignment {
                            *before == consumed
                                && after.assignment == applied.assignment
                                && *after == applied
                                && before.revision < after.revision
                        } else {
                            before == after
                        }
                    })
        }
    }
}

fn selected_copy_outcome_matches(
    precondition: &CourseInstanceWitness,
    outcome: &CourseInstanceWitness,
    applied: super::ObservedCourseInstanceAssignment,
) -> bool {
    outcome.schedule_revision > precondition.schedule_revision
        && outcome.assignments().len() == precondition.assignments().len() + 1
        && outcome
            .assignments()
            .starts_with(precondition.assignments())
        && outcome.assignments().last() == Some(&applied)
}

impl ReconcileCourseInstanceAdoptionReceipt {
    /// Builds immutable reconciliation evidence from its consumed receipt-targeted record.
    pub fn from_server_record(
        record: super::ReconcileCourseInstanceAdoptionApplyRecord,
        server_time: ActivityTimestamp,
    ) -> Result<Self, AssignmentReceiptError> {
        let (receipt, blueprint_application, authorized_account, request_digest, idempotency_key) =
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
                    blueprint_application,
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
pub enum CourseInstanceReceiptTarget {
    Rollover(Box<RolloverCourseInstanceReceipt>),
    ShiftTerm(ShiftCourseInstanceTermReceipt),
    ControlledUpdate(ControlledUpdateBlueprintAssignmentReceipt),
    SelectedCopy(CreateSelectedBlueprintAssignmentReceipt),
    Reconcile(ReconcileCourseInstanceAdoptionReceipt),
}

impl CourseInstanceReceiptTarget {
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
    pub fn destination(&self) -> &CourseInstanceWitness {
        match self {
            Self::Rollover(receipt) => receipt.created_course_instance(),
            Self::ShiftTerm(receipt) => receipt.binding().destination(),
            Self::ControlledUpdate(receipt) => receipt.binding().destination(),
            Self::SelectedCopy(receipt) => receipt.binding().destination(),
            Self::Reconcile(receipt) => receipt.binding().destination(),
        }
    }

    /// Returns the immutable Blueprint application bound to this destination.
    pub fn blueprint_application(&self) -> CourseInstanceBlueprintApplication {
        match self {
            Self::Rollover(receipt) => receipt.created_blueprint_application(),
            Self::ShiftTerm(receipt) => receipt.binding().blueprint_application(),
            Self::ControlledUpdate(receipt) => receipt.binding().blueprint_application(),
            Self::SelectedCopy(receipt) => receipt.binding().blueprint_application(),
            Self::Reconcile(receipt) => receipt.binding().blueprint_application(),
        }
    }

    /// Returns the exact idempotency binding for every operation receipt.
    pub fn idempotency_key(&self) -> &CurriculumAdoptionIdempotencyKey {
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
