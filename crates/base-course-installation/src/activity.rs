//! Deterministic ordinary learner activity for the installed Base Course.

use adapter_native::{NativeAdapter, NativeIssuedAttempt};
use learning_data_access::{
    AssignmentRecord, FlatGradingCapability, IssueQuestionAttemptCommand,
    IssuedQuestionFamilyWitnessV1, IssuedQuestionSnapshotV1, PageRequest, PageSize,
    PresentationCapability, Store, StoreError, StudentWorkRoutingBinding, SubmissionIdempotencyKey,
    TenantContext,
};
use question_model::generation::Seed;
use question_model::presentation::{
    InspectedStudentResponseV1, PresentationV1, build_presentation_v1,
    project_durable_response_to_rendered_v1, reproduce_presentation_v1,
};
use question_model::{
    AssignmentEnrollment, AttemptProvenance, AttemptResult, AttemptStatus, EnrollmentId,
    PresentationBindingV1, QuestionAttemptId, QuestionDefinition, QuestionEnvelope, RunId,
    StudentResponse, UserId,
};

use crate::records::BaseCourseIds;
use crate::{
    AcceptedSubmissionSeedExecutor, AcceptedSubmissionSeedOutcome, AcceptedSubmissionSeedRequest,
    BaseCourseInstallError, BaseCourseParticipants,
};

const COMPLETED_SEED: u64 = 17;
const ACTIVE_SEED: u64 = 23;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletedActivityState {
    NoRun,
    RunWithoutAttempt,
    IssuedAttempt,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveActivityState {
    NoRun,
    RunWithoutAttempt,
    Active,
}

struct InstalledIssuedAttempt {
    envelope: QuestionEnvelope,
    issued_question_snapshot: IssuedQuestionSnapshotV1,
    parameter_hash: String,
    provenance: AttemptProvenance,
    presentation: PresentationBindingV1,
    presentation_snapshot: learning_data_access::ReceiptPresentationSnapshot,
    submission_response: StudentResponse,
    completed_result: AttemptResult,
}

pub(crate) struct ActivityRecords<'a> {
    pub(crate) assignment: &'a AssignmentRecord,
    pub(crate) question: &'a QuestionDefinition,
    pub(crate) mary_enrollment: &'a AssignmentEnrollment,
    pub(crate) jack_enrollment: &'a AssignmentEnrollment,
}

pub(crate) async fn ensure_activity(
    store: &learning_data_access::postgres::PostgresStore,
    seed_executor: &dyn AcceptedSubmissionSeedExecutor,
    context: TenantContext,
    participants: BaseCourseParticipants,
    ids: BaseCourseIds,
    records: ActivityRecords<'_>,
) -> Result<(), BaseCourseInstallError> {
    ensure_completed_activity(
        store,
        seed_executor,
        context,
        participants.mary(),
        ids,
        &records,
    )
    .await?;
    ensure_active_activity(
        store,
        context,
        participants.jack(),
        ids,
        records.assignment,
        records.question,
        records.jack_enrollment,
    )
    .await
}

async fn ensure_completed_activity(
    store: &learning_data_access::postgres::PostgresStore,
    seed_executor: &dyn AcceptedSubmissionSeedExecutor,
    context: TenantContext,
    student: UserId,
    ids: BaseCourseIds,
    records: &ActivityRecords<'_>,
) -> Result<(), BaseCourseInstallError> {
    let assignment = records.assignment;
    let question = records.question;
    let enrollment = records.mary_enrollment;
    let issued = installed_issued_attempt(question, COMPLETED_SEED)?;
    let run = store
        .get_run(context, ids.mary_run)
        .await
        .at("checking the deterministic completed Base Course run")?;
    let attempt = store
        .get_question_attempt(context, ids.mary_attempt)
        .await
        .at("checking the deterministic completed Base Course attempt")?;
    if run.is_some() {
        validate_run_attempt_collection(
            store,
            context,
            ids.mary_run,
            attempt.as_ref(),
            ids.mary_attempt,
        )
        .await?;
    }
    let state =
        completed_activity_state(run.as_ref(), attempt.as_ref(), ids, enrollment.id, &issued)?;
    if attempt.is_some() {
        validate_persisted_issuance(
            store,
            context,
            student,
            StudentWorkRoutingBinding::new(assignment.course_id, assignment.id),
            ids.mary_attempt,
            &issued,
        )
        .await?;
    }

    match state {
        CompletedActivityState::NoRun => {
            reject_unrelated_activity(store, context, enrollment.id, "completed").await?;
            let run = store
                .start_or_resume_run(
                    context,
                    student,
                    StudentWorkRoutingBinding::new(assignment.course_id, assignment.id),
                    ids.mary_run,
                )
                .await
                .at("starting the completed Base Course run")?;
            let attempt = issue_attempt(
                store,
                context,
                student,
                ids,
                ids.mary_attempt,
                run.id,
                &issued,
            )
            .await?;
            submit_attempt(
                seed_executor,
                context,
                student,
                StudentWorkRoutingBinding::new(ids.base_course, ids.assignment),
                attempt.id,
                &issued,
            )
            .await
        }
        CompletedActivityState::RunWithoutAttempt => {
            let run = run.expect("validated completed activity has a run");
            let attempt = issue_attempt(
                store,
                context,
                student,
                ids,
                ids.mary_attempt,
                run.id,
                &issued,
            )
            .await?;
            submit_attempt(
                seed_executor,
                context,
                student,
                StudentWorkRoutingBinding::new(ids.base_course, ids.assignment),
                attempt.id,
                &issued,
            )
            .await
        }
        CompletedActivityState::IssuedAttempt => {
            let attempt = attempt.expect("validated completed activity has an attempt");
            submit_attempt(
                seed_executor,
                context,
                student,
                StudentWorkRoutingBinding::new(ids.base_course, ids.assignment),
                attempt.id,
                &issued,
            )
            .await
        }
        CompletedActivityState::Completed => Ok(()),
    }
}

async fn ensure_active_activity(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    student: UserId,
    ids: BaseCourseIds,
    assignment: &AssignmentRecord,
    question: &QuestionDefinition,
    enrollment: &AssignmentEnrollment,
) -> Result<(), BaseCourseInstallError> {
    let issued = installed_issued_attempt(question, ACTIVE_SEED)?;
    let run = store
        .get_run(context, ids.jack_run)
        .await
        .at("checking the deterministic active Base Course run")?;
    let attempt = store
        .get_question_attempt(context, ids.jack_attempt)
        .await
        .at("checking the deterministic active Base Course attempt")?;
    if run.is_some() {
        validate_run_attempt_collection(
            store,
            context,
            ids.jack_run,
            attempt.as_ref(),
            ids.jack_attempt,
        )
        .await?;
    }
    let state = active_activity_state(run.as_ref(), attempt.as_ref(), ids, enrollment.id, &issued)?;
    if attempt.is_some() {
        validate_persisted_issuance(
            store,
            context,
            student,
            StudentWorkRoutingBinding::new(assignment.course_id, assignment.id),
            ids.jack_attempt,
            &issued,
        )
        .await?;
    }

    match state {
        ActiveActivityState::NoRun => {
            reject_unrelated_activity(store, context, enrollment.id, "active").await?;
            let run = store
                .start_or_resume_run(
                    context,
                    student,
                    StudentWorkRoutingBinding::new(assignment.course_id, assignment.id),
                    ids.jack_run,
                )
                .await
                .at("starting the active Base Course run")?;
            issue_attempt(
                store,
                context,
                student,
                ids,
                ids.jack_attempt,
                run.id,
                &issued,
            )
            .await?;
            Ok(())
        }
        ActiveActivityState::RunWithoutAttempt => {
            let run = run.expect("validated active activity has a run");
            issue_attempt(
                store,
                context,
                student,
                ids,
                ids.jack_attempt,
                run.id,
                &issued,
            )
            .await?;
            Ok(())
        }
        ActiveActivityState::Active => Ok(()),
    }
}

fn completed_activity_state(
    run: Option<&question_model::AssignmentRun>,
    attempt: Option<&question_model::QuestionAttempt>,
    ids: BaseCourseIds,
    enrollment: EnrollmentId,
    issued: &InstalledIssuedAttempt,
) -> Result<CompletedActivityState, BaseCourseInstallError> {
    match (run, attempt) {
        (None, None) => Ok(CompletedActivityState::NoRun),
        (None, Some(_)) => Err(BaseCourseInstallError::baseline(
            "the completed attempt exists without its run",
        )),
        (Some(run), attempt) => {
            validate_run(run, ids.mary_run, enrollment, "completed")?;
            match attempt {
                None if run_is_open(run) => Ok(CompletedActivityState::RunWithoutAttempt),
                Some(attempt) if run_is_open(run) => {
                    validate_attempt(
                        attempt,
                        ids,
                        ids.mary_attempt,
                        ids.mary_run,
                        COMPLETED_SEED,
                        issued,
                        "completed",
                    )?;
                    if is_exact_issued_attempt(attempt) {
                        Ok(CompletedActivityState::IssuedAttempt)
                    } else {
                        Err(BaseCourseInstallError::baseline(
                            "the completed attempt is not an exact issued prefix",
                        ))
                    }
                }
                Some(attempt) if run_is_completed(run) => {
                    validate_attempt(
                        attempt,
                        ids,
                        ids.mary_attempt,
                        ids.mary_run,
                        COMPLETED_SEED,
                        issued,
                        "completed",
                    )?;
                    if is_exact_completed_attempt(attempt, issued)
                        && run.score == Some(issued.completed_result.points_earned)
                    {
                        Ok(CompletedActivityState::Completed)
                    } else {
                        Err(BaseCourseInstallError::baseline(
                            "the completed activity is not the required terminal state",
                        ))
                    }
                }
                _ => Err(BaseCourseInstallError::baseline(
                    "the completed activity has an impossible run state",
                )),
            }
        }
    }
}

fn active_activity_state(
    run: Option<&question_model::AssignmentRun>,
    attempt: Option<&question_model::QuestionAttempt>,
    ids: BaseCourseIds,
    enrollment: EnrollmentId,
    issued: &InstalledIssuedAttempt,
) -> Result<ActiveActivityState, BaseCourseInstallError> {
    match (run, attempt) {
        (None, None) => Ok(ActiveActivityState::NoRun),
        (None, Some(_)) => Err(BaseCourseInstallError::baseline(
            "the active attempt exists without its run",
        )),
        (Some(run), attempt) => {
            validate_run(run, ids.jack_run, enrollment, "active")?;
            match attempt {
                None if run_is_open(run) => Ok(ActiveActivityState::RunWithoutAttempt),
                Some(attempt) if run_is_open(run) => {
                    validate_attempt(
                        attempt,
                        ids,
                        ids.jack_attempt,
                        ids.jack_run,
                        ACTIVE_SEED,
                        issued,
                        "active",
                    )?;
                    if is_exact_issued_attempt(attempt) {
                        Ok(ActiveActivityState::Active)
                    } else {
                        Err(BaseCourseInstallError::baseline(
                            "the active attempt is not an exact open prefix",
                        ))
                    }
                }
                _ => Err(BaseCourseInstallError::baseline(
                    "the active activity is not an open run with one attempt",
                )),
            }
        }
    }
}

fn validate_run(
    run: &question_model::AssignmentRun,
    expected_id: RunId,
    enrollment: EnrollmentId,
    label: &str,
) -> Result<(), BaseCourseInstallError> {
    if run.id != expected_id
        || run.enrollment != enrollment
        || run.run_number != 1
        || run.mode != question_model::RunMode::Assigned
        || run.variation != question_model::run_policy::VariationPolicy::NewSeeds
    {
        return Err(BaseCourseInstallError::baseline(format!(
            "the {label} run has a different identity or enrollment"
        )));
    }
    Ok(())
}

fn run_is_open(run: &question_model::AssignmentRun) -> bool {
    run.completed_at.is_none() && run.score.is_none()
}

fn run_is_completed(run: &question_model::AssignmentRun) -> bool {
    run.completed_at.is_some() && run.score.is_some()
}

async fn validate_run_attempt_collection(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    run: RunId,
    expected_attempt: Option<&question_model::QuestionAttempt>,
    expected_id: QuestionAttemptId,
) -> Result<(), BaseCourseInstallError> {
    let page_size = PageSize::new(100).map_err(|error| {
        BaseCourseInstallError::baseline(format!(
            "the Base Course attempt page size is invalid: {error}"
        ))
    })?;
    let attempts = store
        .list_question_attempts(context, run, PageRequest::first(page_size))
        .await
        .at("checking the deterministic Base Course run attempts")?;
    if !has_exact_attempt_collection(
        &attempts.items,
        attempts.next_cursor.is_some(),
        expected_attempt,
        expected_id,
    ) {
        return Err(BaseCourseInstallError::baseline(
            "a seeded run does not contain exactly its deterministic attempt",
        ));
    }
    Ok(())
}

fn has_exact_attempt_collection(
    attempts: &[question_model::QuestionAttempt],
    has_more: bool,
    expected_attempt: Option<&question_model::QuestionAttempt>,
    expected_id: QuestionAttemptId,
) -> bool {
    !has_more
        && match expected_attempt {
            None => attempts.is_empty(),
            Some(expected) => {
                attempts.len() == 1 && attempts[0].id == expected_id && attempts[0] == *expected
            }
        }
}

fn validate_attempt(
    attempt: &question_model::QuestionAttempt,
    ids: BaseCourseIds,
    expected_id: QuestionAttemptId,
    expected_run: RunId,
    seed: u64,
    issued: &InstalledIssuedAttempt,
    label: &str,
) -> Result<(), BaseCourseInstallError> {
    if attempt.id != expected_id
        || attempt.run != expected_run
        || attempt.problem != ids.problem
        || attempt.question_version != ids.version
        || attempt.assignment_position != 0
        || attempt.seed != seed
        || attempt.parameter_hash != issued.parameter_hash
        || attempt.provenance != issued.provenance
        || attempt.issued_capability
            != question_model::IssuedAttemptCapabilityV1::PresentationEnvelope
    {
        return Err(BaseCourseInstallError::baseline(format!(
            "the {label} attempt differs from the deterministic recipe"
        )));
    }
    Ok(())
}

fn is_exact_issued_attempt(attempt: &question_model::QuestionAttempt) -> bool {
    attempt.status == AttemptStatus::InProgress
        && attempt.response.is_none()
        && attempt.result.is_none()
        && attempt.timer.submitted_at.is_none()
}

fn is_exact_completed_attempt(
    attempt: &question_model::QuestionAttempt,
    issued: &InstalledIssuedAttempt,
) -> bool {
    attempt.status == AttemptStatus::Submitted
        && attempt.response == Some(completed_response())
        && attempt.result == Some(issued.completed_result)
        && attempt.timer.submitted_at.is_some()
}

fn completed_response() -> question_model::StudentResponse {
    question_model::StudentResponse::MultipleChoice {
        selected: vec![question_model::response::ChoiceId::new("amide")],
    }
}

fn browser_completed_response(
    presentation: &PresentationV1,
) -> Result<StudentResponse, BaseCourseInstallError> {
    match project_durable_response_to_rendered_v1(&completed_response(), presentation).map_err(
        |_| {
            BaseCourseInstallError::baseline(
                "the completed Base Course answer cannot bind to its issued presentation",
            )
        },
    )? {
        InspectedStudentResponseV1::MultipleChoice { selected } if selected.len() == 1 => {
            Ok(StudentResponse::MultipleChoice {
                selected: selected
                    .into_iter()
                    .map(|id| question_model::response::ChoiceId::new(id.as_str()))
                    .collect(),
            })
        }
        _ => Err(BaseCourseInstallError::baseline(
            "the completed Base Course answer has an unexpected rendered response shape",
        )),
    }
}

async fn reject_unrelated_activity(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    enrollment: EnrollmentId,
    label: &str,
) -> Result<(), BaseCourseInstallError> {
    let page_size = PageSize::new(100).map_err(|error| {
        BaseCourseInstallError::baseline(format!(
            "the Base Course run page size is invalid: {error}"
        ))
    })?;
    let runs = store
        .list_runs(context, enrollment, PageRequest::first(page_size))
        .await
        .at("checking for retained Base Course learner activity")?;
    if !runs.items.is_empty() {
        return Err(BaseCourseInstallError::baseline(format!(
            "the {label} seed activity is missing while the learner already has another run"
        )));
    }
    Ok(())
}

async fn issue_attempt(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    student: UserId,
    ids: BaseCourseIds,
    attempt: QuestionAttemptId,
    run: RunId,
    issued: &InstalledIssuedAttempt,
) -> Result<question_model::QuestionAttempt, BaseCourseInstallError> {
    store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                binding: StudentWorkRoutingBinding::new(ids.base_course, ids.assignment),
                attempt,
                run,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                issued_question_snapshot: issued.issued_question_snapshot.clone(),
                seed: issued.envelope.seed.value(),
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(issued.presentation),
                presentation_snapshot: Some(issued.presentation_snapshot.clone()),
                grading_envelope: Some(issued.envelope.clone()),
                native_execution_envelope_capability:
                    learning_data_access::NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                webwork_replay: None,
                qti_grading: None,
                qti_grading_capability: learning_data_access::QtiGradingCapability::NotApplicable,
                parameter_hash: issued.parameter_hash.clone(),
                provenance: issued.provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .at("issuing a deterministic Base Course attempt")
}

async fn submit_attempt(
    seed_executor: &dyn AcceptedSubmissionSeedExecutor,
    context: TenantContext,
    student: UserId,
    binding: StudentWorkRoutingBinding,
    attempt: QuestionAttemptId,
    issued: &InstalledIssuedAttempt,
) -> Result<(), BaseCourseInstallError> {
    let response = issued.submission_response.clone();
    let idempotency_key = SubmissionIdempotencyKey::parse("installed-base-course-mary-answer")
        .at("forming the Base Course submission key")?;
    match seed_executor
        .execute_seed_submission(AcceptedSubmissionSeedRequest {
            context,
            actor: student,
            binding,
            attempt,
            response,
            idempotency_key,
        })
        .await
        .at("submitting the completed Base Course answer through accepted submission")?
    {
        AcceptedSubmissionSeedOutcome::Completed => Ok(()),
        AcceptedSubmissionSeedOutcome::PendingRecovery => Err(BaseCourseInstallError::baseline(
            "the completed Base Course answer remains pending accepted-submission recovery",
        )),
    }
}

fn installed_issued_attempt(
    question: &QuestionDefinition,
    seed: u64,
) -> Result<InstalledIssuedAttempt, BaseCourseInstallError> {
    let adapter = NativeAdapter::new();
    let NativeIssuedAttempt {
        envelope,
        parameter_hash,
        provenance,
    } = adapter
        .issue(question, Seed::new(seed), &[])
        .map_err(|source| {
            BaseCourseInstallError::native("issuing the Base Course native question", source)
        })?;
    let presentation = build_presentation_v1(&envelope, &[]).map_err(|source| {
        BaseCourseInstallError::presentation("building the Base Course presentation", source)
    })?;
    let submission_response = browser_completed_response(&presentation)?;
    let issued_question_snapshot = IssuedQuestionSnapshotV1::new(
        question.clone(),
        IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .at("building the Base Course issued question snapshot")?;
    issued_question_snapshot
        .validate_for_attempt(question.problem, question.version)
        .at("validating the Base Course issued question identity")?;
    issued_question_snapshot
        .validate_for_issuance_context(
            FlatGradingCapability::NotApplicable,
            learning_data_access::WebworkGradingCapability::NotApplicable,
            learning_data_access::QtiGradingCapability::NotApplicable,
            Some(&learning_data_access::ReceiptPresentationSnapshot {
                envelope: presentation.envelope.clone(),
                asset_bindings: presentation.asset_bindings.clone(),
            }),
        )
        .at("validating the Base Course issued question snapshot")?;
    Ok(InstalledIssuedAttempt {
        envelope,
        issued_question_snapshot,
        parameter_hash,
        provenance,
        presentation: PresentationBindingV1::new(
            presentation.envelope.presentation_nonce,
            presentation.digest,
        ),
        presentation_snapshot: learning_data_access::ReceiptPresentationSnapshot {
            envelope: presentation.envelope,
            asset_bindings: presentation.asset_bindings,
        },
        submission_response,
        completed_result: AttemptResult {
            points_earned: 1.0,
            points_possible: 1.0,
            correct: true,
        },
    })
}

async fn validate_persisted_issuance(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    student: UserId,
    routing: StudentWorkRoutingBinding,
    attempt: QuestionAttemptId,
    issued: &InstalledIssuedAttempt,
) -> Result<(), BaseCourseInstallError> {
    let read = store
        .read_issued_attempt_evidence(context, student, routing, attempt)
        .await
        .at("reading the Base Course issued-attempt evidence")?;
    // Both Mary (submitted) and Jack (active) validate their common immutable
    // tuple from this single broker-authorized read (ASVS 2.3.3, 15.4.2).
    let evidence = read.receipt_evidence();
    let binding = evidence.presentation_binding().ok_or_else(|| {
        BaseCourseInstallError::baseline("an attempt lacks its presentation binding")
    })?;
    let snapshot = evidence.presentation_snapshot().ok_or_else(|| {
        BaseCourseInstallError::baseline("an attempt lacks its presentation snapshot")
    })?;
    let grading_envelope = evidence
        .grading_envelope()
        .ok_or_else(|| BaseCourseInstallError::baseline("an attempt lacks its grading envelope"))?;
    let reproduced =
        reproduce_presentation_v1(&issued.envelope, &[], binding).map_err(|source| {
            BaseCourseInstallError::presentation("reproducing the Base Course presentation", source)
        })?;
    let expected_snapshot = learning_data_access::ReceiptPresentationSnapshot {
        envelope: reproduced.envelope,
        asset_bindings: reproduced.asset_bindings,
    };
    if snapshot != &expected_snapshot || grading_envelope != &issued.envelope {
        return Err(BaseCourseInstallError::baseline(
            "an attempt does not retain its native issued presentation",
        ));
    }
    Ok(())
}

trait StoreResultExt<T> {
    fn at(self, operation: &'static str) -> Result<T, BaseCourseInstallError>;
}

impl<T> StoreResultExt<T> for Result<T, StoreError> {
    fn at(self, operation: &'static str) -> Result<T, BaseCourseInstallError> {
        self.map_err(|source| BaseCourseInstallError::persistence(operation, source))
    }
}

#[cfg(test)]
mod tests {
    use question_model::{ActivityTimestamp, QuestionDefinition, RunMode, RunReference};
    use uuid::Uuid;

    use super::*;
    use crate::records::base_course_native_draft;

    fn fixture() -> (
        question_model::TenantId,
        BaseCourseIds,
        EnrollmentId,
        QuestionDefinition,
    ) {
        let tenant = question_model::TenantId::from_uuid(Uuid::from_u128(901));
        let ids = BaseCourseIds::for_tenant(tenant);
        let question = QuestionDefinition::from_draft(
            base_course_native_draft(ids.workspace),
            ids.problem,
            ids.version,
            question_model::QuestionSource::Native {
                family: "peptide_bond_geometry".to_string(),
            },
        );
        (
            tenant,
            ids,
            EnrollmentId::from_uuid(Uuid::from_u128(902)),
            question,
        )
    }

    fn run(
        tenant: question_model::TenantId,
        id: RunId,
        enrollment: EnrollmentId,
        completed: bool,
        score: f64,
    ) -> question_model::AssignmentRun {
        question_model::AssignmentRun {
            id,
            reference: RunReference::new(1).unwrap(),
            tenant,
            enrollment,
            run_number: 1,
            started_at: ActivityTimestamp::from_unix_millis(1),
            completed_at: completed.then(|| ActivityTimestamp::from_unix_millis(2)),
            score: completed.then_some(score),
            mode: RunMode::Assigned,
            variation: question_model::run_policy::VariationPolicy::NewSeeds,
        }
    }

    fn attempt(
        tenant: question_model::TenantId,
        ids: BaseCourseIds,
        id: QuestionAttemptId,
        run: RunId,
        seed: u64,
        completed: bool,
        issued: &InstalledIssuedAttempt,
    ) -> question_model::QuestionAttempt {
        question_model::QuestionAttempt {
            id,
            tenant,
            run,
            problem: ids.problem,
            question_version: ids.version,
            assignment_position: 0,
            seed,
            parameter_hash: issued.parameter_hash.clone(),
            response: completed.then(completed_response),
            status: if completed {
                AttemptStatus::Submitted
            } else {
                AttemptStatus::InProgress
            },
            result: completed.then_some(issued.completed_result),
            timer: question_model::AttemptTimerRecord {
                issued_at: ActivityTimestamp::from_unix_millis(1),
                deadline: None,
                submitted_at: completed.then(|| ActivityTimestamp::from_unix_millis(2)),
            },
            provenance: issued.provenance.clone(),
            issued_capability: question_model::IssuedAttemptCapabilityV1::PresentationEnvelope,
        }
    }

    #[test]
    fn completed_and_active_prefixes_converge_or_refuse() {
        let (tenant, ids, enrollment, question) = fixture();
        let mary = installed_issued_attempt(&question, COMPLETED_SEED).unwrap();
        let jack = installed_issued_attempt(&question, ACTIVE_SEED).unwrap();
        let mary_run = run(tenant, ids.mary_run, enrollment, false, 0.0);
        let mary_attempt = attempt(
            tenant,
            ids,
            ids.mary_attempt,
            ids.mary_run,
            COMPLETED_SEED,
            false,
            &mary,
        );
        assert_eq!(
            completed_activity_state(Some(&mary_run), Some(&mary_attempt), ids, enrollment, &mary,)
                .unwrap(),
            CompletedActivityState::IssuedAttempt
        );
        let jack_run = run(tenant, ids.jack_run, enrollment, false, 0.0);
        assert_eq!(
            active_activity_state(Some(&jack_run), None, ids, enrollment, &jack).unwrap(),
            ActiveActivityState::RunWithoutAttempt
        );
        assert!(active_activity_state(None, Some(&mary_attempt), ids, enrollment, &jack).is_err());
    }

    #[test]
    fn native_issue_replay_grading_and_presentation_are_deterministic() {
        let (_, _, _, question) = fixture();
        let issued = installed_issued_attempt(&question, COMPLETED_SEED).unwrap();
        assert_eq!(issued.issued_question_snapshot.question(), &question);
        assert!(matches!(
            issued.issued_question_snapshot.family_witness(),
            IssuedQuestionFamilyWitnessV1::Native {
                physical_asset_bindings
            } if physical_asset_bindings.is_empty()
        ));
        issued
            .issued_question_snapshot
            .validate_for_attempt(question.problem, question.version)
            .expect("snapshot is bound to the exact installed question");
        let replay = NativeAdapter::new()
            .reproduce(
                &question,
                issued.envelope.seed,
                &issued.parameter_hash,
                &issued.provenance,
                &[],
            )
            .unwrap();
        let presentation =
            reproduce_presentation_v1(&issued.envelope, &[], issued.presentation).unwrap();

        assert_eq!(replay, issued.envelope);
        assert_eq!(issued.completed_result.points_earned, 1.0);
        assert_eq!(presentation.envelope, issued.presentation_snapshot.envelope);
        assert!(
            domain::validation::validate_presentation_response_format(
                &presentation.envelope.response,
                &issued.submission_response,
            )
            .is_valid()
        );
        assert_eq!(
            question_model::presentation::translate_rendered_response_v1(
                &issued.submission_response,
                &presentation,
            )
            .unwrap(),
            completed_response(),
        );
    }

    #[test]
    fn exact_attempt_collection_refuses_duplicates_and_extra_pages() {
        let (tenant, ids, _, question) = fixture();
        let issued = installed_issued_attempt(&question, COMPLETED_SEED).unwrap();
        let value = attempt(
            tenant,
            ids,
            ids.mary_attempt,
            ids.mary_run,
            COMPLETED_SEED,
            true,
            &issued,
        );
        assert!(has_exact_attempt_collection(
            std::slice::from_ref(&value),
            false,
            Some(&value),
            ids.mary_attempt,
        ));
        assert!(!has_exact_attempt_collection(
            &[value.clone(), value],
            false,
            None,
            ids.mary_attempt,
        ));
    }
}
