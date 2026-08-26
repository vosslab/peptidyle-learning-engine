//! Primary adoption commands materialize durable course meaning.

use crate::{CurriculumAdoptionStore, ReusableCurriculumStore};
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationPreviewRequest, AssignmentDefinitionSourceView,
    AssignmentFastForwardCommand, AssignmentFastForwardPreviewRequest,
    BlueprintInstantiationCommand, BlueprintInstantiationPreviewRequest,
    CreateSourceDerivedAssignmentCommand, CurriculumAdoptionTitle, CurriculumPinReplacements,
    CurriculumReplayStatus, ForkAlphaCommand, ForkAlphaPreviewRequest,
    ObservedAlphaAssignmentSource, ObservedAlphaSource, ObservedAssignmentRevision,
    SourceDerivedAssignmentPreviewRequest,
};

use super::super::resolve_course;
use super::adoption_inputs::key;
use super::scenario::AdoptionScenario;
use crate::ReplaceAlphaCourseCommand;

#[tokio::test]
async fn alpha_instantiation_materializes_course_and_replays_same_preview() {
    let fixture = AdoptionScenario::new().await;
    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse("Fall biochemistry").expect("title"),
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("preview");
    let command = AlphaInstantiationCommand::from_preview(&preview, key("alpha-materialize"))
        .expect("corrected preview");
    let applied = fixture
        .store
        .apply_alpha_instantiation(fixture.context, fixture.session, command.clone())
        .await
        .expect("apply");
    {
        let state = fixture.store.read_state().expect("state");
        let course = resolve_course(&state, fixture.tenant, applied.course).expect("course");
        assert_eq!(
            state.curriculum_adoption.whole_course_adoptions[&(fixture.tenant, course)]
                .payload
                .title(),
            "Fall biochemistry"
        );
        assert!(
            state.course_memberships.values().any(|membership| {
                membership.course == course && membership.user == fixture.actor
            })
        );
    }
    assert_eq!(
        fixture
            .store
            .apply_alpha_instantiation(fixture.context, fixture.session, command)
            .await
            .expect("replay")
            .replay,
        CurriculumReplayStatus::Replayed
    );
}

#[tokio::test]
async fn blueprint_instantiation_materializes_pinned_import_in_adopted_course() {
    let fixture = AdoptionScenario::new().await;
    let course = fixture.instantiate("blueprint-destination").await.course;
    let preview = fixture
        .store
        .preview_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationPreviewRequest {
                source: fixture.blueprint,
                course,
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("preview");
    let applied = fixture
        .store
        .apply_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationCommand::from_preview(&preview, key("blueprint-materialize"))
                .expect("corrected preview"),
        )
        .await
        .expect("apply");
    let state = fixture.store.read_state().expect("state");
    let assignment = state.assignments_by_reference[&(fixture.tenant, applied.assignment)];
    assert_eq!(
        state.assignments[&(fixture.tenant, assignment)].course_id,
        resolve_course(&state, fixture.tenant, course).expect("course")
    );
    assert_eq!(
        state.curriculum_adoption.import_records[&(fixture.tenant, assignment)]
            .baseline
            .payload
            .title(),
        "Protein structure practice"
    );
}

#[tokio::test]
async fn fast_forward_replaces_unmodified_import_with_new_source_meaning() {
    let fixture = AdoptionScenario::new().await;
    let course = fixture.instantiate("fast-forward-course").await.course;
    let imported = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.session, course)
        .await
        .expect("inspection")
        .expect("imported course")
        .assignments
        .into_iter()
        .next()
        .expect("source import");
    let (assignment, revision) = {
        let state = fixture.store.read_state().expect("state");
        let assignment = state.assignments_by_reference[&(fixture.tenant, imported.assignment)];
        (
            assignment,
            state.assignment_revisions[&(fixture.tenant, assignment)],
        )
    };
    let mut revised = fixture.alpha_input.clone();
    revised.modules[0].definitions[0].title = "Revised protein structure".into();
    let alpha_v2 = fixture
        .store
        .replace_alpha_course(
            fixture.context,
            fixture.session,
            ReplaceAlphaCourseCommand {
                reference: Some(fixture.alpha.reference),
                expected_revision: Some(fixture.alpha.revision),
                definition: revised,
            },
        )
        .await
        .expect("source revision");
    let preview = fixture
        .store
        .preview_assignment_fast_forward(
            fixture.context,
            fixture.session,
            AssignmentFastForwardPreviewRequest {
                course,
                assignment: ObservedAssignmentRevision {
                    assignment: imported.assignment,
                    revision,
                },
                import_revision: imported.revision,
                source: AssignmentDefinitionSourceView::Alpha(
                    ObservedAlphaAssignmentSource::new(
                        ObservedAlphaSource {
                            reference: alpha_v2.reference,
                            revision: alpha_v2.revision,
                        },
                        0,
                        0,
                    )
                    .expect("source assignment"),
                ),
            },
        )
        .await
        .expect("preview");
    let applied = fixture
        .store
        .apply_assignment_fast_forward(
            fixture.context,
            fixture.session,
            AssignmentFastForwardCommand::from_preview(&preview, key("fast-forward"))
                .expect("eligible preview"),
        )
        .await
        .expect("apply");
    let state = fixture.store.read_state().expect("state");
    assert_eq!(
        state.assignments[&(fixture.tenant, assignment)].title,
        "Revised protein structure"
    );
    assert!(applied.import_revision.value() > imported.revision.value());
}

#[tokio::test]
async fn source_derived_assignment_materializes_blueprint_definition() {
    let fixture = AdoptionScenario::new().await;
    let course = fixture.instantiate("derived-destination").await.course;
    let preview = fixture
        .store
        .preview_source_derived_assignment(
            fixture.context,
            fixture.session,
            SourceDerivedAssignmentPreviewRequest {
                course,
                source: AssignmentDefinitionSourceView::Blueprint(fixture.blueprint),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("preview");
    let applied = fixture
        .store
        .create_source_derived_assignment(
            fixture.context,
            fixture.session,
            CreateSourceDerivedAssignmentCommand::from_preview(&preview, key("derived"))
                .expect("corrected preview"),
        )
        .await
        .expect("apply");
    let state = fixture.store.read_state().expect("state");
    let assignment = state.assignments_by_reference[&(fixture.tenant, applied.assignment)];
    assert_eq!(
        state.assignments[&(fixture.tenant, assignment)].title,
        "Protein structure practice"
    );
}

#[tokio::test]
async fn fork_replay_returns_original_destination_after_destination_edit() {
    let fixture = AdoptionScenario::new().await;
    let preview = fixture
        .store
        .preview_fork_alpha(
            fixture.context,
            fixture.session,
            ForkAlphaPreviewRequest {
                source: fixture.alpha,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("preview");
    let command = ForkAlphaCommand::from_preview(&preview, key("fork-replay")).expect("command");
    let forked = fixture
        .store
        .apply_fork_alpha(fixture.context, fixture.session, command.clone())
        .await
        .expect("apply");
    let existing_destination = fixture
        .store
        .get_alpha_course(fixture.context, fixture.session, forked.alpha)
        .await
        .expect("destination lookup")
        .expect("fork destination");
    let mut independently_edited = fixture.alpha_input.clone();
    independently_edited.title = "Forked alpha independently edited".into();
    let destination = fixture
        .store
        .replace_alpha_course(
            fixture.context,
            fixture.session,
            ReplaceAlphaCourseCommand {
                reference: Some(forked.alpha),
                expected_revision: Some(existing_destination.revision),
                definition: independently_edited,
            },
        )
        .await
        .expect("independent destination edit");
    assert!(destination.revision.value() > existing_destination.revision.value());
    let replay = fixture
        .store
        .apply_fork_alpha(fixture.context, fixture.session, command)
        .await
        .expect("replay");
    let state = fixture.store.read_state().expect("state");
    assert_eq!(replay.alpha, forked.alpha);
    assert_eq!(
        state.curriculum_adoption.alpha_fork_lineage[&replay.alpha].source,
        fixture.alpha
    );
}
