//! Derived-subject provenance conformance at the rehearsal Store boundary.

use super::*;
use learning_data_access::{PageRequest, PageSize, PreviewPlaneStore};

#[tokio::test]
async fn fresh_derived_preview_authorizes_one_identity_free_rehearsal_without_a_second_audit() {
    let store = MemoryStore::default();
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract(&store).await;
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment lookup")
        .expect("assignment reference");
    let revision = TeachingOperationRevision::new(fixture.assignment_revision.value())
        .expect("teaching revision");
    let schedule = store
        .list_instructor_preview_schedule(
            fixture.context,
            fixture.instructor,
            fixture.course,
            assignment,
            revision,
            PageRequest::first(PageSize::new(1).expect("page size")),
        )
        .await
        .expect("schedule");
    let membership = match schedule.rows.first().expect("schedule row") {
        question_model::InstructorPreviewScheduleRow::Granted { membership, .. } => *membership,
        question_model::InstructorPreviewScheduleRow::Denied { .. } => {
            panic!("fixture supplies an entitled learner")
        }
    };
    let selected_moment = PreviewSelectedMoment {
        value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("time"),
        time_zone: IanaTimeZone::parse("America/Chicago").expect("zone"),
    };
    let derived = store
        .construct_derived_preview(
            fixture.context,
            fixture.instructor,
            fixture.course,
            question_model::DerivedPreviewSubjectRequest {
                assignment,
                revision,
                selected_moment,
                membership,
            },
        )
        .await
        .expect("derived preview");
    let question_model::PreviewEvaluation::Allowed { subject, .. } = derived.evaluation else {
        panic!("fixture allows derived preview");
    };
    let audits_before_start = store.preview_subject_audits().expect("audit seam");
    let before = store
        .rehearsal_state_effect_fingerprint()
        .expect("baseline");
    let receipt = store
        .start_rehearsal(
            fixture.context,
            StartRehearsalCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment,
                revision,
                subject: RehearsalSubjectStart::Derived { candidate: subject },
                start_new_after_completion: false,
            },
        )
        .await
        .expect("fresh audited derived candidate starts rehearsal");
    assert_eq!(receipt.lifecycle, RehearsalLifecycle::Active);
    assert_eq!(
        store.preview_subject_audits().expect("audit seam"),
        audits_before_start,
        "rehearsal re-resolves provenance without a second learner audit"
    );
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after start")
            .has_only_rehearsal_effects_from(&before)
    );
}
