//! Connected ordinary-course lifecycle relationships for B2.

use learning_data_access::CurriculumAdoptionStore;
use question_model::{
    AlphaInstantiationCommand, AlphaInstantiationPreviewRequest, CourseRolloverCommand,
    CourseRolloverPreviewRequest, CourseTerm, CourseTermShiftCommand,
    CourseTermShiftPreviewOutcome, CourseTermShiftPreviewRequest, CurriculumAdoptionIdempotencyKey,
    CurriculumAdoptionTitle, CurriculumPinReplacements, CurriculumReplayStatus,
};
use sqlx::Row;

use super::fixture::AdoptionFixture;

/// Applies a normal Alpha instantiation, then proves that the Store carries
/// source order into a rollover, leaves learner-linked state empty, and shifts
/// an unissued course atomically through its returned witness.
pub(super) async fn assert_rollover_and_unissued_term_shift(fixture: &AdoptionFixture) {
    let source = instantiate(fixture, "b2-live-lifecycle-source", fall_term()).await;
    let source_inspection = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.instructor_session, source)
        .await
        .expect("source inspection")
        .expect("Alpha instantiation records imports");

    let rollover_preview = fixture
        .store
        .preview_course_rollover(
            fixture.context,
            fixture.instructor_session,
            CourseRolloverPreviewRequest {
                witness: source_inspection.witness.clone(),
                title: CurriculumAdoptionTitle::parse("B2 lifecycle rollover")
                    .expect("fixture title"),
                target_term: spring_term(),
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("rollover preview");
    let rollover_command =
        CourseRolloverCommand::from_preview(&rollover_preview, key("b2-live-rollover"))
            .expect("corrected rollover preview");
    let rollover = fixture
        .store
        .apply_course_rollover(
            fixture.context,
            fixture.instructor_session,
            rollover_command.clone(),
        )
        .await
        .expect("rollover apply");
    let replay = fixture
        .store
        .apply_course_rollover(
            fixture.context,
            fixture.instructor_session,
            rollover_command,
        )
        .await
        .expect("rollover replay");
    assert_eq!(replay.course, rollover.course);
    assert_eq!(replay.replay, CurriculumReplayStatus::Replayed);

    let destination_inspection = fixture
        .store
        .inspect_curriculum_imports(fixture.context, fixture.instructor_session, rollover.course)
        .await
        .expect("rollover inspection")
        .expect("rollover retains import provenance");
    assert_eq!(destination_inspection.term, spring_term());
    assert_eq!(
        destination_inspection
            .assignments
            .iter()
            .map(|entry| entry.assignment)
            .collect::<Vec<_>>()
            .len(),
        source_inspection.assignments.len(),
        "rollover preserves each imported assignment in its source-derived order"
    );
    assert_rollover_has_no_learner_state(fixture, rollover.course).await;

    let shift_preview = fixture
        .store
        .preview_course_term_shift(
            fixture.context,
            fixture.instructor_session,
            CourseTermShiftPreviewRequest {
                witness: destination_inspection.witness,
                target_term: summer_term(),
            },
        )
        .await
        .expect("unissued course term-shift preview");
    let command = CourseTermShiftCommand::from_preview(&shift_preview, key("b2-live-term-shift"))
        .expect("unissued course returns an eligible shift command");
    let shifted = fixture
        .store
        .apply_course_term_shift(fixture.context, fixture.instructor_session, command)
        .await
        .expect("unissued course term shift");
    assert_eq!(shifted.course, rollover.course);
    assert_eq!(shifted.term, summer_term());
    assert!(matches!(
        shift_preview,
        CourseTermShiftPreviewOutcome::Eligible { .. }
    ));
}

async fn instantiate(
    fixture: &AdoptionFixture,
    receipt_key: &str,
    term: CourseTerm,
) -> question_model::CourseReference {
    let preview = fixture
        .store
        .preview_alpha_instantiation(
            fixture.context,
            fixture.instructor_session,
            AlphaInstantiationPreviewRequest {
                source: fixture.alpha,
                title: CurriculumAdoptionTitle::parse(receipt_key).expect("fixture title"),
                target_term: term,
                replacements: CurriculumPinReplacements::default(),
            },
        )
        .await
        .expect("Alpha preview");
    fixture
        .store
        .apply_alpha_instantiation(
            fixture.context,
            fixture.instructor_session,
            AlphaInstantiationCommand::from_preview(&preview, key(receipt_key))
                .expect("Alpha command"),
        )
        .await
        .expect("Alpha instantiation")
        .course
}

async fn assert_rollover_has_no_learner_state(
    fixture: &AdoptionFixture,
    course: question_model::CourseReference,
) {
    let course_id: uuid::Uuid =
        sqlx::query("SELECT course_id FROM public.course WHERE tenant_id = $1 AND public_id = $2")
            .bind(fixture.tenant.as_uuid())
            .bind(i64::from(course.number()))
            .fetch_one(&fixture.pool)
            .await
            .expect("resolve rollover course identity")
            .try_get("course_id")
            .expect("course identity");
    let learner_state_queries = [
        (
            "student membership",
            "SELECT count(*) FROM public.course_member \
             WHERE tenant_id = $1 AND course_id = $2 AND role = 'student'",
        ),
        (
            "enrollment",
            "SELECT count(*) FROM public.enrollment AS enrollment \
             JOIN public.assignment AS assignment \
               ON assignment.tenant_id = enrollment.tenant_id \
              AND assignment.assignment_id = enrollment.assignment_id \
             WHERE enrollment.tenant_id = $1 AND assignment.course_id = $2",
        ),
        (
            "issued run",
            "SELECT count(*) FROM public.assignment_run AS run \
             JOIN public.enrollment AS enrollment \
               ON enrollment.tenant_id = run.tenant_id \
              AND enrollment.enrollment_id = run.enrollment_id \
             JOIN public.assignment AS assignment \
               ON assignment.tenant_id = enrollment.tenant_id \
              AND assignment.assignment_id = enrollment.assignment_id \
             WHERE run.tenant_id = $1 AND assignment.course_id = $2",
        ),
    ];
    for (name, query) in learner_state_queries {
        let count: i64 = sqlx::query_scalar(query)
            .bind(fixture.tenant.as_uuid())
            .bind(course_id)
            .fetch_one(&fixture.pool)
            .await
            .expect("rollover learner-state observation");
        assert_eq!(count, 0, "rollover destination has no {name}");
    }
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

fn summer_term() -> CourseTerm {
    CourseTerm::from_parts("2027-05-17", "2027-08-06", "America/Chicago").expect("summer term")
}
