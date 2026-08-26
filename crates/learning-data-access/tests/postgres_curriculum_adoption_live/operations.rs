use learning_data_access::{
    CurriculumAdoptionStore, PageRequest, PageSize, ReplaceAlphaCourseCommand,
    ReusableCurriculumStore, StoreError,
};
use question_model::{
    AlphaCourseDefinitionInput, AlphaCourseModuleInput, AlphaInstantiationCommand,
    AlphaInstantiationPreviewRequest, BlueprintInstantiationCommand,
    BlueprintInstantiationPreviewRequest, CourseReference, CourseScheduleWitness, CourseTerm,
    CourseTermShiftCommand, CourseTermShiftPreviewRequest, CurriculumAdoptionIdempotencyKey,
    CurriculumAdoptionTitle, CurriculumPinReplacements, ForkAlphaCommand, ForkAlphaPreviewRequest,
};
use sqlx::Row;

use super::fixture::{AdoptionFixture, definition};

pub(super) async fn assert_public_source_and_destination_write(fixture: &AdoptionFixture) {
    let readable = fixture
        .store
        .list_alpha_courses(
            fixture.foreign_context,
            fixture.foreign_instructor_session,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("approved Instructor can read a public Alpha across tenants");
    assert!(
        readable
            .items
            .iter()
            .any(|item| item.reference == fixture.alpha.reference),
        "public Alpha is visible to the destination Instructor"
    );

    let fork_preview = fixture
        .store
        .preview_fork_alpha(
            fixture.foreign_context,
            fixture.foreign_instructor_session,
            ForkAlphaPreviewRequest {
                source: fixture.alpha,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("cross-tenant public Alpha fork preview");
    let forked = fixture
        .store
        .apply_fork_alpha(
            fixture.foreign_context,
            fixture.foreign_instructor_session,
            ForkAlphaCommand::from_preview(
                &fork_preview,
                question_model::CurriculumAdoptionIdempotencyKey::parse("b2-live-fork")
                    .expect("fixture key"),
            )
            .expect("corrected fork preview"),
        )
        .await
        .expect("cross-tenant public Alpha fork");
    assert_eq!(forked.source, fixture.alpha);
    assert_ne!(
        forked.alpha, fixture.alpha.reference,
        "fork receives a distinct destination Alpha reference"
    );
    let fork_detail = fixture
        .store
        .get_alpha_course(
            fixture.foreign_context,
            fixture.foreign_instructor_session,
            forked.alpha,
        )
        .await
        .expect("fork detail")
        .expect("fork remains readable by its creator");
    let unchanged_fork = fixture
        .store
        .replace_alpha_course(
            fixture.foreign_context,
            fixture.foreign_instructor_session,
            ReplaceAlphaCourseCommand {
                reference: Some(forked.alpha),
                expected_revision: Some(fork_detail.revision),
                definition: AlphaCourseDefinitionInput {
                    title: "B2 public Alpha".into(),
                    modules: vec![AlphaCourseModuleInput {
                        label: "B2 module".into(),
                        definitions: vec![
                            definition(
                                fixture.public_question.clone(),
                                "B2 reusable Alpha assignment",
                            )
                            .definition,
                        ],
                    }],
                },
            },
        )
        .await
        .expect("unchanged B1 Alpha save after B2 fork");
    assert_eq!(
        unchanged_fork.revision, fork_detail.revision,
        "the B1 Alpha digest preserves an unchanged fork as a no-op"
    );

    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.foreign_context,
            fixture.foreign_instructor_session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse("B2 destination course")
                    .expect("fixture title"),
                target_term: question_model::CourseTerm::from_parts(
                    "2026-08-24",
                    "2026-12-18",
                    "America/Chicago",
                )
                .expect("fixture term"),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("destination Instructor can prepare a public Alpha instantiation");
    let completed = fixture
        .store
        .apply_alpha_instantiation(
            fixture.foreign_context,
            fixture.foreign_instructor_session,
            AlphaInstantiationCommand::from_preview(
                &preview,
                question_model::CurriculumAdoptionIdempotencyKey::parse("b2-live-alpha")
                    .expect("fixture key"),
            )
            .expect("corrected Alpha preview"),
        )
        .await
        .expect("destination Instructor creates an ordinary destination course");
    assert_eq!(completed.source, fixture.alpha);
    assert_eq!(
        fixture
            .store
            .preview_blueprint_instantiation(
                fixture.context,
                fixture.learner_session,
                question_model::BlueprintInstantiationPreviewRequest {
                    source: fixture.blueprint,
                    course: completed.course,
                    target_term: preview.target_term,
                    replacements: CurriculumPinReplacements::default(),
                },
            )
            .await,
        Err(StoreError::Forbidden),
        "the source tenant learner cannot apply destination-course authority"
    );
}

/// Proves the PostgreSQL broker persists a Blueprint import once, rejects a
/// same-key different destination, rolls back stale/invalid requests, and
/// reloads an answer-free durable inspection through a fresh Store facade.
pub(super) async fn assert_blueprint_replay_refusals_and_reload(fixture: &AdoptionFixture) {
    let first_course = instantiate_source_alpha(fixture, "b2-live-source-course").await;
    let preview = blueprint_preview(fixture, first_course).await;
    let command = BlueprintInstantiationCommand::from_preview(&preview, key("b2-live-blueprint"))
        .expect("Blueprint command");
    let applied = fixture
        .store
        .apply_blueprint_instantiation(fixture.context, fixture.instructor_session, command.clone())
        .await
        .expect("Blueprint apply");
    let replay = fixture
        .store
        .apply_blueprint_instantiation(fixture.context, fixture.instructor_session, command)
        .await
        .expect("Blueprint replay");
    assert_eq!(replay.assignment, applied.assignment);
    assert_eq!(replay.receipt, applied.receipt);
    assert_eq!(
        receipt_operation(fixture, "b2-live-blueprint")
            .await
            .as_deref(),
        Some("blueprintInstantiation"),
        "the broker retains one immutable Blueprint receipt"
    );

    let second_course = instantiate_source_alpha(fixture, "b2-live-collision-course").await;
    let collision = BlueprintInstantiationCommand::from_preview(
        &blueprint_preview(fixture, second_course).await,
        key("b2-live-blueprint"),
    )
    .expect("collision command");
    assert_eq!(
        fixture
            .store
            .apply_blueprint_instantiation(fixture.context, fixture.instructor_session, collision)
            .await,
        Err(StoreError::Conflict),
        "a completed key cannot select another destination"
    );

    let shift_preview = fixture
        .store
        .preview_course_term_shift(
            fixture.context,
            fixture.instructor_session,
            CourseTermShiftPreviewRequest {
                witness: inspection_witness(
                    fixture
                        .store
                        .inspect_curriculum_imports(
                            fixture.context,
                            fixture.instructor_session,
                            first_course,
                        )
                        .await
                        .expect("inspection")
                        .expect("course import"),
                ),
                target_term: spring_term(),
            },
        )
        .await
        .expect("term shift preview");
    fixture
        .store
        .apply_blueprint_instantiation(
            fixture.context,
            fixture.instructor_session,
            BlueprintInstantiationCommand::from_preview(
                &blueprint_preview(fixture, first_course).await,
                key("b2-live-stale-writer"),
            )
            .expect("writer command"),
        )
        .await
        .expect("writer advances the schedule witness");
    assert_eq!(
        fixture
            .store
            .apply_course_term_shift(
                fixture.context,
                fixture.instructor_session,
                CourseTermShiftCommand::from_preview(&shift_preview, key("b2-live-stale-shift"))
                    .expect("stale shift command"),
            )
            .await,
        Err(StoreError::Conflict),
        "stale witness creates neither a completion nor a partial term update"
    );
    assert_eq!(
        receipt_operation(fixture, "b2-live-stale-shift").await,
        None
    );

    assert_eq!(
        fixture
            .store
            .preview_blueprint_instantiation(
                fixture.context,
                fixture.instructor_session,
                BlueprintInstantiationPreviewRequest {
                    source: fixture.blueprint,
                    course: CourseReference::new(9_999_999).expect("unknown reference"),
                    target_term: fall_term(),
                    replacements: CurriculumPinReplacements::default(),
                },
            )
            .await,
        Err(StoreError::NotFound),
        "an invalid destination locator is refused before a B2 write"
    );

    let reloaded = fixture.reloaded_store();
    let inspection = reloaded
        .inspect_curriculum_imports(fixture.context, fixture.instructor_session, first_course)
        .await
        .expect("fresh Store inspection")
        .expect("durable import inspection");
    assert!(inspection.assignments.iter().any(|import| {
        import.assignment == applied.assignment
            && matches!(
                import.source,
                question_model::curriculum_adoption::CurriculumAssignmentImportSourceView::Reusable {
                    definition: question_model::AssignmentDefinitionSourceView::Blueprint(source),
                } if source == fixture.blueprint
            )
    }));
}

async fn instantiate_source_alpha(fixture: &AdoptionFixture, receipt_key: &str) -> CourseReference {
    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.instructor_session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse(receipt_key).expect("fixture title"),
                target_term: fall_term(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("source Alpha preview");
    fixture
        .store
        .apply_alpha_instantiation(
            fixture.context,
            fixture.instructor_session,
            AlphaInstantiationCommand::from_preview(&preview, key(receipt_key))
                .expect("source Alpha command"),
        )
        .await
        .expect("source Alpha apply")
        .course
}

async fn blueprint_preview(
    fixture: &AdoptionFixture,
    course: CourseReference,
) -> question_model::BlueprintInstantiationPreviewView {
    fixture
        .store
        .preview_blueprint_instantiation(
            fixture.context,
            fixture.instructor_session,
            BlueprintInstantiationPreviewRequest {
                source: fixture.blueprint,
                course,
                target_term: fall_term(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("Blueprint preview")
}

async fn receipt_operation(fixture: &AdoptionFixture, key: &str) -> Option<String> {
    sqlx::query(
        "SELECT operation FROM public.curriculum_adoption_receipt \
         WHERE tenant_id = $1 AND idempotency_key = $2",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(key)
    .fetch_optional(&fixture.pool)
    .await
    .expect("receipt observation")
    .map(|row| row.try_get("operation").expect("receipt operation"))
}

fn key(value: &str) -> CurriculumAdoptionIdempotencyKey {
    CurriculumAdoptionIdempotencyKey::parse(value).expect("fixture key")
}

fn fall_term() -> CourseTerm {
    CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago").expect("fall term")
}

fn spring_term() -> CourseTerm {
    CourseTerm::from_parts("2027-01-11", "2027-05-08", "America/Chicago").expect("spring term")
}

fn inspection_witness(
    inspection: question_model::CurriculumCourseImportView,
) -> CourseScheduleWitness {
    inspection.witness
}
