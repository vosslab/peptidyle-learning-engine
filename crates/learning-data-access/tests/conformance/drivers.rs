use super::*;

#[tokio::test]
async fn memory_store_conforms() {
    let store = MemoryStore::default();
    exercise_store(&store).await;
    exercise_navigation_reference_authority(&store).await;
    exercise_durable_publication_assignment_contract(&store).await;
    exercise_course_pagination_scale(&store).await;
    group_store_memory::exercise_course_group_management_contract(&store).await;
    co_instructor_memory::exercise_co_instructor_authority_contract(&store).await;
    co_instructor_memory::exercise_memory_co_instructor_expiry(&store).await;
}

#[tokio::test]
async fn memory_run_api_store_conforms() {
    for (fixture_offset, disclosure_policy) in [
        (
            0,
            StudentDisclosurePolicy {
                score: StudentDisclosureTiming::DuringAttempt,
                per_item_correctness: StudentDisclosureTiming::Never,
                feedback_text: StudentDisclosureTiming::AfterSubmit,
                solution: StudentDisclosureTiming::AfterClose,
                class_statistics: StudentDisclosureTiming::Never,
            },
        ),
        (
            10_000,
            StudentDisclosurePolicy {
                score: StudentDisclosureTiming::AfterDue,
                per_item_correctness: StudentDisclosureTiming::AfterSubmit,
                feedback_text: StudentDisclosureTiming::DuringAttempt,
                solution: StudentDisclosureTiming::AfterSubmit,
                class_statistics: StudentDisclosureTiming::Never,
            },
        ),
    ] {
        let store = MemoryStore::default();
        let sealed_private_execution = store.sealed_private_execution_store();
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
            .expect("memory clock");
        exercise_run_api_store(
            &store,
            &sealed_private_execution,
            disclosure_policy,
            fixture_offset,
        )
        .await;
    }
}
