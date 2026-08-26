//! Closed, versioned server-private DTOs for the B2 PostgreSQL broker.

use question_model::curriculum_adoption::CurriculumSemanticPayload;
use question_model::{
    AssignmentDefinitionSourceView, AssignmentReference, AssignmentRevision, CourseTerm,
    CurriculumAssignmentImportSourceView, CurriculumCourseImportOriginView,
    CurriculumImportRevision, ReplacementQuestionChoices,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::StoreError;
use crate::curriculum_adoption::{
    ResolvedPinReplacement, SemanticPayloadInputV1, TeachingAssignmentInputV1, normalize_payload,
    semantic_payload_input, substitute_resolved_pins,
};

pub(super) const BRIDGE_VERSION: u8 = 1;

mod imports;
mod integration;
mod lifecycle;
mod source_adoption;

// The adapter is deliberately the only transaction owner.  These typed
// projectors keep the source, lifecycle, and import families explicit at that
// boundary rather than accepting a generic JSON operation.
pub(super) use imports::{
    prepare_fast_forward, prepare_reconciliation, project_fast_forward, project_import_inspection,
    project_reconciliation_result,
};
pub(super) use integration::{
    complete_alpha, complete_blueprint, complete_fast_forward, complete_fork, complete_rollover,
    complete_source_derived, complete_term_shift, reconciliation_result,
};
pub(super) use lifecycle::{
    prepare_rollover, prepare_term_shift, project_rollover, project_term_shift,
};
pub(super) use source_adoption::{
    prepare_alpha, prepare_blueprint, prepare_fork, prepare_source_derived, project_alpha,
    project_blueprint, project_fork, project_source_derived,
};

use imports::{PreparedCurrentImportRepairV1, PreparedFastForwardPlanV1};
use lifecycle::{PreparedCourseRolloverPlanV1, PreparedCourseTermShiftPlanV1};
use source_adoption::{
    PreparedForkPlanV1, PreparedSourceAssignmentPlanV1, PreparedSourceCoursePlanV1,
};

/// Exact normalized meaning together with the qmodel-owned canonical evidence
/// used to bind SQL preparation to Rust materialization.  SQL treats this as
/// opaque evidence; qmodel remains the only semantic authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PreparedSemanticV1 {
    pub(super) semantic_input: SemanticPayloadInputV1,
    pub(super) canonical_version: u8,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) semantic_digest: [u8; 32],
}

/// One assignment-sized immutable evidence envelope within a whole-course
/// materialization. The course envelope preserves module labels and full
/// course meaning; this row gives every created assignment its own reusable
/// baseline for inspection and controlled updates.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PreparedCourseAssignmentV1 {
    pub(super) module_position: u16,
    pub(super) assignment_position: u16,
    pub(super) semantic: PreparedSemanticV1,
    pub(super) materialization: crate::curriculum_adoption::AssignmentMaterializationPlan,
}

/// Strict versioned response from the broker's relational snapshot phase.
///
/// This wrapper deliberately separates a harmless preview from a locked
/// preparation and an idempotent replay.  A materializer can therefore only
/// run with the `preparationId` minted by its own locked snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum SnapshotFactsV1 {
    Preview {
        version: u8,
        operation: CurriculumAdoptionBridgeOperationV1,
        facts: OperationFactsV1,
    },
    Prepare {
        version: u8,
        operation: CurriculumAdoptionBridgeOperationV1,
        preparation_id: Uuid,
        actor: question_model::UserId,
        request_sha256: [u8; 32],
        facts: OperationFactsV1,
    },
    /// Reconciliation consumes an immutable receipt to repair only derived
    /// indexes. It deliberately has no receipt-creating request digest.
    ReconciliationPrepare {
        version: u8,
        operation: CurriculumAdoptionBridgeOperationV1,
        preparation_id: Uuid,
        actor: question_model::UserId,
        facts: OperationFactsV1,
    },
    Replay {
        version: u8,
        operation: CurriculumAdoptionBridgeOperationV1,
        actor: question_model::UserId,
        request_sha256: [u8; 32],
        result: SqlAdoptionResultV1,
    },
    /// Inspection alone may return an authorized empty projection.
    Absent {
        version: u8,
        operation: CurriculumAdoptionBridgeOperationV1,
    },
}

/// Opaque Rust-owned command identity supplied when a receipt-creating
/// snapshot is locked. PostgreSQL validates the actor against the active
/// session and retains the digest with the preparation; it never recreates
/// qmodel's typed canonicalization.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MaterializationBindingV1 {
    pub(super) version: u8,
    pub(super) actor: question_model::UserId,
    pub(super) request_sha256: [u8; 32],
}

/// Rust-owned materialization envelope.  It binds the server-derived actor
/// and exact domain-separated typed command digest to the locked preparation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PreparedMaterializationV1 {
    pub(super) version: u8,
    pub(super) operation: CurriculumAdoptionBridgeOperationV1,
    pub(super) preparation_id: Uuid,
    pub(super) actor: question_model::UserId,
    pub(super) request_sha256: [u8; 32],
    pub(super) plan: MaterializationPlanV1,
}

/// Receipt-led repair preparation.  This is intentionally separate from
/// `PreparedMaterializationV1`: reconciliation repairs B2-owned derived rows
/// selected from an existing immutable receipt and cannot mint a new request
/// digest or receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PreparedReconciliationV1 {
    pub(super) version: u8,
    pub(super) operation: CurriculumAdoptionBridgeOperationV1,
    pub(super) preparation_id: Uuid,
    pub(super) actor: question_model::UserId,
    pub(super) receipt: question_model::CurriculumAdoptionReceiptBinding,
    pub(super) repairs: Vec<PreparedCurrentImportRepairV1>,
}

/// The complete closed materialization vocabulary for the typed fact broker.
///
/// The envelope owns common transaction binding (`preparation_id`, actor, and
/// request digest); each variant carries only the operation-specific facts
/// that its family projector has already revalidated.  Keeping this union at
/// the facade prevents SQL from receiving a partially overlapping set of
/// optional fields and gives the adapter one exhaustive dispatch point.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum MaterializationPlanV1 {
    ForkAlpha {
        plan: PreparedForkPlanV1,
    },
    BlueprintInstantiation {
        plan: PreparedSourceAssignmentPlanV1,
    },
    AlphaInstantiation {
        plan: PreparedSourceCoursePlanV1,
    },
    CourseRollover {
        plan: PreparedCourseRolloverPlanV1,
    },
    CourseTermShift {
        plan: PreparedCourseTermShiftPlanV1,
    },
    AssignmentFastForward {
        plan: PreparedFastForwardPlanV1,
    },
    SourceDerivedAssignment {
        plan: PreparedSourceAssignmentPlanV1,
    },
}

/// Operation-tagged private relational facts.  Browser-safe projections are
/// produced only after qmodel normalization, schedule resolution, and
/// evidence validation have completed in Rust.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum OperationFactsV1 {
    ForkAlpha {
        source: SourceFactsV1,
    },
    BlueprintInstantiation {
        source: SourceFactsV1,
        destination: DestinationFactsV1,
    },
    AlphaInstantiation {
        source: SourceFactsV1,
    },
    CourseRollover {
        source: LifecycleFactsV1,
    },
    CourseTermShift {
        course: LifecycleFactsV1,
    },
    AssignmentFastForward {
        import: Box<ImportFactsV1>,
    },
    SourceDerivedAssignment {
        source: SourceFactsV1,
        destination: DestinationFactsV1,
    },
    Inspection {
        inspection: Box<ImportFactsV1>,
    },
    Reconcile {
        reconciliation: Box<ImportFactsV1>,
    },
}

/// Source fact shape shared by the source-adoption family.  `raw_semantic` is
/// exact pinned meaning selected under broker authority, never a browser DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SourceFactsV1 {
    pub(super) requested_source: SourceBindingV1,
    pub(super) current_source: SourceBindingV1,
    pub(super) raw_semantic: SemanticPayloadInputV1,
    pub(super) resolved_replacements: Vec<ResolvedPinReplacement>,
    pub(super) target_term: Option<CourseTerm>,
    pub(super) requested_replacements: question_model::CurriculumPinReplacements,
    pub(super) pin_availability: PinAvailabilityV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum SourceBindingV1 {
    Alpha {
        source: question_model::ObservedAlphaSource,
    },
    Blueprint {
        source: question_model::ObservedBlueprintSource,
    },
    Assignment {
        source: AssignmentDefinitionSourceView,
    },
}

/// Destination concurrency evidence read and locked by the broker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DestinationFactsV1 {
    pub(super) witness: question_model::CourseScheduleWitness,
}

/// Course lifecycle facts retain existing course topology/source ordering
/// separately from reusable semantic meaning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LifecycleFactsV1 {
    /// Existing teaching course title and absolute teaching-policy assignments.
    /// Rust projects this through the shared relative-schedule normalizer.
    pub(super) source_title: String,
    pub(super) source_term: CourseTerm,
    pub(super) modules: Vec<TeachingCourseModuleV1>,
    pub(super) resolved_replacements: Vec<ResolvedPinReplacement>,
    pub(super) target_term: Option<CourseTerm>,
    pub(super) witness: Option<question_model::CourseScheduleWitness>,
    pub(super) ordered_rollover_sources: Vec<OrderedRolloverSourceV1>,
    pub(super) term_shift_eligibility: TermShiftEligibilityV1,
    pub(super) resulting_title: Option<question_model::CurriculumAdoptionTitle>,
    pub(super) requested_replacements: question_model::CurriculumPinReplacements,
    pub(super) pin_availability: PinAvailabilityV1,
}

/// Stable source identity and revision for each assignment copied by a
/// course rollover.  The lifecycle projector checks these positions against
/// the normalized course before serializing a materialization plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OrderedRolloverSourceV1 {
    pub(super) module_position: u16,
    pub(super) assignment_position: u16,
    pub(super) source_assignment_id: Uuid,
    pub(super) source_assignment_revision: question_model::AssignmentRevision,
}

/// One source-course module preserving its authored label and assignment
/// order while its assignment policies are converted to reusable meaning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TeachingCourseModuleV1 {
    pub(super) label: String,
    pub(super) assignments: Vec<crate::curriculum_adoption::TeachingAssignmentInputV1>,
}

/// Broker-authorized availability for the first unresolved exact pin. Rust
/// turns this into the public recovery action only after it has validated the
/// complete operation/request binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum PinAvailabilityV1 {
    Available,
    Unavailable {
        pin: crate::curriculum_adoption::PositionedPin,
        candidates: ReplacementQuestionChoices,
    },
}

/// Import, fast-forward, inspection, and reconciliation facts.  The closed
/// variants rule out invalid optional-field combinations before any semantic
/// projection or write reaches the broker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum ImportFactsV1 {
    FastForward {
        destination: Box<ImportAssignmentFactsV1>,
        source: FastForwardSourceFactsV1,
    },
    Inspection {
        course: ImportInspectionFactsV1,
    },
    Reconciliation {
        receipt: ReconciliationReceiptFactsV1,
        assignments: Vec<ReconciliationAssignmentFactsV1>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ImportAssignmentFactsV1 {
    pub(super) witness: question_model::CourseScheduleWitness,
    pub(super) target_term: CourseTerm,
    pub(super) assignment: AssignmentReference,
    pub(super) assignment_revision: AssignmentRevision,
    pub(super) import_revision: CurriculumImportRevision,
    pub(super) imported_source: Option<AssignmentDefinitionSourceView>,
    pub(super) baseline_semantic: SemanticPayloadInputV1,
    pub(super) baseline_evidence: SemanticEvidenceV1,
    /// ASVS 1.5.2 and 2.2.1: accept only closed stored teaching state; Rust
    /// derives its relative schedule against `target_term` before comparison.
    pub(super) current_teaching: TeachingAssignmentInputV1,
    pub(super) issued_work: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FastForwardSourceFactsV1 {
    pub(super) requested_source: AssignmentDefinitionSourceView,
    pub(super) current_source: AssignmentDefinitionSourceView,
    pub(super) raw_semantic: SemanticPayloadInputV1,
    pub(super) resolved_replacements: Vec<ResolvedPinReplacement>,
    pub(super) unavailable_pin: Option<crate::curriculum_adoption::PositionedPin>,
    pub(super) replacement_choices: Option<ReplacementQuestionChoices>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ImportInspectionFactsV1 {
    pub(super) witness: question_model::CourseScheduleWitness,
    pub(super) origin: CurriculumCourseImportOriginView,
    pub(super) term: CourseTerm,
    pub(super) assignments: Vec<ImportInspectionAssignmentFactsV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ImportInspectionAssignmentFactsV1 {
    pub(super) assignment: AssignmentReference,
    pub(super) source: CurriculumAssignmentImportSourceView,
    pub(super) revision: CurriculumImportRevision,
    pub(super) baseline_semantic: SemanticPayloadInputV1,
    pub(super) baseline_evidence: SemanticEvidenceV1,
    /// ASVS 1.5.2 and 2.2.1: the enclosing inspection term is the only
    /// authority for deriving relative schedule meaning from stored state.
    pub(super) current_teaching: TeachingAssignmentInputV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReconciliationReceiptFactsV1 {
    pub(super) receipt: question_model::CurriculumAdoptionReceiptBinding,
    pub(super) destination_assignments: Vec<AssignmentReference>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReconciliationAssignmentFactsV1 {
    pub(super) assignment: AssignmentReference,
    pub(super) expected_revision: AssignmentRevision,
    pub(super) current_pointer: Option<CurriculumImportPointerV1>,
    pub(super) immutable_evidence: Vec<ReconciliationEvidenceV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CurriculumImportPointerV1 {
    pub(super) receipt: question_model::CurriculumAdoptionReceiptBinding,
    pub(super) revision: CurriculumImportRevision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReconciliationEvidenceV1 {
    pub(super) receipt: question_model::CurriculumAdoptionReceiptBinding,
    pub(super) revision: CurriculumImportRevision,
    pub(super) baseline_semantic: SemanticPayloadInputV1,
    pub(super) baseline_evidence: SemanticEvidenceV1,
}

/// Immutable semantic facts stored with a receipt/import baseline.  Rust
/// re-derives all three values from `raw_semantic` before accepting them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SemanticEvidenceV1 {
    pub(super) canonical_version: u8,
    pub(super) canonical_bytes: Vec<u8>,
    pub(super) digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum TermShiftEligibilityV1 {
    Eligible {
        ordered_assignments: Vec<OrderedTermShiftAssignmentV1>,
    },
    IssuedWork,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OrderedTermShiftAssignmentV1 {
    pub(super) module_position: u16,
    pub(super) assignment_position: u16,
    pub(super) assignment: AssignmentReference,
    pub(super) expected_revision: AssignmentRevision,
}

/// Shared receipt facts returned by the relational materializer.  The broker
/// never serializes browser completion DTOs: Rust reconstructs those only
/// after request bindings are rechecked in the same transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SqlReceiptResultV1 {
    pub(super) idempotency_key: question_model::CurriculumAdoptionIdempotencyKey,
    pub(super) replayed: bool,
}

/// Relational-only materializer result.  The tagged operation prevents a
/// completion for one command family from being decoded as another.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum SqlAdoptionResultV1 {
    ForkAlpha {
        receipt: SqlReceiptResultV1,
        source: question_model::ObservedAlphaSource,
        alpha: question_model::AlphaCourseReference,
    },
    BlueprintInstantiation {
        receipt: SqlReceiptResultV1,
        course: question_model::CourseReference,
        assignment: AssignmentReference,
    },
    AlphaInstantiation {
        receipt: SqlReceiptResultV1,
        source: question_model::ObservedAlphaSource,
        course: question_model::CourseReference,
    },
    CourseRollover {
        receipt: SqlReceiptResultV1,
        source_course: question_model::CourseReference,
        course: question_model::CourseReference,
    },
    CourseTermShift {
        receipt: SqlReceiptResultV1,
        course: question_model::CourseReference,
        term: CourseTerm,
    },
    AssignmentFastForward {
        receipt: SqlReceiptResultV1,
        course: question_model::CourseReference,
        assignment: AssignmentReference,
        import_revision: CurriculumImportRevision,
    },
    SourceDerivedAssignment {
        receipt: SqlReceiptResultV1,
        course: question_model::CourseReference,
        assignment: AssignmentReference,
    },
    Reconcile {
        receipt: SqlReceiptResultV1,
        repaired_assignments: Vec<AssignmentReference>,
    },
}

/// Derives one prepared semantic envelope from broker-authorized exact pins.
///
/// ASVS 1.5.2 and 2.2.1: only the closed semantic input is accepted, then
/// qmodel normalizes it before its canonical bytes/digest can cross back to
/// SQL.  Each operation remains responsible for checking its own source and
/// destination witness before it asks for this common semantic preparation.
pub(super) fn prepare_semantic(
    raw_semantic: &SemanticPayloadInputV1,
    replacements: &[ResolvedPinReplacement],
) -> Result<(CurriculumSemanticPayload, PreparedSemanticV1), StoreError> {
    let normalized = normalize_payload(raw_semantic.clone()).map_err(invalid_snapshot)?;
    let payload = substitute_resolved_pins(&normalized, replacements).map_err(invalid_snapshot)?;
    Ok((payload.clone(), prepared_semantic(&payload)))
}

/// Produces qmodel's canonical evidence envelope for already normalized,
/// replacement-applied meaning. This keeps the course and assignment evidence
/// paths on one canonical-byte owner.
pub(super) fn prepared_semantic(payload: &CurriculumSemanticPayload) -> PreparedSemanticV1 {
    let envelope = payload.canonical_envelope();
    PreparedSemanticV1 {
        semantic_input: semantic_payload_input(payload),
        canonical_version: envelope.version(),
        canonical_bytes: envelope.canonical_bytes().to_vec(),
        semantic_digest: envelope.digest().as_bytes(),
    }
}

/// Builds and rechecks every assignment-sized evidence row in authored course
/// topology order. Storage never infers assignment evidence from the course
/// envelope, so later inspection and fast-forward have an exact baseline.
pub(super) fn prepare_course_assignments(
    course: &question_model::curriculum_adoption::CurriculumSemanticCourse,
    target_term: &CourseTerm,
) -> Result<Vec<PreparedCourseAssignmentV1>, StoreError> {
    let mut rows = Vec::new();
    for (module_position, module) in course.modules().iter().enumerate() {
        let module_position = u16::try_from(module_position)
            .map_err(|_| invalid_snapshot("course module position exceeds the contract bound"))?;
        for (assignment_position, assignment) in module.assignments().iter().enumerate() {
            let assignment_position = u16::try_from(assignment_position).map_err(|_| {
                invalid_snapshot("course assignment position exceeds the contract bound")
            })?;
            let payload = CurriculumSemanticPayload::assignment(assignment.clone());
            let materialization = crate::curriculum_adoption::plan_assignment_materialization(
                assignment,
                target_term,
            )
            .map_err(invalid_snapshot)?;
            rows.push(PreparedCourseAssignmentV1 {
                module_position,
                assignment_position,
                semantic: prepared_semantic(&payload),
                materialization,
            });
        }
    }
    validate_prepared_course_assignments(course, target_term, &rows)?;
    Ok(rows)
}

/// Rejects any detached, reordered, or flattened assignment evidence before a
/// whole-course plan crosses the broker boundary.
pub(super) fn validate_prepared_course_assignments(
    course: &question_model::curriculum_adoption::CurriculumSemanticCourse,
    target_term: &CourseTerm,
    rows: &[PreparedCourseAssignmentV1],
) -> Result<(), StoreError> {
    let expected =
        course
            .modules()
            .iter()
            .enumerate()
            .flat_map(|(module_position, module)| {
                module.assignments().iter().enumerate().map(
                    move |(assignment_position, assignment)| {
                        (module_position, assignment_position, assignment)
                    },
                )
            })
            .collect::<Vec<_>>();
    if rows.len() != expected.len() {
        return Err(invalid_snapshot(
            "course assignment evidence count disagrees with course tree",
        ));
    }
    for ((module_position, assignment_position, assignment), row) in expected.iter().zip(rows) {
        let module_position = u16::try_from(*module_position)
            .map_err(|_| invalid_snapshot("course module position exceeds the contract bound"))?;
        let assignment_position = u16::try_from(*assignment_position).map_err(|_| {
            invalid_snapshot("course assignment position exceeds the contract bound")
        })?;
        let payload = CurriculumSemanticPayload::assignment((*assignment).clone());
        let materialization =
            crate::curriculum_adoption::plan_assignment_materialization(assignment, target_term)
                .map_err(invalid_snapshot)?;
        if row.module_position != module_position
            || row.assignment_position != assignment_position
            || row.semantic != prepared_semantic(&payload)
            || row.materialization != materialization
        {
            return Err(invalid_snapshot(
                "course assignment evidence disagrees with course tree",
            ));
        }
    }
    Ok(())
}

/// Rebuilds reusable course meaning from real teaching assignments without
/// treating resolved absolute schedules as reusable source data.
pub(super) fn prepare_lifecycle_semantic(
    title: &str,
    source_term: &CourseTerm,
    modules: &[TeachingCourseModuleV1],
    replacements: &[ResolvedPinReplacement],
) -> Result<(CurriculumSemanticPayload, PreparedSemanticV1), StoreError> {
    let modules = modules
        .iter()
        .map(|module| {
            let assignments = module
                .assignments
                .iter()
                .cloned()
                .map(|assignment| {
                    crate::curriculum_adoption::normalize_teaching_assignment(
                        assignment,
                        source_term,
                    )
                    .map_err(invalid_snapshot)
                })
                .collect::<Result<Vec<_>, _>>()?;
            question_model::curriculum_adoption::CurriculumSemanticModule::new(
                module.label.clone(),
                assignments,
            )
            .map_err(invalid_snapshot)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let course = question_model::curriculum_adoption::CurriculumSemanticCourse::new(
        title.to_owned(),
        modules,
    )
    .map_err(invalid_snapshot)?;
    let input = semantic_payload_input(&CurriculumSemanticPayload::course(course));
    prepare_semantic(&input, replacements)
}

/// Computes the shared domain-separated typed receipt binding.  The broker
/// receives the digest as evidence; it never hashes or canonicalizes JSON.
pub(super) fn request_digest<T: Serialize>(
    operation: CurriculumAdoptionBridgeOperationV1,
    actor: question_model::UserId,
    request: &T,
) -> Result<[u8; 32], StoreError> {
    crate::curriculum_adoption::request_digest(
        operation.request_digest_operation()?,
        actor,
        request,
    )
    .map(|digest| *digest.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum CurriculumAdoptionBridgeOperationV1 {
    PreviewForkAlpha,
    ApplyForkAlpha,
    PreviewBlueprintInstantiation,
    ApplyBlueprintInstantiation,
    PreviewAlphaInstantiation,
    ApplyAlphaInstantiation,
    PreviewCourseRollover,
    ApplyCourseRollover,
    PreviewCourseTermShift,
    ApplyCourseTermShift,
    PreviewAssignmentFastForward,
    ApplyAssignmentFastForward,
    PreviewSourceDerivedAssignment,
    CreateSourceDerivedAssignment,
    InspectImports,
    Reconcile,
}

impl CurriculumAdoptionBridgeOperationV1 {
    fn request_digest_operation(
        self,
    ) -> Result<crate::curriculum_adoption::CurriculumAdoptionOperation, StoreError> {
        use crate::curriculum_adoption::CurriculumAdoptionOperation as Operation;
        match self {
            Self::ApplyForkAlpha => Ok(Operation::ForkAlpha),
            Self::ApplyBlueprintInstantiation => Ok(Operation::InstantiateBlueprint),
            Self::ApplyAlphaInstantiation => Ok(Operation::InstantiateAlpha),
            Self::ApplyCourseRollover => Ok(Operation::RolloverCourse),
            Self::ApplyCourseTermShift => Ok(Operation::ShiftCourseTerm),
            Self::ApplyAssignmentFastForward => Ok(Operation::FastForwardAssignment),
            Self::CreateSourceDerivedAssignment => Ok(Operation::CreateSourceDerivedAssignment),
            Self::PreviewForkAlpha
            | Self::PreviewBlueprintInstantiation
            | Self::PreviewAlphaInstantiation
            | Self::PreviewCourseRollover
            | Self::PreviewCourseTermShift
            | Self::PreviewAssignmentFastForward
            | Self::PreviewSourceDerivedAssignment
            | Self::InspectImports
            | Self::Reconcile => Err(StoreError::Unavailable(
                "curriculum adoption operation has no receipt digest".into(),
            )),
        }
    }
}

impl SnapshotFactsV1 {
    pub(super) fn validate_for(
        &self,
        expected: CurriculumAdoptionBridgeOperationV1,
    ) -> Result<(), StoreError> {
        let (version, operation) = match self {
            Self::Preview {
                version, operation, ..
            }
            | Self::Prepare {
                version, operation, ..
            }
            | Self::ReconciliationPrepare {
                version, operation, ..
            }
            | Self::Replay {
                version, operation, ..
            }
            | Self::Absent { version, operation } => (*version, *operation),
        };
        if version != BRIDGE_VERSION || operation != expected {
            return Err(StoreError::Unavailable(
                "curriculum adoption broker returned a mismatched snapshot".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn preview_facts(
        &self,
        expected: CurriculumAdoptionBridgeOperationV1,
    ) -> Result<&OperationFactsV1, StoreError> {
        self.validate_for(expected)?;
        match self {
            Self::Preview { facts, .. } => Ok(facts),
            _ => Err(StoreError::Unavailable(
                "curriculum adoption broker did not return preview facts".into(),
            )),
        }
    }

    pub(super) fn preparation(
        &self,
        expected: CurriculumAdoptionBridgeOperationV1,
    ) -> Result<(Uuid, question_model::UserId, [u8; 32], &OperationFactsV1), StoreError> {
        self.validate_for(expected)?;
        match self {
            Self::Prepare {
                preparation_id,
                actor,
                request_sha256,
                facts,
                ..
            } => Ok((*preparation_id, *actor, *request_sha256, facts)),
            _ => Err(StoreError::Unavailable(
                "curriculum adoption broker did not return locked preparation facts".into(),
            )),
        }
    }

    pub(super) fn reconciliation_preparation(
        &self,
    ) -> Result<(Uuid, question_model::UserId, &OperationFactsV1), StoreError> {
        self.validate_for(CurriculumAdoptionBridgeOperationV1::Reconcile)?;
        match self {
            Self::ReconciliationPrepare {
                preparation_id,
                actor,
                facts,
                ..
            } => Ok((*preparation_id, *actor, facts)),
            _ => Err(StoreError::Unavailable(
                "curriculum adoption broker did not return reconciliation preparation facts".into(),
            )),
        }
    }
}

fn invalid_snapshot(error: impl std::fmt::Display) -> StoreError {
    StoreError::Unavailable(format!("curriculum adoption snapshot is invalid: {error}"))
}
