use super::*;

#[tokio::test]
async fn memory_rejects_missing_or_mismatched_issued_snapshots_before_mutation() {
    let store = MemoryStore::default();
    let fixture = exercise_run_api_receipts(&store, FeedbackDisclosure::ImmediateFull).await;
    let run = store
        .start_or_resume_run(
            fixture.context,
            fixture.student_user,
            fixture.assignment,
            RunId::from_uuid(uuid(420)),
        )
        .await
        .expect("completed fixture assignment permits a fresh run");
    let (binding, snapshot) = receipt_presentation(fixture.version, 778, 12);
    let command = IssueQuestionAttemptCommand {
        actor: fixture.student_user,
        attempt: QuestionAttemptId::from_uuid(uuid(421)),
        run: run.id,
        assignment_position: 0,
        problem: fixture.problem,
        question_version: fixture.version,
        seed: 778,
        presentation_capability: PresentationCapability::EnvelopeV1,
        presentation: Some(binding),
        presentation_snapshot: None,
        grading_envelope: Some(grading_envelope(fixture.version, 778)),
        flat_grading: None,
        flat_grading_capability: FlatGradingCapability::NotApplicable,
        webwork_grading: None,
        webwork_grading_capability: learning_data_access::WebworkGradingCapability::NotApplicable,
        parameter_hash: "receipt-validation".to_string(),
        provenance: AttemptProvenance {
            adapter: implementation("native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("numeric"),
            rendered_question_sha256: "receipt-validation-render".to_string(),
        },
        webwork_replay: None,
        prefetched: None,
        predecessor_submission: None,
    };
    assert!(matches!(
        store
            .issue_or_resume_question_attempt(fixture.context, command.clone())
            .await,
        Err(StoreError::Unavailable(_))
    ));

    let mut mismatched_snapshot = snapshot;
    mismatched_snapshot.envelope.seed = question_model::generation::Seed::new(779);
    assert!(matches!(
        store
            .issue_or_resume_question_attempt(
                fixture.context,
                IssueQuestionAttemptCommand {
                    attempt: QuestionAttemptId::from_uuid(uuid(422)),
                    presentation_snapshot: Some(mismatched_snapshot),
                    ..command
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(
        store
            .list_question_attempts(
                fixture.context,
                run.id,
                PageRequest::first(PageSize::new(10).expect("valid page size")),
            )
            .await
            .expect("failed issuance leaves the run readable")
            .items
            .is_empty(),
        "invalid issued presentation state must not create an attempt"
    );
}
