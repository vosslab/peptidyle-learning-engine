use super::*;

#[tokio::test]
async fn memory_store_conforms() {
    let store = MemoryStore::default();
    exercise_store(&store).await;
    exercise_navigation_reference_authority(&store).await;
    exercise_durable_publication_assignment_contract(&store).await;
    exercise_course_pagination_scale(&store).await;
}

#[tokio::test]
async fn memory_run_api_store_conforms() {
    for (fixture_offset, disclosure_policy) in [
        (
            0,
            LearnerDisclosurePolicy {
                score: LearnerDisclosureTiming::DuringAttempt,
                per_item_correctness: LearnerDisclosureTiming::Never,
                feedback_text: LearnerDisclosureTiming::AfterSubmit,
                solution: LearnerDisclosureTiming::AfterClose,
                class_statistics: LearnerDisclosureTiming::Never,
            },
        ),
        (
            10_000,
            LearnerDisclosurePolicy {
                score: LearnerDisclosureTiming::AfterDue,
                per_item_correctness: LearnerDisclosureTiming::AfterSubmit,
                feedback_text: LearnerDisclosureTiming::DuringAttempt,
                solution: LearnerDisclosureTiming::AfterSubmit,
                class_statistics: LearnerDisclosureTiming::Never,
            },
        ),
    ] {
        let store = MemoryStore::default();
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
            .expect("memory clock");
        exercise_run_api_store(&store, disclosure_policy, fixture_offset).await;
    }
}
