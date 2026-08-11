use super::*;

#[tokio::test]
async fn memory_store_conforms() {
    let store = MemoryStore::default();
    exercise_store(&store).await;
    exercise_assignment_cas(&store).await;
    exercise_course_pagination_scale(&store).await;
    exercise_publication_identity_boundary(&store).await;
    exercise_source_artifact_binding(&store).await;
}

#[tokio::test]
async fn memory_run_api_store_conforms() {
    for disclosure in [
        FeedbackDisclosure::ImmediateFull,
        FeedbackDisclosure::ImmediateCorrectness,
        FeedbackDisclosure::Deferred,
        FeedbackDisclosure::OnRelease,
    ] {
        let store = MemoryStore::default();
        store
            .set_authoritative_time(ActivityTimestamp::from_unix_millis(500))
            .expect("memory clock");
        exercise_run_api_store(&store, disclosure).await;
    }
}
