//! Completed-receipt locator and rollover-provenance integrity behavior.

use crate::{CurriculumAdoptionStore, StoreError};
use question_model::{
    BlueprintInstantiationCommand, BlueprintInstantiationPreviewRequest, CourseRolloverCommand,
    CourseRolloverPreviewRequest, CourseTerm, CurriculumAdoptionTitle, CurriculumPinReplacements,
    ForkAlphaCommand, ForkAlphaPreviewRequest, ReconcileCurriculumAdoptionCommand,
};

use super::super::{course_witness, resolve_course};
use super::adoption_inputs::key;
use super::scenario::AdoptionScenario;

/// An imported assignment cannot borrow another completed operation's receipt
/// and evidence, even when its current projection matches that forged row.
#[tokio::test]
async fn inspection_refuses_current_import_bound_to_another_completed_assignment() {
    let scenario = AdoptionScenario::new().await;
    let course = scenario.instantiate("receipt-current-import").await.course;
    let apply_blueprint = |key_suffix| {
        let store = scenario.store.clone();
        let term = scenario.term.clone();
        async move {
            let preview = store
                .preview_blueprint_instantiation(
                    scenario.context,
                    scenario.session,
                    BlueprintInstantiationPreviewRequest {
                        source: scenario.blueprint,
                        course,
                        target_term: term,
                        replacements: CurriculumPinReplacements::default(),
                    },
                )
                .await
                .expect("Blueprint preview");
            store
                .apply_blueprint_instantiation(
                    scenario.context,
                    scenario.session,
                    BlueprintInstantiationCommand::from_preview(&preview, key(key_suffix))
                        .expect("Blueprint command"),
                )
                .await
                .expect("Blueprint apply")
        }
    };
    let first = apply_blueprint("receipt-import-first").await;
    let second = apply_blueprint("receipt-import-second").await;
    let corrupted = {
        let mut state = scenario.store.write_state().expect("state");
        let first_id = *state
            .assignments_by_reference
            .get(&(scenario.tenant, first.assignment))
            .expect("first assignment");
        let second_id = *state
            .assignments_by_reference
            .get(&(scenario.tenant, second.assignment))
            .expect("second assignment");
        let forged_evidence = state.curriculum_adoption.assignment_evidence[&(
            scenario.tenant,
            second.receipt.idempotency_key.clone(),
            second_id,
        )]
            .clone();
        state.curriculum_adoption.assignment_evidence.insert(
            (
                scenario.tenant,
                second.receipt.idempotency_key.clone(),
                first_id,
            ),
            forged_evidence.clone(),
        );
        state.curriculum_adoption.import_records.insert(
            (scenario.tenant, first_id),
            super::super::state::StoredAssignmentImport {
                baseline: forged_evidence.baseline,
                provenance: forged_evidence.provenance,
            },
        );
        state.curriculum_adoption.clone()
    };

    assert!(matches!(
        scenario
            .store
            .inspect_curriculum_imports(scenario.context, scenario.session, course)
            .await,
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(
        scenario
            .store
            .read_state()
            .expect("state")
            .curriculum_adoption,
        corrupted,
        "inspection refuses corrupt provenance without repairing it"
    );
}

/// A completed Alpha-course receipt names one immutable aggregate receipt key.
#[tokio::test]
async fn alpha_course_receipt_refuses_a_duplicate_selected_key() {
    let scenario = AdoptionScenario::new().await;
    let completed = scenario.instantiate("duplicate-alpha-receipt").await;
    let duplicate = key("duplicate-alpha-receipt-key");
    let before = {
        let mut state = scenario.store.write_state().expect("state");
        let original = state.curriculum_adoption.receipts
            [&(scenario.tenant, completed.receipt.idempotency_key.clone())]
            .clone();
        state
            .curriculum_adoption
            .receipts
            .insert((scenario.tenant, duplicate.clone()), original.clone());
        (state.curriculum_adoption.clone(), original)
    };
    let state = scenario.store.read_state().expect("state");
    assert!(matches!(
        super::super::ensure_completed_outcome_binding(
            &state,
            scenario.tenant,
            &duplicate,
            &before.1,
        ),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(
        state.curriculum_adoption, before.0,
        "receipt binding validation preserves the injected duplicate"
    );
}

/// A detached Alpha reverse locator invalidates both replay and reconciliation
/// for a fork receipt; neither operation rewrites the immutable lineage.
#[tokio::test]
async fn fork_receipt_refuses_detached_destination_locator_without_mutation() {
    let scenario = AdoptionScenario::new().await;
    let preview = scenario
        .store
        .preview_fork_alpha(
            scenario.context,
            scenario.session,
            ForkAlphaPreviewRequest {
                source: scenario.alpha,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("fork preview");
    let command = ForkAlphaCommand::from_preview(&preview, key("fork-detached-locator"))
        .expect("fork command");
    let completed = scenario
        .store
        .apply_fork_alpha(scenario.context, scenario.session, command.clone())
        .await
        .expect("fork apply");
    let corrupted = {
        let mut state = scenario.store.write_state().expect("state");
        state.alpha_courses_by_reference.remove(&completed.alpha);
        (
            state.curriculum_adoption.clone(),
            state.alpha_course_references.clone(),
            state
                .alpha_courses_by_reference
                .contains_key(&completed.alpha),
        )
    };

    assert!(matches!(
        scenario
            .store
            .apply_fork_alpha(scenario.context, scenario.session, command)
            .await,
        Err(StoreError::Unavailable(_))
    ));
    assert!(matches!(
        scenario
            .store
            .reconcile_curriculum_adoption(
                scenario.context,
                scenario.session,
                ReconcileCurriculumAdoptionCommand {
                    receipt: completed.receipt,
                },
            )
            .await,
        Err(StoreError::Unavailable(_))
    ));
    let state = scenario.store.read_state().expect("state");
    assert_eq!(
        (
            state.curriculum_adoption.clone(),
            state.alpha_course_references.clone(),
            state
                .alpha_courses_by_reference
                .contains_key(&completed.alpha),
        ),
        corrupted,
        "receipt failures preserve the injected locator corruption without repair"
    );
}

/// Rollover receipt evidence binds the source course and assignment witness.
/// A corrupted immutable source provenance prevents replay and inspection.
#[tokio::test]
async fn rollover_receipt_refuses_misbound_immutable_source_provenance() {
    let scenario = AdoptionScenario::new().await;
    let source = scenario.instantiate("receipt-rollover-source").await;
    let witness = {
        let state = scenario.store.read_state().expect("state");
        let course = resolve_course(&state, scenario.tenant, source.course).expect("course");
        course_witness(&state, scenario.tenant, course).expect("witness")
    };
    let preview = scenario
        .store
        .preview_course_rollover(
            scenario.context,
            scenario.session,
            CourseRolloverPreviewRequest {
                witness,
                title: CurriculumAdoptionTitle::parse("Receipt rollover").expect("title"),
                target_term: CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago")
                    .expect("target term"),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("rollover preview");
    let command = CourseRolloverCommand::from_preview(&preview, key("receipt-rollover"))
        .expect("rollover command");
    let completed = scenario
        .store
        .apply_course_rollover(scenario.context, scenario.session, command.clone())
        .await
        .expect("rollover apply");
    let duplicate = key("receipt-rollover-duplicate-key");
    let duplicate_receipt = {
        let mut state = scenario.store.write_state().expect("state");
        let original = state.curriculum_adoption.receipts
            [&(scenario.tenant, completed.receipt.idempotency_key.clone())]
            .clone();
        state
            .curriculum_adoption
            .receipts
            .insert((scenario.tenant, duplicate.clone()), original.clone());
        original
    };
    let duplicate_refused = {
        let state = scenario.store.read_state().expect("state");
        let refused = matches!(
            super::super::ensure_completed_outcome_binding(
                &state,
                scenario.tenant,
                &duplicate,
                &duplicate_receipt,
            ),
            Err(StoreError::Unavailable(_))
        );
        (refused, state.curriculum_adoption.clone())
    };
    assert!(duplicate_refused.0);
    assert_eq!(
        scenario
            .store
            .read_state()
            .expect("state")
            .curriculum_adoption,
        duplicate_refused.1,
        "selected-receipt refusal preserves the duplicate receipt corruption"
    );
    let corrupted = {
        let mut state = scenario.store.write_state().expect("state");
        let destination =
            resolve_course(&state, scenario.tenant, completed.course).expect("destination");
        let assignment = state.curriculum_adoption.whole_course_adoptions
            [&(scenario.tenant, destination)]
            .destination_assignments[0];
        let evidence = state
            .curriculum_adoption
            .assignment_evidence
            .get_mut(&(
                scenario.tenant,
                completed.receipt.idempotency_key.clone(),
                assignment,
            ))
            .expect("rollover evidence");
        let super::super::state::StoredAssignmentImportSource::Rollover(provenance) =
            &mut evidence.provenance.source
        else {
            panic!("rollover evidence has rollover provenance");
        };
        provenance.source_course = completed.course;
        (
            state.curriculum_adoption.clone(),
            state.assignments.clone(),
            state.course_schedule_revisions.clone(),
        )
    };

    assert!(matches!(
        scenario
            .store
            .apply_course_rollover(scenario.context, scenario.session, command)
            .await,
        Err(StoreError::Unavailable(_))
    ));
    assert!(matches!(
        scenario
            .store
            .inspect_curriculum_imports(scenario.context, scenario.session, completed.course)
            .await,
        Err(StoreError::Unavailable(_))
    ));
    let state = scenario.store.read_state().expect("state");
    assert_eq!(
        (
            state.curriculum_adoption.clone(),
            state.assignments.clone(),
            state.course_schedule_revisions.clone(),
        ),
        corrupted,
        "receipt failures preserve the injected provenance corruption without repair"
    );
}
