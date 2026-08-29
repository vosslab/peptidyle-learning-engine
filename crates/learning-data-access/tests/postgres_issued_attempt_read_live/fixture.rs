//! Exact issued-question evidence used by the PostgreSQL route-read oracle.

use super::*;

pub(super) struct IssueFixture {
    pub(super) context: TenantContext,
    pub(super) student: UserId,
    pub(super) course: CourseId,
    pub(super) assignment: AssignmentId,
    pub(super) run: RunId,
    pub(super) reference: ProblemVersionRef,
}

impl IssueFixture {
    pub(super) async fn issue(
        &self,
        store: &PostgresStore,
        position: u32,
        seed: u64,
    ) -> QuestionAttemptId {
        let (binding, presentation_snapshot) = presentation(self.reference, seed);
        let attempt = QuestionAttemptId::from_uuid(id());
        store
            .issue_or_resume_question_attempt(
                self.context,
                IssueQuestionAttemptCommand {
                    actor: self.student,
                    binding: StudentWorkRoutingBinding::new(self.course, self.assignment),
                    attempt,
                    run: self.run,
                    assignment_position: position,
                    problem: self.reference.problem,
                    question_version: self.reference.version,
                    issued_question_snapshot: issued_snapshot(
                        store,
                        self.context,
                        self.reference,
                        IssuedQuestionFamilyWitnessV1::Native {
                            physical_asset_bindings: Vec::new(),
                        },
                    )
                    .await,
                    seed,
                    presentation_capability: PresentationCapability::EnvelopeV1,
                    presentation: Some(binding),
                    presentation_snapshot: Some(presentation_snapshot),
                    grading_envelope: Some(envelope(self.reference, seed)),
                    native_execution_envelope_capability:
                        NativeExecutionEnvelopeCapability::Required,
                    flat_grading: None,
                    flat_grading_capability: FlatGradingCapability::NotApplicable,
                    webwork_grading: None,
                    webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                    qti_grading: None,
                    qti_grading_capability: QtiGradingCapability::NotApplicable,
                    parameter_hash: format!("issued-read-parameters-{seed}"),
                    provenance: provenance(),
                    webwork_replay: None,
                    prefetched: None,
                    predecessor_submission: None,
                },
            )
            .await
            .expect("issue ordinary native attempt");
        attempt
    }
}

pub(super) async fn issue_webwork(
    fixture: &IssueFixture,
    store: &PostgresStore,
    contract: IssuedWebworkGradingContract,
    provenance: AttemptProvenance,
) -> QuestionAttemptId {
    let (binding, presentation_snapshot) = presentation(fixture.reference, 91);
    let attempt = QuestionAttemptId::from_uuid(id());
    store
        .issue_or_resume_question_attempt(
            fixture.context,
            IssueQuestionAttemptCommand {
                actor: fixture.student,
                binding: StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
                attempt,
                run: fixture.run,
                assignment_position: 0,
                problem: fixture.reference.problem,
                question_version: fixture.reference.version,
                issued_question_snapshot: issued_snapshot(
                    store,
                    fixture.context,
                    fixture.reference,
                    IssuedQuestionFamilyWitnessV1::Webwork {},
                )
                .await,
                seed: 91,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(binding),
                presentation_snapshot: Some(presentation_snapshot),
                grading_envelope: Some(envelope(fixture.reference, 91)),
                native_execution_envelope_capability:
                    NativeExecutionEnvelopeCapability::NotApplicable,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: Some(contract),
                webwork_grading_capability: WebworkGradingCapability::Required,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                parameter_hash: "webwork-issued-read".to_string(),
                provenance,
                webwork_replay: Some(webwork_replay()),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issue WebWork attempt");
    attempt
}

async fn issued_snapshot(
    store: &PostgresStore,
    context: TenantContext,
    reference: ProblemVersionRef,
    witness: IssuedQuestionFamilyWitnessV1,
) -> IssuedQuestionSnapshotV1 {
    let question = store
        .get_catalog_problem(context, reference)
        .await
        .expect("load exact published question for issued snapshot")
        .expect("published question exists for issued snapshot")
        .question;
    IssuedQuestionSnapshotV1::new(question, witness)
        .expect("published question and issued family witness agree")
}

fn webwork_replay() -> WebworkReplayMappingV1 {
    WebworkReplayMappingV1::SingleChoice {
        items: vec![
            WebworkReplayControlV1 {
                item: RenderedItemIdV1::parse("a1b2").expect("item"),
                field: "AnSwEr0001".to_string(),
                value: "0".to_string(),
            },
            WebworkReplayControlV1 {
                item: RenderedItemIdV1::parse("c3d4").expect("item"),
                field: "AnSwEr0001".to_string(),
                value: "1".to_string(),
            },
        ],
    }
}
