//! PostgreSQL relational-fact projection for imported teaching state.
//!
//! This module owns the Rust/qmodel side of assignment fast-forward,
//! Instructor import inspection, and receipt-led current-import repair.  The
//! broker supplies authenticated, locked relational facts; this module
//! validates their immutable semantic evidence and creates bounded public
//! projections. SQL transaction ownership remains in the parent adapter.

use question_model::curriculum_adoption::CurriculumSemanticPayload;
use std::collections::BTreeSet;

use question_model::{
    AssignmentFastForwardPreviewRequest, AssignmentFastForwardPreviewView, CourseScheduleWitness,
    CurriculumAdoptionReceiptBinding, CurriculumAdoptionReconciliationResult,
    CurriculumAdoptionRepairedProjection, CurriculumAdoptionRepairedProjections,
    CurriculumCourseImportView, CurriculumImportView, ObservedAssignmentRevision,
    ReconcileCurriculumAdoptionCommand,
};

use crate::StoreError;
use crate::curriculum_adoption::{
    CurrentTeachingImportInput, CurriculumImportInspectionInput, FastForwardProjectionInput,
    ObservedSemanticEnvelope, TeachingAssignmentInputV1, normalize_payload,
    normalize_teaching_assignment, plan_assignment_materialization,
    project_current_teaching_import, project_curriculum_import_inspection,
    project_fast_forward_decision, validate_semantic_evidence,
};

use super::{
    FastForwardSourceFactsV1, ImportAssignmentFactsV1, ImportFactsV1, ImportInspectionFactsV1,
    PreparedSemanticV1, ReconciliationAssignmentFactsV1, ReconciliationEvidenceV1,
    SemanticEvidenceV1, prepare_semantic,
};

/// Exact fast-forward materialization facts. SQL receives the destination
/// witness and expected assignment revision to recheck before it mutates
/// teaching state; the semantic plan carries no destination database ID.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::postgres::curriculum_adoption) struct PreparedFastForwardPlanV1 {
    pub(super) semantic: PreparedSemanticV1,
    pub(super) witness: CourseScheduleWitness,
    pub(super) assignment: question_model::AssignmentReference,
    pub(super) expected_assignment_revision: question_model::AssignmentRevision,
    pub(super) expected_import_revision: question_model::CurriculumImportRevision,
    pub(super) target_term: question_model::CourseTerm,
    pub(super) materialization: crate::curriculum_adoption::AssignmentMaterializationPlan,
}

/// One derived current-import pointer that SQL may repair from already
/// validated immutable evidence. No authoritative teaching field is present.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::postgres::curriculum_adoption) struct PreparedCurrentImportRepairV1 {
    pub(super) assignment: question_model::AssignmentReference,
    pub(super) expected_assignment_revision: question_model::AssignmentRevision,
    pub(super) receipt: CurriculumAdoptionReceiptBinding,
    pub(super) revision: question_model::CurriculumImportRevision,
}

/// Projects one authenticated immutable import baseline into the bounded
/// Instructor inspection record. The enclosing course projection performs the
/// witness and origin consistency checks after every row has validated.
pub(in crate::postgres::curriculum_adoption) fn project_import_inspection(
    facts: &ImportFactsV1,
) -> Result<Option<CurriculumCourseImportView>, StoreError> {
    let ImportFactsV1::Inspection { course } = facts else {
        return Err(invalid_facts("inspection requires inspection facts"));
    };
    project_course_import_inspection(course).map(Some)
}

fn project_course_import_inspection(
    facts: &ImportInspectionFactsV1,
) -> Result<CurriculumCourseImportView, StoreError> {
    let assignments = facts
        .assignments
        .iter()
        .map(|row| {
            let baseline = assignment_payload(&row.baseline_semantic, "inspection baseline")?;
            validate_evidence(&baseline, &row.baseline_evidence)?;
            let current = current_teaching_payload(
                &row.current_teaching,
                &facts.term,
                "inspection current teaching",
            )?;
            project_current_teaching_import(CurrentTeachingImportInput {
                assignment: row.assignment,
                source: row.source.clone(),
                revision: row.revision,
                baseline: &baseline,
                baseline_evidence: observed_evidence(&row.baseline_evidence),
                current: &current,
            })
            .map_err(semantic_error)
        })
        .collect::<Result<Vec<CurriculumImportView>, StoreError>>()?;
    ensure_import_rows_follow_witness(&facts.witness, &assignments)?;
    project_curriculum_import_inspection(CurriculumImportInspectionInput {
        witness: facts.witness.clone(),
        origin: facts.origin.clone(),
        term: facts.term.clone(),
        assignments,
    })
    .map_err(semantic_error)
}

/// Validates the common fast-forward facts and applies the shared precedence
/// matrix before a preview is exposed. The final unavailable-pin inputs are
/// supplied by the closed source facts once the broker has reauthorized the
/// exact source pins.
pub(in crate::postgres::curriculum_adoption) fn project_fast_forward(
    request: &AssignmentFastForwardPreviewRequest,
    facts: &ImportFactsV1,
) -> Result<AssignmentFastForwardPreviewView, StoreError> {
    let ImportFactsV1::FastForward {
        destination,
        source,
    } = facts
    else {
        return Err(invalid_facts("fast-forward requires fast-forward facts"));
    };
    ensure_fast_forward_request(request, destination, source)?;
    let baseline = assignment_payload(&destination.baseline_semantic, "fast-forward baseline")?;
    validate_evidence(&baseline, &destination.baseline_evidence)?;
    let current = current_teaching_payload(
        &destination.current_teaching,
        &destination.target_term,
        "fast-forward current teaching",
    )?;
    let _source = prepared_assignment_payload(source)?;
    let decision = project_fast_forward_decision(FastForwardProjectionInput {
        imported_source: destination.imported_source,
        requested_source: source.requested_source,
        current_source: source.current_source,
        baseline: &baseline,
        current: &current,
        issued_work: destination.issued_work,
        unavailable_pin: source.unavailable_pin,
        replacement_choices: source.replacement_choices.clone(),
    })
    .map_err(semantic_error)?;
    Ok(AssignmentFastForwardPreviewView {
        course: request.course,
        assignment: request.assignment,
        import_revision: request.import_revision,
        source: request.source,
        witness: destination.witness.clone(),
        decision,
    })
}

/// Builds an apply-time plan only when the same locked facts remain eligible.
/// The caller supplies the preview request repeated by the command, keeping
/// command-to-fact binding explicit at this Rust boundary.
pub(in crate::postgres::curriculum_adoption) fn prepare_fast_forward(
    request: &AssignmentFastForwardPreviewRequest,
    facts: &ImportFactsV1,
) -> Result<PreparedFastForwardPlanV1, StoreError> {
    let ImportFactsV1::FastForward {
        destination,
        source,
    } = facts
    else {
        return Err(invalid_facts(
            "fast-forward preparation requires fast-forward facts",
        ));
    };
    let preview = project_fast_forward(request, facts)?;
    if preview.decision != question_model::AssignmentFastForwardDecision::Eligible {
        return Err(StoreError::Conflict);
    }
    let (payload, semantic) =
        prepare_semantic(&source.raw_semantic, &source.resolved_replacements)?;
    let CurriculumSemanticPayload::Assignment(assignment) = payload else {
        return Err(invalid_facts(
            "fast-forward source is not assignment meaning",
        ));
    };
    let materialization = plan_assignment_materialization(&assignment, &destination.target_term)
        .map_err(semantic_error)?;
    Ok(PreparedFastForwardPlanV1 {
        semantic,
        witness: destination.witness.clone(),
        assignment: destination.assignment,
        expected_assignment_revision: destination.assignment_revision,
        expected_import_revision: destination.import_revision,
        target_term: destination.target_term.clone(),
        materialization,
    })
}

/// Selects the only newest valid immutable evidence row for every repairable
/// current-import projection. The broker supplies bounded rows under its
/// locked receipt scope; this function verifies each envelope before selecting
/// a winner, so a forged `latest` label cannot alter the repair target.
pub(in crate::postgres::curriculum_adoption) fn prepare_reconciliation(
    command: &ReconcileCurriculumAdoptionCommand,
    facts: &ImportFactsV1,
) -> Result<Vec<PreparedCurrentImportRepairV1>, StoreError> {
    let ImportFactsV1::Reconciliation {
        receipt,
        assignments,
    } = facts
    else {
        return Err(invalid_facts(
            "reconciliation requires reconciliation facts",
        ));
    };
    if receipt.receipt != command.receipt {
        return Err(invalid_facts(
            "reconciliation receipt disagrees with command",
        ));
    }
    let mut references = BTreeSet::new();
    let mut repairs = Vec::new();
    for assignment in assignments {
        if !references.insert(assignment.assignment) {
            return Err(invalid_facts("reconciliation repeats an assignment"));
        }
        let newest = newest_evidence(assignment)?;
        let pointer_matches = assignment.current_pointer.as_ref().is_some_and(|pointer| {
            pointer.receipt == newest.receipt && pointer.revision == newest.revision
        });
        if !pointer_matches {
            repairs.push(PreparedCurrentImportRepairV1 {
                assignment: assignment.assignment,
                expected_assignment_revision: assignment.expected_revision,
                receipt: newest.receipt.clone(),
                revision: newest.revision,
            });
        }
    }
    let receipt_assignments = receipt
        .destination_assignments
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if references != receipt_assignments
        || receipt_assignments.len() != receipt.destination_assignments.len()
    {
        return Err(invalid_facts(
            "reconciliation assignments disagree with the receipt destination set",
        ));
    }
    repairs.sort_unstable_by_key(|repair| repair.assignment);
    Ok(repairs)
}

/// Converts the relational materializer's bounded repair report into the
/// answer-free qmodel result after rechecking it against the same immutable
/// facts selected for preparation.
pub(in crate::postgres::curriculum_adoption) fn project_reconciliation_result(
    command: &ReconcileCurriculumAdoptionCommand,
    facts: &ImportFactsV1,
    repaired_assignments: &[question_model::AssignmentReference],
) -> Result<CurriculumAdoptionReconciliationResult, StoreError> {
    let receipt = reconciliation_receipt(facts)?;
    if receipt != &command.receipt {
        return Err(invalid_facts(
            "reconciliation receipt disagrees with command",
        ));
    }
    let repairs = prepare_reconciliation(command, facts)?;
    let expected = repairs
        .iter()
        .map(|repair| repair.assignment)
        .collect::<Vec<_>>();
    if repaired_assignments != expected {
        return Err(invalid_facts(
            "reconciliation materializer repair set disagrees with immutable evidence",
        ));
    }
    if expected.is_empty() {
        return Ok(CurriculumAdoptionReconciliationResult::AlreadyConsistent {
            receipt: command.receipt.clone(),
        });
    }
    let projections = expected
        .into_iter()
        .map(
            |assignment| CurriculumAdoptionRepairedProjection::AssignmentImportCurrent {
                assignment,
            },
        )
        .collect::<Vec<_>>();
    let projections = CurriculumAdoptionRepairedProjections::new(projections)
        .map_err(|error| invalid_facts(&error.to_string()))?;
    Ok(CurriculumAdoptionReconciliationResult::Repaired {
        receipt: command.receipt.clone(),
        projections,
    })
}

fn ensure_fast_forward_request(
    request: &AssignmentFastForwardPreviewRequest,
    destination: &ImportAssignmentFactsV1,
    source: &FastForwardSourceFactsV1,
) -> Result<(), StoreError> {
    let observed = ObservedAssignmentRevision {
        assignment: destination.assignment,
        revision: destination.assignment_revision,
    };
    if destination.witness.course != request.course
        || request.assignment != observed
        || request.import_revision != destination.import_revision
        || request.source != source.requested_source
        || !destination.witness.contains_assignment(observed)
    {
        return Err(invalid_facts(
            "fast-forward facts disagree with the request witness",
        ));
    }
    Ok(())
}

fn assignment_payload(
    input: &crate::curriculum_adoption::SemanticPayloadInputV1,
    label: &str,
) -> Result<CurriculumSemanticPayload, StoreError> {
    let payload = normalize_payload(input.clone()).map_err(semantic_error)?;
    if matches!(payload, CurriculumSemanticPayload::Assignment(_)) {
        Ok(payload)
    } else {
        Err(invalid_facts(&format!("{label} is not assignment meaning")))
    }
}

/// Rebuilds reusable comparison meaning from the broker's raw teaching state.
///
/// ASVS 1.5.2 and 2.2.1: deserialized facts remain closed, and normalization
/// failure reaches the existing unavailable invalid-facts boundary before any
/// divergence or answer-free projection is exposed.
fn current_teaching_payload(
    teaching: &TeachingAssignmentInputV1,
    term: &question_model::CourseTerm,
    label: &str,
) -> Result<CurriculumSemanticPayload, StoreError> {
    let assignment = normalize_teaching_assignment(teaching.clone(), term)
        .map_err(|error| invalid_facts(&format!("{label} cannot normalize: {error}")))?;
    Ok(CurriculumSemanticPayload::assignment(assignment))
}

fn prepared_assignment_payload(
    source: &FastForwardSourceFactsV1,
) -> Result<CurriculumSemanticPayload, StoreError> {
    let (payload, _) = prepare_semantic(&source.raw_semantic, &source.resolved_replacements)?;
    if matches!(payload, CurriculumSemanticPayload::Assignment(_)) {
        Ok(payload)
    } else {
        Err(invalid_facts(
            "fast-forward source is not assignment meaning",
        ))
    }
}

fn newest_evidence(
    facts: &ReconciliationAssignmentFactsV1,
) -> Result<&ReconciliationEvidenceV1, StoreError> {
    for evidence in &facts.immutable_evidence {
        let payload = assignment_payload(&evidence.baseline_semantic, "reconciliation baseline")?;
        validate_evidence(&payload, &evidence.baseline_evidence)?;
    }
    let newest_revision = facts
        .immutable_evidence
        .iter()
        .map(|evidence| evidence.revision)
        .max()
        .ok_or_else(|| invalid_facts("reconciliation has no immutable evidence"))?;
    let mut newest = facts
        .immutable_evidence
        .iter()
        .filter(|evidence| evidence.revision == newest_revision);
    let selected = newest
        .next()
        .ok_or_else(|| invalid_facts("reconciliation has no immutable evidence"))?;
    if newest.next().is_some() {
        return Err(invalid_facts(
            "reconciliation has duplicate newest immutable evidence",
        ));
    }
    Ok(selected)
}

fn reconciliation_receipt(
    facts: &ImportFactsV1,
) -> Result<&CurriculumAdoptionReceiptBinding, StoreError> {
    let ImportFactsV1::Reconciliation { receipt, .. } = facts else {
        return Err(invalid_facts(
            "reconciliation requires reconciliation facts",
        ));
    };
    Ok(&receipt.receipt)
}

fn validate_evidence(
    payload: &CurriculumSemanticPayload,
    evidence: &SemanticEvidenceV1,
) -> Result<(), StoreError> {
    validate_semantic_evidence(payload, observed_evidence(evidence)).map_err(semantic_error)
}

fn observed_evidence(evidence: &SemanticEvidenceV1) -> ObservedSemanticEnvelope<'_> {
    ObservedSemanticEnvelope {
        canonical_version: evidence.canonical_version,
        canonical_bytes: &evidence.canonical_bytes,
        digest: evidence.digest,
    }
}

/// A stored import subset preserves the deterministic course-witness order.
/// The qmodel validates membership; this keeps SQL's ordered relational facts
/// aligned with the same canonical witness used by the public projection.
fn ensure_import_rows_follow_witness(
    witness: &CourseScheduleWitness,
    imports: &[CurriculumImportView],
) -> Result<(), StoreError> {
    let mut previous = None;
    for import in imports {
        let position = witness
            .assignment_revisions()
            .iter()
            .position(|observed| observed.assignment == import.assignment)
            .ok_or_else(|| invalid_facts("inspection import is absent from its witness"))?;
        if previous.is_some_and(|prior| position <= prior) {
            return Err(invalid_facts(
                "inspection imports are not in deterministic witness order",
            ));
        }
        previous = Some(position);
    }
    Ok(())
}

fn semantic_error(error: impl std::fmt::Display) -> StoreError {
    StoreError::Unavailable(format!(
        "curriculum adoption import facts are invalid: {error}"
    ))
}

fn invalid_facts(message: &str) -> StoreError {
    StoreError::Unavailable(format!(
        "curriculum adoption import facts are invalid: {message}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use question_model::curriculum_adoption::{
        AssignmentDefinitionSourceView, CurriculumAssignmentImportSourceView,
        CurriculumSemanticAssignment, CurriculumSemanticAssignmentEntry, ObservedBlueprintSource,
    };
    use question_model::{
        ActivityTimestamp, AssignmentDeadlineBehavior, AssignmentFastForwardDecision,
        AssignmentInstructions, AssignmentReference, AssignmentScoringMode,
        AssignmentTeachingSettingsField, BaseAssignmentPolicy, BlueprintReference,
        BlueprintRevision, CompletionRequirement, ContinuedPractice, CourseLocalDateTime,
        CourseReference, CourseScheduleRevision, CourseTerm, CurriculumCourseImportOriginView,
        GradePolicy, LateSubmissionPolicy, LearnerDisclosurePolicy, ObservedAssignmentRevision,
        PointValue, ProblemId, ProblemVersionRef, RelativeAssignmentSchedule,
        ReusableAssignmentDefaults, RunPolicies, VariationPolicy, VersionId,
    };

    use crate::curriculum_adoption::{SemanticAssignmentEntryInputV1, semantic_payload_input};

    fn reference(value: u128) -> ProblemVersionRef {
        ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid::Uuid::from_u128(value)),
            version: VersionId::from_uuid(uuid::Uuid::from_u128(value + 1)),
        }
    }

    fn defaults() -> ReusableAssignmentDefaults {
        ReusableAssignmentDefaults {
            time_limit_seconds: None,
            attempt_limit: None,
            late_submission: LateSubmissionPolicy::Accept,
            deadline_behavior: AssignmentDeadlineBehavior::AutoSubmit,
            run_policies: RunPolicies {
                completion: CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
            learner_disclosure: LearnerDisclosurePolicy::default(),
        }
    }

    fn term() -> CourseTerm {
        CourseTerm::from_parts("2026-08-24", "2026-12-12", "America/Chicago").expect("course term")
    }

    fn current_teaching(title: &str) -> TeachingAssignmentInputV1 {
        let term = term();
        current_teaching_for_term(title, &term, "2026-09-01T17:30:00.000")
    }

    fn current_teaching_for_term(
        title: &str,
        term: &CourseTerm,
        due_local: &str,
    ) -> TeachingAssignmentInputV1 {
        let due_at = CourseLocalDateTime::parse(due_local)
            .expect("course local due time")
            .resolve_for_course(term, AssignmentTeachingSettingsField::DueAt)
            .expect("resolved due time");
        TeachingAssignmentInputV1 {
            title: title.into(),
            instructions: AssignmentInstructions::default(),
            entries: vec![SemanticAssignmentEntryInputV1::Fixed {
                reference: reference(30),
                points_possible: PointValue::from_whole(1),
                scoring_mode: AssignmentScoringMode::Normal,
            }],
            defaults: defaults(),
            base_policy: BaseAssignmentPolicy {
                due_at: Some(due_at),
                ..BaseAssignmentPolicy::default()
            },
        }
    }

    fn later_term() -> CourseTerm {
        CourseTerm::from_parts("2027-01-11", "2027-05-07", "America/New_York")
            .expect("later course term")
    }

    fn baseline() -> CurriculumSemanticPayload {
        CurriculumSemanticPayload::assignment(
            CurriculumSemanticAssignment::new(
                "Imported assignment".into(),
                AssignmentInstructions::default(),
                vec![CurriculumSemanticAssignmentEntry::Fixed {
                    reference: reference(1),
                    points_possible: PointValue::from_whole(1),
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                defaults(),
                RelativeAssignmentSchedule::default(),
            )
            .expect("semantic assignment"),
        )
    }

    fn evidence(revision: u64) -> ReconciliationEvidenceV1 {
        let payload = baseline();
        let envelope = payload.canonical_envelope();
        ReconciliationEvidenceV1 {
            receipt: CurriculumAdoptionReceiptBinding {
                idempotency_key: question_model::CurriculumAdoptionIdempotencyKey::parse(&format!(
                    "repair-{revision}"
                ))
                .expect("receipt key"),
            },
            revision: question_model::CurriculumImportRevision::new(revision)
                .expect("import revision"),
            baseline_semantic: semantic_payload_input(&payload),
            baseline_evidence: SemanticEvidenceV1 {
                canonical_version: envelope.version(),
                canonical_bytes: envelope.canonical_bytes().to_vec(),
                digest: envelope.digest().as_bytes(),
            },
        }
    }

    fn facts(evidence: Vec<ReconciliationEvidenceV1>) -> ReconciliationAssignmentFactsV1 {
        ReconciliationAssignmentFactsV1 {
            assignment: AssignmentReference::new(1).expect("assignment"),
            expected_revision: question_model::AssignmentRevision::new(1)
                .expect("assignment revision"),
            current_pointer: None,
            immutable_evidence: evidence,
        }
    }

    fn semantic_evidence(payload: &CurriculumSemanticPayload) -> SemanticEvidenceV1 {
        let envelope = payload.canonical_envelope();
        SemanticEvidenceV1 {
            canonical_version: envelope.version(),
            canonical_bytes: envelope.canonical_bytes().to_vec(),
            digest: envelope.digest().as_bytes(),
        }
    }

    fn import_assignment_facts() -> ImportAssignmentFactsV1 {
        let assignment = AssignmentReference::new(1).expect("assignment");
        let assignment_revision =
            question_model::AssignmentRevision::new(2).expect("assignment revision");
        let baseline = baseline();
        ImportAssignmentFactsV1 {
            witness: CourseScheduleWitness::new(
                CourseReference::new(1).expect("course"),
                CourseScheduleRevision::INITIAL,
                vec![ObservedAssignmentRevision {
                    assignment,
                    revision: assignment_revision,
                }],
            )
            .expect("course witness"),
            target_term: term(),
            assignment,
            assignment_revision,
            import_revision: question_model::CurriculumImportRevision::new(3)
                .expect("import revision"),
            imported_source: None,
            baseline_semantic: semantic_payload_input(&baseline),
            baseline_evidence: semantic_evidence(&baseline),
            current_teaching: current_teaching("Imported assignment"),
            issued_work: false,
        }
    }

    fn reusable_source() -> CurriculumAssignmentImportSourceView {
        CurriculumAssignmentImportSourceView::Reusable {
            definition: AssignmentDefinitionSourceView::Blueprint(ObservedBlueprintSource {
                reference: BlueprintReference::new(1).expect("blueprint"),
                revision: BlueprintRevision::new(2).expect("blueprint revision"),
            }),
        }
    }

    fn inspection_assignment_facts() -> super::super::ImportInspectionAssignmentFactsV1 {
        let baseline = baseline();
        super::super::ImportInspectionAssignmentFactsV1 {
            assignment: AssignmentReference::new(1).expect("assignment"),
            source: reusable_source(),
            revision: question_model::CurriculumImportRevision::new(3).expect("import revision"),
            baseline_semantic: semantic_payload_input(&baseline),
            baseline_evidence: semantic_evidence(&baseline),
            current_teaching: current_teaching("Imported assignment"),
        }
    }

    fn source_definition(revision: u64) -> AssignmentDefinitionSourceView {
        AssignmentDefinitionSourceView::Blueprint(ObservedBlueprintSource {
            reference: BlueprintReference::new(1).expect("blueprint"),
            revision: BlueprintRevision::new(revision).expect("blueprint revision"),
        })
    }

    fn baseline_for(
        current_teaching: &TeachingAssignmentInputV1,
        term: &CourseTerm,
    ) -> CurriculumSemanticPayload {
        CurriculumSemanticPayload::assignment(
            normalize_teaching_assignment(current_teaching.clone(), term)
                .expect("current teaching baseline normalizes"),
        )
    }

    fn fast_forward_facts(
        target_term: CourseTerm,
        baseline: CurriculumSemanticPayload,
        current_teaching: TeachingAssignmentInputV1,
    ) -> (AssignmentFastForwardPreviewRequest, ImportFactsV1) {
        let assignment = AssignmentReference::new(1).expect("assignment");
        let assignment_revision =
            question_model::AssignmentRevision::new(2).expect("assignment revision");
        let requested_source = source_definition(2);
        let request = AssignmentFastForwardPreviewRequest {
            course: CourseReference::new(1).expect("course"),
            assignment: ObservedAssignmentRevision {
                assignment,
                revision: assignment_revision,
            },
            import_revision: question_model::CurriculumImportRevision::new(3)
                .expect("import revision"),
            source: requested_source,
        };
        let facts = ImportFactsV1::FastForward {
            destination: Box::new(ImportAssignmentFactsV1 {
                witness: CourseScheduleWitness::new(
                    request.course,
                    CourseScheduleRevision::INITIAL,
                    vec![request.assignment],
                )
                .expect("course witness"),
                target_term,
                assignment,
                assignment_revision,
                import_revision: request.import_revision,
                imported_source: Some(source_definition(1)),
                baseline_semantic: semantic_payload_input(&baseline),
                baseline_evidence: semantic_evidence(&baseline),
                current_teaching,
                issued_work: false,
            }),
            source: super::super::FastForwardSourceFactsV1 {
                requested_source,
                current_source: requested_source,
                raw_semantic: semantic_payload_input(&baseline),
                resolved_replacements: Vec::new(),
                unavailable_pin: None,
                replacement_choices: None,
            },
        };
        (request, facts)
    }

    fn inspection_facts(
        term: CourseTerm,
        baseline: CurriculumSemanticPayload,
        current_teaching: TeachingAssignmentInputV1,
    ) -> ImportFactsV1 {
        let assignment = AssignmentReference::new(1).expect("assignment");
        let assignment_revision =
            question_model::AssignmentRevision::new(2).expect("assignment revision");
        ImportFactsV1::Inspection {
            course: super::super::ImportInspectionFactsV1 {
                witness: CourseScheduleWitness::new(
                    CourseReference::new(1).expect("course"),
                    CourseScheduleRevision::INITIAL,
                    vec![ObservedAssignmentRevision {
                        assignment,
                        revision: assignment_revision,
                    }],
                )
                .expect("course witness"),
                origin: CurriculumCourseImportOriginView::Ordinary,
                term,
                assignments: vec![super::super::ImportInspectionAssignmentFactsV1 {
                    assignment,
                    source: reusable_source(),
                    revision: question_model::CurriculumImportRevision::new(3)
                        .expect("import revision"),
                    baseline_semantic: semantic_payload_input(&baseline),
                    baseline_evidence: semantic_evidence(&baseline),
                    current_teaching,
                }],
            },
        }
    }

    #[test]
    fn reconciliation_selects_only_the_newest_valid_immutable_evidence() {
        let rows = facts(vec![evidence(1), evidence(2)]);
        assert_eq!(
            newest_evidence(&rows)
                .expect("newest evidence")
                .revision
                .value(),
            2
        );
    }

    #[test]
    fn reconciliation_refuses_two_rows_at_the_newest_immutable_revision() {
        let rows = facts(vec![evidence(1), evidence(2), evidence(2)]);
        assert!(newest_evidence(&rows).is_err());
    }

    #[test]
    fn reconciliation_binds_the_locked_receipt_before_preparing_a_repair() {
        let row = facts(vec![evidence(1)]);
        let locked_receipt = CurriculumAdoptionReceiptBinding {
            idempotency_key: question_model::CurriculumAdoptionIdempotencyKey::parse(
                "locked-repair",
            )
            .expect("receipt key"),
        };
        let facts = ImportFactsV1::Reconciliation {
            receipt: super::super::ReconciliationReceiptFactsV1 {
                receipt: locked_receipt,
                destination_assignments: vec![row.assignment],
            },
            assignments: vec![row],
        };
        let command = ReconcileCurriculumAdoptionCommand {
            receipt: CurriculumAdoptionReceiptBinding {
                idempotency_key: question_model::CurriculumAdoptionIdempotencyKey::parse(
                    "different-repair",
                )
                .expect("receipt key"),
            },
        };
        assert!(prepare_reconciliation(&command, &facts).is_err());
    }

    #[test]
    fn fast_forward_facts_require_raw_current_teaching_state_on_the_wire() {
        let facts = import_assignment_facts();
        let wire = serde_json::to_value(&facts).expect("facts serialize");
        let current = &wire["currentTeaching"];
        assert!(current.get("basePolicy").is_some());
        assert!(current.get("schedule").is_none());
        assert!(wire.get("currentSemantic").is_none());
        assert_eq!(
            serde_json::from_value::<ImportAssignmentFactsV1>(wire.clone())
                .expect("current teaching facts deserialize"),
            facts
        );

        let mut legacy = wire;
        let object = legacy.as_object_mut().expect("facts object");
        let current = object
            .remove("currentTeaching")
            .expect("current teaching field");
        object.insert("currentSemantic".into(), current);
        assert!(serde_json::from_value::<ImportAssignmentFactsV1>(legacy).is_err());
    }

    #[test]
    fn inspection_facts_require_raw_current_teaching_state_on_the_wire() {
        let facts = inspection_assignment_facts();
        let wire = serde_json::to_value(&facts).expect("facts serialize");
        let current = &wire["currentTeaching"];
        assert!(current.get("basePolicy").is_some());
        assert!(current.get("schedule").is_none());
        assert!(wire.get("currentSemantic").is_none());
        assert_eq!(
            serde_json::from_value::<super::super::ImportInspectionAssignmentFactsV1>(
                wire.clone(),
            )
            .expect("current teaching facts deserialize"),
            facts
        );

        let mut legacy = wire;
        let object = legacy.as_object_mut().expect("facts object");
        let current = object
            .remove("currentTeaching")
            .expect("current teaching field");
        object.insert("currentSemantic".into(), current);
        assert!(
            serde_json::from_value::<super::super::ImportInspectionAssignmentFactsV1>(legacy)
                .is_err()
        );
    }

    #[test]
    fn current_teaching_uses_the_authoritative_course_term_for_schedule_normalization() {
        let payload = current_teaching_payload(
            &current_teaching("Imported assignment"),
            &term(),
            "current teaching",
        )
        .expect("current teaching normalizes");
        let CurriculumSemanticPayload::Assignment(assignment) = payload else {
            panic!("current teaching is assignment meaning");
        };
        let due = assignment.schedule().due_at.as_ref().expect("relative due");
        assert_eq!(due.day_offset, 8);
        assert_eq!(due.local_time.as_str(), "17:30:00.000");
    }

    #[test]
    fn invalid_current_teaching_stops_at_the_invalid_facts_boundary() {
        let error = current_teaching_payload(&current_teaching(""), &term(), "current teaching")
            .expect_err("empty teaching title is invalid");
        assert!(matches!(
            error,
            StoreError::Unavailable(message) if message.starts_with(
                "curriculum adoption import facts are invalid: current teaching cannot normalize:"
            )
        ));
    }

    #[test]
    fn fast_forward_projects_eligible_raw_teaching_across_distinct_terms() {
        let autumn_term = term();
        let autumn_teaching = current_teaching_for_term(
            "Imported assignment",
            &autumn_term,
            "2026-09-01T17:30:00.000",
        );
        let (autumn_request, autumn_facts) = fast_forward_facts(
            autumn_term.clone(),
            baseline_for(&autumn_teaching, &autumn_term),
            autumn_teaching,
        );
        assert_eq!(
            project_fast_forward(&autumn_request, &autumn_facts)
                .expect("autumn fast-forward projection")
                .decision,
            AssignmentFastForwardDecision::Eligible
        );

        let spring_term = later_term();
        let spring_teaching = current_teaching_for_term(
            "Imported assignment",
            &spring_term,
            "2027-01-19T17:30:00.000",
        );
        let (spring_request, spring_facts) = fast_forward_facts(
            spring_term.clone(),
            baseline_for(&spring_teaching, &spring_term),
            spring_teaching,
        );
        assert_eq!(
            project_fast_forward(&spring_request, &spring_facts)
                .expect("spring fast-forward projection")
                .decision,
            AssignmentFastForwardDecision::Eligible
        );
    }

    #[test]
    fn inspection_projects_equivalent_raw_teaching_across_distinct_terms() {
        let autumn_term = term();
        let autumn_teaching = current_teaching_for_term(
            "Imported assignment",
            &autumn_term,
            "2026-09-01T17:30:00.000",
        );
        let autumn_facts = inspection_facts(
            autumn_term.clone(),
            baseline_for(&autumn_teaching, &autumn_term),
            autumn_teaching,
        );
        let autumn = project_import_inspection(&autumn_facts)
            .expect("autumn inspection projection")
            .expect("autumn imports");
        assert_eq!(autumn.term, autumn_term);
        assert!(matches!(
            autumn.assignments(),
            [row] if row.reusable_meaning_matches_baseline
        ));

        let spring_term = later_term();
        let spring_teaching = current_teaching_for_term(
            "Imported assignment",
            &spring_term,
            "2027-01-19T17:30:00.000",
        );
        let spring_facts = inspection_facts(
            spring_term.clone(),
            baseline_for(&spring_teaching, &spring_term),
            spring_teaching,
        );
        let spring = project_import_inspection(&spring_facts)
            .expect("spring inspection projection")
            .expect("spring imports");
        assert_eq!(spring.term, spring_term);
        assert!(matches!(
            spring.assignments(),
            [row] if row.reusable_meaning_matches_baseline
        ));
    }

    #[test]
    fn fast_forward_refuses_ambiguous_dst_current_teaching_at_the_invalid_facts_boundary() {
        let mut ambiguous_teaching = current_teaching("Imported assignment");
        ambiguous_teaching.base_policy.due_at =
            Some(ActivityTimestamp::from_unix_millis(1_793_514_600_000));
        let (request, facts) = fast_forward_facts(term(), baseline(), ambiguous_teaching);
        let error = project_fast_forward(&request, &facts)
            .expect_err("ambiguous DST teaching time must not project");
        assert!(matches!(
            error,
            StoreError::Unavailable(message) if message.starts_with(
                "curriculum adoption import facts are invalid: fast-forward current teaching cannot normalize:"
            )
        ));
    }
}
