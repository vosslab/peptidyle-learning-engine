use crate::{
    CourseCreationAuthority, CourseRecord, CreateCourseCommand, CurriculumAdoptionStore,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store, StoreError,
};
use question_model::curriculum_adoption::{
    CurriculumAssignmentImportSourceView, CurriculumCourseImportOriginView,
    CurriculumSemanticAssignment, CurriculumSemanticCourse, CurriculumSemanticModule,
    CurriculumSemanticPayload,
};
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationPreviewRequest, AssignmentDefinitionSourceView,
    AssignmentId, AssignmentReference, AssignmentRevision, BlueprintInstantiationCommand,
    BlueprintInstantiationPreviewRequest, CourseId, CourseRolloverCommand,
    CourseRolloverPreviewRequest, CourseScheduleWitness, CourseTerm, CurriculumAdoptionTitle,
    CurriculumPinReplacements, UserRole,
};
use uuid::Uuid;

use super::super::{course_assignment_ids, course_witness, resolve_course};
use super::adoption_inputs::key;
use super::scenario::AdoptionScenario;

/// A course adoption aggregate keeps its normalized course payload and each
/// immutable assignment baseline bound together.  A shape-preserving semantic
/// substitution must therefore fail closed, even if its replacement digest is
/// internally consistent.
#[tokio::test]
async fn substituted_course_semantics_refuse_replay_atomically() {
    let fixture = AdoptionScenario::new().await;
    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse("Semantic evidence").expect("title"),
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("preview");
    let command = AlphaInstantiationCommand::from_preview(&preview, key("semantic-evidence"))
        .expect("command");
    let applied = fixture
        .store
        .apply_alpha_instantiation(fixture.context, fixture.session, command.clone())
        .await
        .expect("apply");

    let (course, before_assignments, before_evidence, substituted) = {
        let mut state = fixture.store.write_state().expect("state");
        let course = resolve_course(&state, fixture.tenant, applied.course).expect("course");
        let record = state
            .curriculum_adoption
            .whole_course_adoptions
            .get(&(fixture.tenant, course))
            .expect("course adoption");
        let substituted = substitute_first_assignment(&record.payload);
        let before_assignments = state.assignments.clone();
        let before_evidence = state.curriculum_adoption.assignment_evidence.clone();
        let adoption = state
            .curriculum_adoption
            .whole_course_adoptions
            .get_mut(&(fixture.tenant, course))
            .expect("course adoption");
        adoption.payload = substituted.clone();
        adoption.digest = CurriculumSemanticPayload::course(substituted.clone()).digest();
        (course, before_assignments, before_evidence, substituted)
    };

    let result = fixture
        .store
        .apply_alpha_instantiation(fixture.context, fixture.session, command)
        .await;
    assert!(matches!(result, Err(StoreError::Unavailable(_))));

    let state = fixture.store.read_state().expect("state");
    let adoption = &state.curriculum_adoption.whole_course_adoptions[&(fixture.tenant, course)];
    assert_eq!(adoption.payload, substituted);
    assert_eq!(
        state.assignments, before_assignments,
        "a refused receipt replay leaves the assignment aggregate unchanged"
    );
    assert_eq!(
        state.curriculum_adoption.assignment_evidence, before_evidence,
        "a refused receipt replay leaves immutable evidence unchanged"
    );
}

#[tokio::test]
async fn inspection_closes_ordinary_alpha_and_rollover_provenance() {
    let fixture = AdoptionScenario::new().await;

    let alpha = fixture.instantiate("inspect-alpha").await;
    let alpha_view = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.session, alpha.course)
        .await
        .expect("Alpha inspection")
        .expect("Alpha import");
    assert!(matches!(
        alpha_view.origin,
        CurriculumCourseImportOriginView::Alpha { source } if source == fixture.alpha
    ));
    assert!(
        alpha_view
            .assignments
            .iter()
            .find_map(|import| match &import.source {
                CurriculumAssignmentImportSourceView::Reusable {
                    definition: AssignmentDefinitionSourceView::Alpha(source),
                } if source.source() == fixture.alpha => Some(source),
                _ => None,
            })
            .is_some()
    );

    let later_alpha_preview = fixture
        .store
        .preview_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationPreviewRequest {
                source: fixture.blueprint,
                course: alpha.course,
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("later Alpha-course preview");
    fixture
        .store
        .apply_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationCommand::from_preview(
                &later_alpha_preview,
                key("inspect-alpha-later"),
            )
            .expect("later Alpha-course command"),
        )
        .await
        .expect("later Alpha-course apply");
    let alpha_with_later_import = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.session, alpha.course)
        .await
        .expect("Alpha inspection after later import")
        .expect("Alpha import");
    assert!(
        alpha_with_later_import
            .assignments
            .iter()
            .any(|import| matches!(
                import.source,
                CurriculumAssignmentImportSourceView::Reusable {
                    definition: AssignmentDefinitionSourceView::Blueprint(source),
                } if source == fixture.blueprint
            ))
    );

    let ordinary_course = CourseId::from_uuid(Uuid::from_u128(121_050));
    let ordinary_session = SessionTokenHash::compute(b"curriculum-adoption-ordinary-inspection");
    fixture
        .store
        .create_session(
            ordinary_session,
            SessionSubject::new(
                fixture.tenant,
                fixture.actor,
                "Inspection Instructor",
                vec![UserRole::Instructor],
            )
            .expect("subject"),
            SessionLifetime::from_seconds(3_600).expect("lifetime"),
        )
        .await
        .expect("session");
    fixture
        .store
        .create_course(
            fixture.context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: ordinary_course,
                    tenant: fixture.tenant,
                    title: "Ordinary inspection".into(),
                    term: fixture.term.clone(),
                },
                authority: CourseCreationAuthority::ApprovedInstructor {
                    actor: fixture.actor,
                    session: ordinary_session,
                },
            },
        )
        .await
        .expect("ordinary course");
    let ordinary_reference = fixture.store.read_state().expect("state").course_references
        [&(fixture.tenant, ordinary_course)];
    let ordinary_preview = fixture
        .store
        .preview_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationPreviewRequest {
                source: fixture.blueprint,
                course: ordinary_reference,
                target_term: fixture.term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("ordinary preview");
    fixture
        .store
        .apply_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationCommand::from_preview(&ordinary_preview, key("inspect-ordinary"))
                .expect("ordinary command"),
        )
        .await
        .expect("ordinary apply");
    let ordinary_view = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.session, ordinary_reference)
        .await
        .expect("ordinary inspection")
        .expect("ordinary import");
    assert!(matches!(
        ordinary_view.origin,
        CurriculumCourseImportOriginView::Ordinary
    ));
    assert!(matches!(
        ordinary_view.assignments.as_slice(),
        [import] if matches!(
            import.source,
            CurriculumAssignmentImportSourceView::Reusable {
                definition: AssignmentDefinitionSourceView::Blueprint(source),
            } if source == fixture.blueprint
        )
    ));

    let source_witness = witness(&fixture, alpha.course);
    let rollover_term = CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago")
        .expect("rollover term");
    let rollover_preview = fixture
        .store
        .preview_course_rollover(
            fixture.context,
            fixture.session,
            CourseRolloverPreviewRequest {
                witness: source_witness.clone(),
                title: CurriculumAdoptionTitle::parse("Rollover inspection").expect("title"),
                target_term: rollover_term.clone(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("rollover preview");
    let rollover = fixture
        .store
        .apply_course_rollover(
            fixture.context,
            fixture.session,
            CourseRolloverCommand::from_preview(&rollover_preview, key("inspect-rollover"))
                .expect("rollover command"),
        )
        .await
        .expect("rollover apply");
    let later_rollover_preview = fixture
        .store
        .preview_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationPreviewRequest {
                source: fixture.blueprint,
                course: rollover.course,
                target_term: rollover_term,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("later rollover preview");
    fixture
        .store
        .apply_blueprint_instantiation(
            fixture.context,
            fixture.session,
            BlueprintInstantiationCommand::from_preview(
                &later_rollover_preview,
                key("inspect-rollover-later"),
            )
            .expect("later rollover command"),
        )
        .await
        .expect("later rollover apply");
    let rollover_view = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.session, rollover.course)
        .await
        .expect("rollover inspection")
        .expect("rollover import");
    let CurriculumCourseImportOriginView::Rollover { source } = rollover_view.origin else {
        panic!("rollover import has a closed rollover origin");
    };
    assert_eq!(source.source_schedule, source_witness);
    assert!(rollover_view.assignments.iter().any(|import| matches!(
        &import.source,
        CurriculumAssignmentImportSourceView::Rollover { source }
            if source_witness.contains_assignment(source.assignment())
    )));
    assert!(rollover_view.assignments.iter().any(|import| matches!(
        import.source,
        CurriculumAssignmentImportSourceView::Reusable {
            definition: AssignmentDefinitionSourceView::Blueprint(source),
        } if source == fixture.blueprint
    )));
}

#[tokio::test]
async fn inspection_witness_includes_current_teaching_outside_the_import_subset() {
    let fixture = AdoptionScenario::new().await;
    let alpha = fixture.instantiate("complete-witness").await;
    let extra_id = AssignmentId::from_uuid(Uuid::from_u128(121_051));
    let extra_reference = AssignmentReference::new(121_051).expect("assignment reference");
    let extra_revision = AssignmentRevision::new(7).expect("assignment revision");
    {
        let mut state = fixture.store.write_state().expect("state");
        let course = resolve_course(&state, fixture.tenant, alpha.course).expect("course");
        let imported = course_assignment_ids(&state, fixture.tenant, course)[0];
        let mut extra = state.assignments[&(fixture.tenant, imported)].clone();
        extra.id = extra_id;
        extra.title = "Current non-imported teaching".into();
        state.assignments.insert((fixture.tenant, extra_id), extra);
        state
            .assignment_revisions
            .insert((fixture.tenant, extra_id), extra_revision);
        state
            .assignment_references
            .insert((fixture.tenant, extra_id), extra_reference);
        state
            .assignments_by_reference
            .insert((fixture.tenant, extra_reference), extra_id);
    }

    let inspection = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.session, alpha.course)
        .await
        .expect("inspection")
        .expect("import");
    assert!(
        inspection
            .witness
            .assignment_revisions()
            .iter()
            .any(|observed| observed.assignment == extra_reference
                && observed.revision == extra_revision)
    );
    assert!(
        inspection
            .assignments
            .iter()
            .all(|import| import.assignment != extra_reference)
    );
}

fn substitute_first_assignment(course: &CurriculumSemanticCourse) -> CurriculumSemanticCourse {
    let modules = course
        .modules()
        .iter()
        .enumerate()
        .map(|(module_index, module)| {
            let assignments = module
                .assignments()
                .iter()
                .enumerate()
                .map(|(assignment_index, assignment)| {
                    let title = if module_index == 0 && assignment_index == 0 {
                        "Substituted semantic assignment".to_owned()
                    } else {
                        assignment.title().to_owned()
                    };
                    CurriculumSemanticAssignment::new(
                        title,
                        assignment.instructions().clone(),
                        assignment.entries().to_vec(),
                        assignment.defaults().clone(),
                        assignment.schedule().clone(),
                    )
                    .expect("shape-preserving semantic assignment")
                })
                .collect();
            CurriculumSemanticModule::new(module.label().to_owned(), assignments)
                .expect("shape-preserving semantic module")
        })
        .collect();
    CurriculumSemanticCourse::new(course.title().to_owned(), modules)
        .expect("shape-preserving semantic course")
}

fn witness(
    fixture: &AdoptionScenario,
    course: question_model::CourseReference,
) -> CourseScheduleWitness {
    let state = fixture.store.read_state().expect("state");
    let course = resolve_course(&state, fixture.tenant, course).expect("course");
    course_witness(&state, fixture.tenant, course).expect("witness")
}
