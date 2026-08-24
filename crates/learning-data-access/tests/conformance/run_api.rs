use super::*;

pub(super) async fn exercise_run_api_store<S>(
    store: &S,
    disclosure_policy: LearnerDisclosurePolicy,
    fixture_offset: u128,
) where
    S: Store
        + CatalogStore
        + CourseRosterStore
        + JobStore
        + AssignmentScoringWorkerStore
        + SessionStore,
{
    let fixture = exercise_run_api_receipts(store, disclosure_policy, fixture_offset).await;
    let current = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment edit read")
        .expect("run assignment exists");
    let withheld = store
        .replace_assignment(
            fixture.context,
            ReplaceAssignmentCommand {
                actor: fixture.publisher,
                course: fixture.course,
                assignment: fixture.assignment,
                expected_revision: current.revision,
                update: AssignmentUpdate {
                    title: current.record.title.clone(),
                    audience: current.record.audience.clone(),
                    items: current.record.items.clone(),
                    selection_groups: current.record.selection_groups.clone(),
                    disclosure_policy: LearnerDisclosurePolicy {
                        score: LearnerDisclosureTiming::Never,
                        per_item_correctness: LearnerDisclosureTiming::Never,
                        feedback_text: LearnerDisclosureTiming::Never,
                        solution: LearnerDisclosureTiming::Never,
                        class_statistics: LearnerDisclosureTiming::Never,
                    },
                    policies: current.record.policies,
                },
            },
        )
        .await
        .expect("assignment policy change");
    let changed = store
        .get_run_summary_page(
            fixture.context,
            fixture.student_user,
            fixture.run.id,
            PageRequest::first(PageSize::new(10).expect("valid bounded page")),
        )
        .await
        .expect("summary uses current assignment disclosure policy");
    assert!(
        !changed.outcomes.items[0].disclosure.decision().score
            && !changed.outcomes.items[0]
                .disclosure
                .decision()
                .per_item_correctness
            && !changed.outcomes.items[0]
                .disclosure
                .decision()
                .feedback_text
            && !changed.outcomes.items[0].disclosure.decision().solution
            && !changed.outcomes.items[0]
                .disclosure
                .decision()
                .class_statistics,
        "a current assignment policy change reprojects every learner field; old question policy and release audits cannot preserve disclosure"
    );
    assert_eq!(
        withheld.record.disclosure_policy.score,
        LearnerDisclosureTiming::Never
    );
    exercise_run_rescoring(store, &fixture).await;
    exercise_delete_and_regrade(store, &fixture).await;
    exercise_attempt_support(store, &fixture).await;
    exercise_run_summary_scale(store, &fixture).await;
}
