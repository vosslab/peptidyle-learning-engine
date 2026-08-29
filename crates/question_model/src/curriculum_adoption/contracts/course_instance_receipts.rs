//! Immutable server receipt evidence for CourseInstance operations.

use crate::{ActivityTimestamp, CourseTerm, ResolvedRelativeAssignmentSchedule, UserId};

use super::{
    AssignmentDefinitionSourceView, BoundedResolvedScheduleSet, CourseInstanceCreationWitness,
    CourseInstanceImportWitness, CourseInstanceWitness, CurriculumAdoptionIdempotencyKey,
    CurriculumImportRevision, ObservedBlueprintSource, RolloverCourseInstanceManifest,
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
    destination: CourseInstanceWitness,
    authorized_actor: UserId,
    idempotency_key: CurriculumAdoptionIdempotencyKey,
    request_digest: [u8; 32],
    server_time: ActivityTimestamp,
}

impl CourseInstanceReceiptBinding {
    fn new(
        operation: CourseInstanceOperationKind,
        destination: CourseInstanceWitness,
        authorized_actor: UserId,
        idempotency_key: CurriculumAdoptionIdempotencyKey,
        request_digest: [u8; 32],
        server_time: ActivityTimestamp,
    ) -> Self {
        Self {
            operation,
            destination,
            authorized_actor,
            idempotency_key,
            request_digest,
            server_time,
        }
    }

    pub fn operation(&self) -> CourseInstanceOperationKind {
        self.operation
    }
    pub fn destination(&self) -> &CourseInstanceWitness {
        &self.destination
    }
    /// Returns the authenticated actor bound by the consumed server apply record.
    pub fn authorized_actor(&self) -> UserId {
        self.authorized_actor
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
    created_course_instance: CourseInstanceWitness,
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
    source: AssignmentDefinitionSourceView,
    import: CourseInstanceImportWitness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSelectedBlueprintAssignmentReceipt {
    binding: CourseInstanceReceiptBinding,
    source: AssignmentDefinitionSourceView,
    schedule: ResolvedRelativeAssignmentSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileCourseInstanceAdoptionReceipt {
    binding: CourseInstanceReceiptBinding,
    source: ObservedBlueprintSource,
    import_revision: CurriculumImportRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolloverCourseInstanceReceiptError {
    CreationSourceMismatch,
    CreationTermMismatch,
    CreatedCourseMismatch,
}

impl RolloverCourseInstanceReceipt {
    /// Builds immutable rollover evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::RolloverCourseInstanceApplyRecord,
        created_course_instance: CourseInstanceWitness,
        server_time: ActivityTimestamp,
    ) -> Result<Self, RolloverCourseInstanceReceiptError> {
        let (source_course_instance, target_term, manifest, created_from) =
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
            created_course_instance,
            created_from,
            target_term,
            manifest,
            server_time,
        })
    }

    pub fn source_course_instance(&self) -> &CourseInstanceWitness {
        &self.source_course_instance
    }
    pub fn created_course_instance(&self) -> &CourseInstanceWitness {
        &self.created_course_instance
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
    /// Returns the authenticated actor bound by the rollover creation witness.
    pub fn authorized_actor(&self) -> UserId {
        self.created_from.authorized_actor()
    }
}

impl ShiftCourseInstanceTermReceipt {
    /// Builds immutable term-shift evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::ShiftCourseInstanceTermApplyRecord,
        server_time: ActivityTimestamp,
    ) -> Self {
        let (
            destination,
            target_term,
            schedules,
            authorized_actor,
            request_digest,
            idempotency_key,
        ) = record.into_receipt_parts();
        Self {
            binding: CourseInstanceReceiptBinding::new(
                CourseInstanceOperationKind::ShiftTerm,
                destination,
                authorized_actor,
                idempotency_key,
                request_digest,
                server_time,
            ),
            target_term,
            schedules,
        }
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
        server_time: ActivityTimestamp,
    ) -> Self {
        let (source, import, destination, authorized_actor, request_digest, idempotency_key) =
            record.into_receipt_parts();
        Self {
            binding: CourseInstanceReceiptBinding::new(
                CourseInstanceOperationKind::ControlledUpdate,
                destination,
                authorized_actor,
                idempotency_key,
                request_digest,
                server_time,
            ),
            source,
            import,
        }
    }

    pub fn binding(&self) -> &CourseInstanceReceiptBinding {
        &self.binding
    }
    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn import(&self) -> &CourseInstanceImportWitness {
        &self.import
    }
}

impl CreateSelectedBlueprintAssignmentReceipt {
    /// Builds immutable selected-copy evidence from the consumed server apply record.
    pub fn from_server_record(
        record: super::CreateSelectedBlueprintAssignmentApplyRecord,
        server_time: ActivityTimestamp,
    ) -> Self {
        let (source, destination, schedule, authorized_actor, request_digest, idempotency_key) =
            record.into_receipt_parts();
        Self {
            binding: CourseInstanceReceiptBinding::new(
                CourseInstanceOperationKind::SelectedCopy,
                destination,
                authorized_actor,
                idempotency_key,
                request_digest,
                server_time,
            ),
            source,
            schedule,
        }
    }

    pub fn binding(&self) -> &CourseInstanceReceiptBinding {
        &self.binding
    }
    pub fn source(&self) -> AssignmentDefinitionSourceView {
        self.source
    }
    pub fn schedule(&self) -> &ResolvedRelativeAssignmentSchedule {
        &self.schedule
    }
}

impl ReconcileCourseInstanceAdoptionReceipt {
    /// Builds immutable reconciliation evidence from its consumed receipt-targeted record.
    pub fn from_server_record(
        record: super::ReconcileCourseInstanceAdoptionApplyRecord,
        source: ObservedBlueprintSource,
        import_revision: CurriculumImportRevision,
        server_time: ActivityTimestamp,
    ) -> Self {
        let (receipt, authorized_actor, request_digest, idempotency_key) =
            record.into_receipt_parts();
        Self {
            binding: CourseInstanceReceiptBinding::new(
                CourseInstanceOperationKind::Reconcile,
                receipt.destination().clone(),
                authorized_actor,
                idempotency_key,
                request_digest,
                server_time,
            ),
            source,
            import_revision,
        }
    }

    pub fn binding(&self) -> &CourseInstanceReceiptBinding {
        &self.binding
    }
    pub fn source(&self) -> &ObservedBlueprintSource {
        &self.source
    }
    pub fn import_revision(&self) -> CurriculumImportRevision {
        self.import_revision
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

    /// Returns the authenticated actor retained by every receipt target.
    pub fn authorized_actor(&self) -> UserId {
        match self {
            Self::Rollover(receipt) => receipt.authorized_actor(),
            Self::ShiftTerm(receipt) => receipt.binding().authorized_actor(),
            Self::ControlledUpdate(receipt) => receipt.binding().authorized_actor(),
            Self::SelectedCopy(receipt) => receipt.binding().authorized_actor(),
            Self::Reconcile(receipt) => receipt.binding().authorized_actor(),
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
