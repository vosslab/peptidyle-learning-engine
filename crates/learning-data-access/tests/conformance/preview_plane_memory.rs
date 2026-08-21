//! Public MemoryStore conformance for the identity-free T3 preview plane.

use super::*;
use learning_data_access::{NavigationReferenceStore, PreviewPlaneStore};
use question_model::{
    CourseLocalDateTime, IanaTimeZone, PolicyModificationModeView, PolicyPatchView,
    PreviewEvaluation, PreviewSelectedMoment, PreviewSyntheticGroupReferences,
    SyntheticPreviewModifiers, SyntheticPreviewSubjectRequest, TeachingAttemptLimitFieldPatch,
    TeachingLimitFieldPatch, TeachingTimeFieldPatch,
};

#[path = "preview_plane_memory/matrix.rs"]
mod matrix;

pub(crate) async fn exercise_preview_plane_memory_contract(store: &MemoryStore) {
    let fixture =
        effective_policy::exercise_effective_policy_gate_and_materialization_contract(store).await;
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment reference lookup")
        .expect("instructor assignment reference");
    let revision =
        question_model::TeachingOperationRevision::new(fixture.assignment_revision.value())
            .expect("teaching revision");
    let first_result = store
        .list_instructor_preview_schedule(
            fixture.context,
            fixture.instructor,
            fixture.course,
            assignment,
            revision,
            PageRequest::first(PageSize::new(1).expect("page size")),
        )
        .await;
    let first = first_result.expect("first instructor preview schedule page");
    let cursor = first
        .next_cursor
        .clone()
        .expect("second preview schedule page");
    let second = store
        .list_instructor_preview_schedule(
            fixture.context,
            fixture.instructor,
            fixture.course,
            assignment,
            revision,
            PageRequest::after(
                learning_data_access::Cursor::parse(cursor).expect("cursor"),
                PageSize::new(1).expect("page size"),
            ),
        )
        .await
        .expect("second instructor preview schedule page");
    assert_eq!(
        first.rows.len() + second.rows.len(),
        2,
        "stable schedule traversal"
    );
    assert!(
        second.next_cursor.is_none(),
        "schedule cursor reaches the end"
    );

    let selected_moment = PreviewSelectedMoment {
        value: CourseLocalDateTime::parse("2026-08-25T09:00:00.000").expect("local time"),
        time_zone: IanaTimeZone::parse("America/Chicago").expect("course zone"),
    };
    let modifiers = SyntheticPreviewModifiers {
        mode: PolicyModificationModeView::ExtendOnly,
        patch: PolicyPatchView {
            available_at: TeachingTimeFieldPatch::Inherit,
            due_at: TeachingTimeFieldPatch::Inherit,
            closes_at: TeachingTimeFieldPatch::Inherit,
            time_limit_seconds: TeachingLimitFieldPatch::Inherit,
            attempt_limit: TeachingAttemptLimitFieldPatch::Inherit,
        },
    };
    let before_audits = store.preview_subject_audits().expect("audit seam");
    let before_synthetic = store
        .preview_plane_state_effect_fingerprint()
        .expect("state-effect fingerprint");
    let synthetic = store
        .construct_synthetic_preview(
            fixture.context,
            fixture.instructor,
            fixture.course,
            SyntheticPreviewSubjectRequest {
                assignment,
                revision,
                selected_moment: selected_moment.clone(),
                groups: PreviewSyntheticGroupReferences::try_from(Vec::new()).expect("no groups"),
                modifiers,
            },
        )
        .await
        .expect("identity-free synthetic preview");
    assert!(matches!(
        synthetic.evaluation,
        PreviewEvaluation::Allowed { .. }
    ));
    assert_eq!(
        store.preview_subject_audits().expect("audit seam"),
        before_audits
    );
    assert!(
        store
            .preview_plane_state_effect_fingerprint()
            .expect("state-effect fingerprint")
            .is_unchanged_from(&before_synthetic),
        "synthetic construction remains identity-free and audit-free"
    );

    let membership = match &first.rows[0] {
        question_model::InstructorPreviewScheduleRow::Granted { membership, .. }
        | question_model::InstructorPreviewScheduleRow::Denied { membership, .. } => *membership,
    };
    let no_audit_on_denial = store.preview_subject_audits().expect("audit seam");
    let before_refusals = store
        .preview_plane_state_effect_fingerprint()
        .expect("state-effect fingerprint");
    assert_eq!(
        store
            .list_instructor_preview_schedule(
                fixture.context,
                UserId::from_uuid(uuid(99_999)),
                fixture.course,
                assignment,
                revision,
                PageRequest::first(PageSize::new(1).expect("page size")),
            )
            .await,
        Err(StoreError::NotFound),
        "outsider cannot inspect the schedule"
    );
    assert!(matches!(
        store
            .construct_derived_preview(
                fixture.context,
                fixture.instructor,
                fixture.course,
                question_model::DerivedPreviewSubjectRequest {
                    assignment,
                    revision: question_model::TeachingOperationRevision::new(revision.value() + 1)
                        .expect("stale revision"),
                    selected_moment: selected_moment.clone(),
                    membership,
                },
            )
            .await,
        Err(StoreError::Conflict)
    ));
    assert_eq!(
        store.preview_subject_audits().expect("audit seam"),
        no_audit_on_denial,
        "outsider and stale paths append no audit"
    );
    assert!(
        store
            .preview_plane_state_effect_fingerprint()
            .expect("state-effect fingerprint")
            .is_unchanged_from(&before_refusals),
        "authorization and stale-revision refusals preserve every Memory collection"
    );
    let before_derived = store
        .preview_plane_state_effect_fingerprint()
        .expect("state-effect fingerprint");
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
        .expect("identity-free derived preview");
    let audits = store.preview_subject_audits().expect("audit seam");
    assert_eq!(
        audits.len(),
        before_audits.len() + 1,
        "one private derived audit"
    );
    assert!(
        store
            .preview_plane_state_effect_fingerprint()
            .expect("state-effect fingerprint")
            .has_one_appended_preview_subject_audit_from(&before_derived),
        "derived construction appends only its private audit"
    );
    let PreviewEvaluation::Allowed { subject, .. } = derived.evaluation else {
        panic!("course-wide fixture permits derived preview");
    };
    let json = serde_json::to_string(&subject).expect("subject serialization");
    for forbidden in ["M-", "U-", "CI-", "PV-", "Policy learner"] {
        assert!(
            !json.contains(forbidden),
            "subject must not serialize {forbidden}"
        );
    }

    matrix::exercise_residual_memory_matrix(store, &fixture).await;
}

#[tokio::test]
async fn memory_preview_plane_conforms() {
    exercise_preview_plane_memory_contract(&MemoryStore::default()).await;
}
