//! Rollover preservation of reusable course-tree meaning and provenance.

use crate::{CurriculumAdoptionStore, ReplaceAlphaCourseCommand, ReusableCurriculumStore};
use question_model::{
    AlphaCourseDefinitionInput, AlphaCourseModuleInput, AlphaInstantiationCommand,
    AlphaInstantiationPreviewRequest, CourseRolloverCommand, CourseRolloverPreviewRequest,
    CurriculumAdoptionTitle, CurriculumPinReplacements, ObservedAlphaSource,
};

use super::super::{course_witness, resolve_course};
use super::adoption_inputs::{definition, key};
use super::scenario::AdoptionScenario;

/// Rollover preserves the reusable course tree and binds each destination
/// baseline to one observed source assignment in that semantic traversal.
#[tokio::test]
async fn rollover_preserves_two_module_topology_and_exact_source_provenance() {
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
                    title: "Two-module rollover source".into(),
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
        reference: source_revision.reference,
        revision: source_revision.revision,
    };
    let source_preview = scenario
        .store
        .preview_alpha_instantiation(
            scenario.context,
            scenario.session,
            AlphaInstantiationPreviewRequest {
                source,
                title: CurriculumAdoptionTitle::parse("Source teaching course").expect("title"),
                target_term: scenario.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("source preview");
    let source_course = scenario
        .store
        .apply_alpha_instantiation(
            scenario.context,
            scenario.session,
            AlphaInstantiationCommand::from_preview(
                &source_preview,
                key("rollover-topology-source"),
            )
            .expect("source command"),
        )
        .await
        .expect("source apply");
    let source_witness = {
        let state = scenario.store.read_state().expect("state");
        let source_id =
            resolve_course(&state, scenario.tenant, source_course.course).expect("course");
        course_witness(&state, scenario.tenant, source_id).expect("source witness")
    };
    let target_term =
        question_model::CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago")
            .expect("target term");
    let rollover_preview = scenario
        .store
        .preview_course_rollover(
            scenario.context,
            scenario.session,
            CourseRolloverPreviewRequest {
                witness: source_witness,
                title: CurriculumAdoptionTitle::parse("Rolled topology").expect("title"),
                target_term,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("rollover preview");
    let rollover = scenario
        .store
        .apply_course_rollover(
            scenario.context,
            scenario.session,
            CourseRolloverCommand::from_preview(&rollover_preview, key("rollover-topology"))
                .expect("rollover command"),
        )
        .await
        .expect("rollover apply");

    let state = scenario.store.read_state().expect("state");
    let source_id =
        resolve_course(&state, scenario.tenant, source_course.course).expect("source course");
    let destination_id =
        resolve_course(&state, scenario.tenant, rollover.course).expect("destination course");
    let source_adoption =
        &state.curriculum_adoption.whole_course_adoptions[&(scenario.tenant, source_id)];
    let destination_adoption =
        &state.curriculum_adoption.whole_course_adoptions[&(scenario.tenant, destination_id)];
    assert_eq!(
        destination_adoption
            .payload
            .modules()
            .iter()
            .map(|module| module.label())
            .collect::<Vec<_>>(),
        source_adoption
            .payload
            .modules()
            .iter()
            .map(|module| module.label())
            .collect::<Vec<_>>(),
        "rollover retains the source semantic module order"
    );
    assert_eq!(
        destination_adoption.destination_assignments.len(),
        source_adoption.destination_assignments.len(),
        "every ordered source assignment has one destination assignment"
    );
    assert!(
        destination_adoption
            .destination_assignments
            .iter()
            .zip(source_adoption.destination_assignments.iter())
            .all(|(destination, source)| {
                let import =
                    &state.curriculum_adoption.import_records[&(scenario.tenant, *destination)];
                matches!(
                    &import.provenance.source,
                    super::super::state::StoredAssignmentImportSource::Rollover(provenance)
                        if provenance.source_course == source_course.course
                            && provenance.source_assignment.assignment
                                == state.assignment_references[&(scenario.tenant, *source)]
                            && provenance.source_assignment.revision
                                == state.assignment_revisions[&(scenario.tenant, *source)]
                )
            })
    );
}
