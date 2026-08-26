//! Receipt-led repair behavior for the current import projection.

use crate::{
    CurriculumAdoptionStore, ReplaceAlphaCourseCommand, ReusableCurriculumStore, StoreError,
};
use question_model::{
    AlphaCourseDefinitionInput, AlphaCourseModuleInput, AlphaInstantiationCommand,
    AlphaInstantiationPreviewRequest, CurriculumAdoptionReconciliationResult,
    CurriculumAdoptionTitle, CurriculumPinReplacements, ObservedAlphaSource,
    ReconcileCurriculumAdoptionCommand,
};
use std::collections::BTreeSet;

use super::super::resolve_course;
use super::adoption_inputs::{definition, key};
use super::scenario::AdoptionScenario;

/// Intact receipt-keyed evidence leaves reconciliation as an explicit no-op.
#[tokio::test]
async fn reconciliation_reports_already_consistent_without_state_change() {
    let fixture = AdoptionScenario::new().await;
    let applied = fixture.instantiate("reconcile-intact").await;
    let before = {
        let state = fixture.store.read_state().expect("state");
        (
            authoritative_snapshot(&state),
            state.curriculum_adoption.clone(),
        )
    };

    let result = fixture
        .store
        .reconcile_curriculum_adoption(
            fixture.context,
            fixture.session,
            ReconcileCurriculumAdoptionCommand {
                receipt: applied.receipt.clone(),
            },
        )
        .await
        .expect("reconciliation");

    assert!(matches!(
        result,
        CurriculumAdoptionReconciliationResult::AlreadyConsistent { receipt }
            if receipt == applied.receipt
    ));
    let state = fixture.store.read_state().expect("state");
    assert_eq!(
        (
            authoritative_snapshot(&state),
            state.curriculum_adoption.clone()
        ),
        before,
        "intact reconciliation preserves teaching authority and every B2 record"
    );
}

/// Reconciliation restores the current projection from immutable evidence and
/// leaves the authoritative course, learner, schedule, and evidence records
/// intact.
#[tokio::test]
async fn reconciliation_repairs_one_current_import_without_mutating_authority() {
    let fixture = AdoptionScenario::new().await;
    let applied = fixture.instantiate("reconcile-one").await;
    let (assignment, assignment_reference) = {
        let state = fixture.store.read_state().expect("state");
        let assignment = state.curriculum_adoption.whole_course_adoptions[&(
            fixture.tenant,
            state.courses_by_reference[&(fixture.tenant, applied.course)],
        )]
            .destination_assignments[0];
        (
            assignment,
            state.assignment_references[&(fixture.tenant, assignment)],
        )
    };
    let authoritative_before = {
        let mut state = fixture.store.write_state().expect("state");
        state
            .curriculum_adoption
            .import_records
            .remove(&(fixture.tenant, assignment));
        authoritative_snapshot(&state)
    };

    let result = fixture
        .store
        .reconcile_curriculum_adoption(
            fixture.context,
            fixture.session,
            ReconcileCurriculumAdoptionCommand {
                receipt: applied.receipt.clone(),
            },
        )
        .await
        .expect("reconciliation");

    assert!(matches!(
        result,
        CurriculumAdoptionReconciliationResult::Repaired { receipt, projections }
            if receipt == applied.receipt && projections.as_slice().iter().any(|projection| matches!(
                projection,
                question_model::CurriculumAdoptionRepairedProjection::AssignmentImportCurrent { assignment: reference }
                    if *reference == assignment_reference
            ))
    ));
    let state = fixture.store.read_state().expect("state");
    assert_eq!(
        authoritative_snapshot(&state),
        authoritative_before,
        "reconciliation changes only the repairable current import projection"
    );
}

/// An adopted course remains integrity-closed when exactly one original
/// projection is absent.  Reconciliation restores it from the receipt-keyed
/// evidence, then inspection again exposes both the original module subset
/// and the course origin.
#[tokio::test]
async fn reconciliation_restores_one_missing_original_whole_course_import() {
    let scenario = AdoptionScenario::new().await;
    let two_module_alpha = scenario
        .store
        .replace_alpha_course(
            scenario.context,
            scenario.session,
            ReplaceAlphaCourseCommand {
                reference: Some(scenario.alpha.reference),
                expected_revision: Some(scenario.alpha.revision),
                definition: AlphaCourseDefinitionInput {
                    title: "Two-module adoption source".into(),
                    modules: vec![
                        AlphaCourseModuleInput {
                            label: "Protein structure".into(),
                            definitions: vec![definition(scenario.source_question.clone())],
                        },
                        AlphaCourseModuleInput {
                            label: "Molecular recognition".into(),
                            definitions: vec![definition(scenario.replacement_question.clone())],
                        },
                    ],
                },
            },
        )
        .await
        .expect("two-module Alpha source");
    let source = ObservedAlphaSource {
        reference: two_module_alpha.reference,
        revision: two_module_alpha.revision,
    };
    let preview = scenario
        .store
        .preview_alpha_instantiation(
            scenario.context,
            scenario.session,
            AlphaInstantiationPreviewRequest {
                source,
                title: CurriculumAdoptionTitle::parse("Two-module course").expect("title"),
                target_term: scenario.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("two-module preview");
    let applied = scenario
        .store
        .apply_alpha_instantiation(
            scenario.context,
            scenario.session,
            AlphaInstantiationCommand::from_preview(&preview, key("reconcile-whole-course"))
                .expect("two-module command"),
        )
        .await
        .expect("two-module apply");
    let (missing, retained, original_references) = {
        let mut state = scenario.store.write_state().expect("state");
        let course = resolve_course(&state, scenario.tenant, applied.course).expect("course");
        let assignments = &state.curriculum_adoption.whole_course_adoptions
            [&(scenario.tenant, course)]
            .destination_assignments;
        let missing = assignments[0];
        let retained = assignments[1];
        let original_references = assignments
            .iter()
            .map(|assignment| state.assignment_references[&(scenario.tenant, *assignment)])
            .collect::<BTreeSet<_>>();
        state
            .curriculum_adoption
            .import_records
            .remove(&(scenario.tenant, missing));
        (missing, retained, original_references)
    };

    assert!(matches!(
        scenario
            .store
            .inspect_curriculum_imports(scenario.context, scenario.session, applied.course)
            .await,
        Err(StoreError::Unavailable(_))
    ));
    assert!(
        scenario
            .store
            .read_state()
            .expect("state")
            .curriculum_adoption
            .import_records
            .contains_key(&(scenario.tenant, retained))
    );

    let repaired = scenario
        .store
        .reconcile_curriculum_adoption(
            scenario.context,
            scenario.session,
            ReconcileCurriculumAdoptionCommand {
                receipt: applied.receipt.clone(),
            },
        )
        .await
        .expect("whole-course reconciliation");
    assert!(matches!(
        repaired,
        CurriculumAdoptionReconciliationResult::Repaired { receipt, .. }
            if receipt == applied.receipt
    ));
    let inspection = scenario
        .store
        .inspect_curriculum_imports(scenario.context, scenario.session, applied.course)
        .await
        .expect("repaired inspection")
        .expect("whole-course imports");
    assert_eq!(
        inspection
            .assignments
            .iter()
            .map(|import| import.assignment)
            .collect::<BTreeSet<_>>(),
        original_references
    );
    assert!(
        scenario
            .store
            .read_state()
            .expect("state")
            .curriculum_adoption
            .import_records
            .contains_key(&(scenario.tenant, missing))
    );
}

/// One whole-course receipt repairs its entire original projection as an
/// atomic unit.  Missing immutable evidence refuses before either row is
/// reconstructed.
#[tokio::test]
async fn reconciliation_repairs_multiple_current_imports_atomically() {
    let scenario = AdoptionScenario::new().await;
    let source_revision = scenario
        .store
        .replace_alpha_course(
            scenario.context,
            scenario.session,
            ReplaceAlphaCourseCommand {
                reference: Some(scenario.alpha.reference),
                expected_revision: Some(scenario.alpha.revision),
                definition: AlphaCourseDefinitionInput {
                    title: "Atomic repair source".into(),
                    modules: vec![
                        AlphaCourseModuleInput {
                            label: "Protein structure".into(),
                            definitions: vec![definition(scenario.source_question.clone())],
                        },
                        AlphaCourseModuleInput {
                            label: "Molecular recognition".into(),
                            definitions: vec![definition(scenario.replacement_question.clone())],
                        },
                    ],
                },
            },
        )
        .await
        .expect("two-module source");
    let preview = scenario
        .store
        .preview_alpha_instantiation(
            scenario.context,
            scenario.session,
            AlphaInstantiationPreviewRequest {
                source: ObservedAlphaSource {
                    reference: source_revision.reference,
                    revision: source_revision.revision,
                },
                title: CurriculumAdoptionTitle::parse("Atomic repair course").expect("title"),
                target_term: scenario.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("two-module preview");
    let applied = scenario
        .store
        .apply_alpha_instantiation(
            scenario.context,
            scenario.session,
            AlphaInstantiationCommand::from_preview(&preview, key("atomic-repair"))
                .expect("two-module command"),
        )
        .await
        .expect("two-module apply");
    let (assignments, references, authoritative_before) = {
        let mut state = scenario.store.write_state().expect("state");
        let course = resolve_course(&state, scenario.tenant, applied.course).expect("course");
        let assignments = state.curriculum_adoption.whole_course_adoptions
            [&(scenario.tenant, course)]
            .destination_assignments
            .clone();
        let references = assignments
            .iter()
            .map(|assignment| state.assignment_references[&(scenario.tenant, *assignment)])
            .collect::<BTreeSet<_>>();
        for assignment in &assignments {
            state
                .curriculum_adoption
                .import_records
                .remove(&(scenario.tenant, *assignment));
        }
        (assignments, references, authoritative_snapshot(&state))
    };

    let repaired = scenario
        .store
        .reconcile_curriculum_adoption(
            scenario.context,
            scenario.session,
            ReconcileCurriculumAdoptionCommand {
                receipt: applied.receipt.clone(),
            },
        )
        .await
        .expect("atomic repair");
    assert!(matches!(
        repaired,
        CurriculumAdoptionReconciliationResult::Repaired { receipt, projections }
            if receipt == applied.receipt
                && projections.as_slice().iter().map(|projection| match projection {
                    question_model::CurriculumAdoptionRepairedProjection::AssignmentImportCurrent { assignment } => *assignment,
                }).collect::<BTreeSet<_>>() == references
    ));
    {
        let state = scenario.store.read_state().expect("state");
        assert_eq!(authoritative_snapshot(&state), authoritative_before);
        assert!(assignments.iter().all(|assignment| {
            let current =
                &state.curriculum_adoption.import_records[&(scenario.tenant, *assignment)];
            state.curriculum_adoption.assignment_evidence.iter().any(
                |((tenant, receipt, evidence_assignment), evidence)| {
                    *tenant == scenario.tenant
                        && receipt == &applied.receipt.idempotency_key
                        && evidence_assignment == assignment
                        && current.baseline == evidence.baseline
                        && current.provenance == evidence.provenance
                },
            )
        }));
    }

    let refused_snapshot = {
        let mut state = scenario.store.write_state().expect("state");
        for assignment in &assignments {
            state
                .curriculum_adoption
                .import_records
                .remove(&(scenario.tenant, *assignment));
        }
        state.curriculum_adoption.assignment_evidence.remove(&(
            scenario.tenant,
            applied.receipt.idempotency_key.clone(),
            assignments[0],
        ));
        authoritative_snapshot(&state)
    };
    assert!(matches!(
        scenario
            .store
            .reconcile_curriculum_adoption(
                scenario.context,
                scenario.session,
                ReconcileCurriculumAdoptionCommand {
                    receipt: applied.receipt,
                },
            )
            .await,
        Err(StoreError::Unavailable(_))
    ));
    let state = scenario.store.read_state().expect("state");
    assert_eq!(authoritative_snapshot(&state), refused_snapshot);
    assert!(assignments.iter().all(|assignment| {
        !state
            .curriculum_adoption
            .import_records
            .contains_key(&(scenario.tenant, *assignment))
    }));
}

/// Reconciliation never manufactures authority from mutable current rows when
/// the immutable completed receipt is absent.
#[tokio::test]
async fn reconciliation_refuses_missing_immutable_receipt_without_mutating_state() {
    let scenario = AdoptionScenario::new().await;
    let applied = scenario.instantiate("reconcile-missing-receipt").await;
    {
        let mut state = scenario.store.write_state().expect("state");
        state
            .curriculum_adoption
            .receipts
            .remove(&(scenario.tenant, applied.receipt.idempotency_key.clone()));
    }
    let corrupted = authoritative_snapshot(&scenario.store.read_state().expect("state"));

    assert!(matches!(
        scenario
            .store
            .reconcile_curriculum_adoption(
                scenario.context,
                scenario.session,
                ReconcileCurriculumAdoptionCommand {
                    receipt: applied.receipt,
                },
            )
            .await,
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(
        authoritative_snapshot(&scenario.store.read_state().expect("state")),
        corrupted,
        "a missing immutable receipt leaves every authoritative record unchanged"
    );
}

fn authoritative_snapshot(
    state: &crate::in_memory::State,
) -> impl PartialEq + std::fmt::Debug + use<> {
    (
        (
            state.courses.clone(),
            state.course_references.clone(),
            state.courses_by_reference.clone(),
            state.assignments.clone(),
            state.assignment_references.clone(),
            state.assignments_by_reference.clone(),
        ),
        (
            state.course_memberships.clone(),
            state.enrollments.clone(),
            state.runs.clone(),
            state.course_schedule_revisions.clone(),
            state.curriculum_adoption.assignment_evidence.clone(),
            state.curriculum_adoption.whole_course_adoptions.clone(),
            state.curriculum_adoption.receipts.clone(),
        ),
    )
}
