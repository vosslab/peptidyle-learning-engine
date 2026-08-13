//! Host-only E2E seed scoring capability.

use super::*;
use question_model::envelope::QuestionEnvelope;
use question_model::generation::Seed;
use question_model::presentation::{
    NonceSourceV1, PresentationBuildError, build_presentation_v1_with_nonce_source,
};

/// Fixed nonce source for the deterministic disposable database acceptance seed.
///
/// The seed's issue command and its submission receipt must describe the same
/// public presentation. Production issuance uses operating-system randomness;
/// this fixture uses a stable nonce so its immutable binding can be asserted.
struct SeedNonce([u8; 16]);

impl NonceSourceV1 for SeedNonce {
    fn next_nonce(&mut self) -> Result<[u8; 16], PresentationBuildError> {
        Ok(self.0)
    }
}

fn issued_scoring_presentation(
    ids: SeedIds,
    seed: u64,
) -> Result<(
    PresentationBindingV1,
    learning_data_access::ReceiptPresentationSnapshot,
    QuestionEnvelope,
)> {
    let definition = native_draft(ids.workspace);
    let envelope = QuestionEnvelope {
        version: ids.version,
        seed: Seed::new(seed),
        title: definition.metadata.title,
        prompt: definition.prompt,
        response: definition.response,
    };
    let nonce_byte = u8::try_from(seed).context("seed fixture nonce exceeds one byte")?;
    let mut nonce = SeedNonce([nonce_byte; 16]);
    let presentation = build_presentation_v1_with_nonce_source(&envelope, &[], &mut nonce)
        .map_err(|error| anyhow::anyhow!("building E2E receipt presentation: {error}"))?;
    let binding = PresentationBindingV1::new(
        presentation.envelope.presentation_nonce,
        presentation.digest,
    );
    Ok((
        binding,
        learning_data_access::ReceiptPresentationSnapshot {
            envelope: presentation.envelope,
            asset_bindings: presentation.asset_bindings,
        },
        envelope,
    ))
}

pub(super) async fn exercise_scoring_generation(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    instructor: UserId,
    student: UserId,
    ids: SeedIds,
    assignment: AssignmentRecord,
) -> Result<()> {
    let (presentation_binding, presentation, grading_envelope) =
        issued_scoring_presentation(ids, 17)?;
    let run = store
        .start_or_resume_run(context, student, ids.assignment, ids.run)
        .await
        .context("starting database scoring acceptance run")?;
    let implementation = |name: &str| ImplementationVersion {
        id: name.to_string(),
        version: "acceptance-1".to_string(),
    };
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.attempt,
                run: run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 17,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(presentation_binding),
                presentation_snapshot: Some(presentation.clone()),
                grading_envelope: Some(grading_envelope),
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                webwork_replay: None,
                parameter_hash: "database-scoring-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("native"),
                    rendered_question_sha256: "database-scoring-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing database scoring acceptance attempt")?;
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: attempt.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: false,
                    points_earned: 0.5,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("database-scoring-acceptance")?,
            },
        )
        .await
        .context("submitting database scoring acceptance attempt")?;
    let current = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading assignment before database scoring acceptance edit")?
        .ok_or_else(|| anyhow::anyhow!("acceptance assignment disappeared"))?;
    let mut items = assignment.items;
    items[0].points_possible = PointValue::from_whole(2);
    let changed = store
        .replace_assignment_preserving_timing(
            context,
            ids.course,
            ids.assignment,
            current.revision,
            AssignmentUpdate {
                title: current.record.title,
                items,
                selection_groups: current.record.selection_groups,
                policies: current.record.policies,
            },
        )
        .await
        .context("changing points for database scoring acceptance")?;
    if changed.scoring_status != question_model::ScoringStatus::Recalculating {
        bail!("point change did not hide stale scores as recalculating");
    }
    let claimed = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(60)?)
        .await
        .context("claiming database scoring acceptance job")?
        .ok_or_else(|| anyhow::anyhow!("point change did not queue scoring work"))?;
    let JobPayload::RecalculateAssignment {
        assignment,
        generation,
    } = claimed.payload
    else {
        bail!("database scoring acceptance claimed another job family");
    };
    let command = AssignmentScoringWorkerCommand {
        job: claimed.id,
        lease: claimed.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(context, command)
        .await
        .context("staging database scoring generation")?;
    let staged = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading privately staged database scoring assignment")?
        .ok_or_else(|| anyhow::anyhow!("staged acceptance assignment disappeared"))?;
    if staged.scoring_status != question_model::ScoringStatus::Recalculating {
        bail!("private staging exposed partial current scores");
    }
    let mut superseding_items = changed.record.items.clone();
    superseding_items[0].points_possible = PointValue::from_whole(3);
    let superseding = store
        .replace_assignment_preserving_timing(
            context,
            ids.course,
            ids.assignment,
            changed.revision,
            AssignmentUpdate {
                title: changed.record.title,
                items: superseding_items,
                selection_groups: changed.record.selection_groups,
                policies: changed.record.policies,
            },
        )
        .await
        .context("superseding an in-flight database scoring generation")?;
    if superseding.scoring_generation <= generation
        || superseding.scoring_status != question_model::ScoringStatus::Recalculating
    {
        bail!("new scoring generation did not supersede staged work");
    }
    if store
        .commit_assignment_scoring(context, command)
        .await
        .context("discarding superseded database scoring staging")?
        != AssignmentScoringCommitOutcome::Superseded
    {
        bail!("old scoring generation was not discarded");
    }
    let pending = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading assignment after superseded database scoring work")?
        .ok_or_else(|| anyhow::anyhow!("superseded acceptance assignment disappeared"))?;
    if pending.scoring_generation != superseding.scoring_generation
        || pending.scoring_status != question_model::ScoringStatus::Recalculating
    {
        bail!("discarding old scoring work changed the current generation");
    }
    let current_job = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(60)?)
        .await
        .context("claiming superseding database scoring acceptance job")?
        .ok_or_else(|| anyhow::anyhow!("superseding point change did not queue scoring work"))?;
    let JobPayload::RecalculateAssignment {
        assignment: current_assignment,
        generation: current_generation,
    } = current_job.payload
    else {
        bail!("database scoring acceptance claimed another job family");
    };
    if current_assignment != ids.assignment || current_generation != superseding.scoring_generation
    {
        bail!("database scoring acceptance claimed the wrong generation");
    }
    let current_command = AssignmentScoringWorkerCommand {
        job: current_job.id,
        lease: current_job.lease_token,
        assignment: current_assignment,
        generation: current_generation,
    };
    store
        .prepare_assignment_scoring(context, current_command)
        .await
        .context("staging current database scoring generation")?;
    let concurrent_run = store
        .start_or_resume_run(context, student, ids.assignment, ids.concurrent_run)
        .await
        .context("starting a run during database scoring acceptance")?;
    let (concurrent_presentation_binding, concurrent_presentation, concurrent_grading_envelope) =
        issued_scoring_presentation(ids, 18)?;
    let concurrent_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.concurrent_attempt,
                run: concurrent_run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 18,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(concurrent_presentation_binding),
                presentation_snapshot: Some(concurrent_presentation.clone()),
                grading_envelope: Some(concurrent_grading_envelope),
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                webwork_replay: None,
                parameter_hash: "database-scoring-concurrent-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("native"),
                    rendered_question_sha256: "database-scoring-concurrent-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing an attempt during database scoring acceptance")?;
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: concurrent_attempt.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("database-scoring-concurrent")?,
            },
        )
        .await
        .context("submitting an attempt during database scoring acceptance")?;
    if store
        .commit_assignment_scoring(context, current_command)
        .await
        != Err(StoreError::Conflict)
    {
        bail!("stale scoring staging ignored a concurrent submission");
    }
    store
        .prepare_assignment_scoring(context, current_command)
        .await
        .context("restaging after concurrent database scoring activity")?;
    if store
        .commit_assignment_scoring(context, current_command)
        .await
        .context("committing current database scoring generation")?
        != AssignmentScoringCommitOutcome::Committed
    {
        bail!("current scoring generation did not commit");
    }
    let committed = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading committed database scoring assignment")?
        .ok_or_else(|| anyhow::anyhow!("rescored acceptance assignment disappeared"))?;
    if committed.scoring_status != question_model::ScoringStatus::Current
        || committed.scoring_generation != current_generation
    {
        bail!("rescored assignment did not become current");
    }
    assert_seed_summary_score(store, context, student, ids.run, 1.0).await?;
    exercise_attempt_support(store, context, instructor, student, ids).await?;
    recalculate_seed_item(
        store,
        context,
        ids,
        PointValue::ZERO,
        AssignmentScoringMode::Normal,
    )
    .await?;
    assert_seed_summary_score(store, context, student, ids.run, 0.0).await?;
    recalculate_seed_item(
        store,
        context,
        ids,
        PointValue::from_whole(4),
        AssignmentScoringMode::FullCredit,
    )
    .await?;
    assert_seed_summary_score(store, context, student, ids.run, 1.0).await?;
    recalculate_seed_item(
        store,
        context,
        ids,
        PointValue::from_whole(2),
        AssignmentScoringMode::ExtraCredit,
    )
    .await?;
    assert_seed_summary_score(store, context, student, ids.run, 2.0).await?;
    recalculate_seed_item(
        store,
        context,
        ids,
        PointValue::from_whole(2),
        AssignmentScoringMode::Excluded,
    )
    .await?;
    assert_seed_summary_score(store, context, student, ids.run, 0.0).await?;
    exercise_delete_and_regrade(store, context, student, ids).await?;
    Ok(())
}

pub(super) async fn exercise_attempt_support(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    instructor: UserId,
    student: UserId,
    ids: SeedIds,
) -> Result<()> {
    let (presentation_binding, presentation, grading_envelope) =
        issued_scoring_presentation(ids, 20)?;
    let run = store
        .start_or_resume_run(context, student, ids.assignment, ids.support_run)
        .await
        .context("starting attempt-support acceptance run")?;
    let provenance = AttemptProvenance {
        adapter: ImplementationVersion {
            id: "native".to_string(),
            version: "acceptance-1".to_string(),
        },
        renderer: None,
        generator: None,
        source_artifact: None,
        asset_objects: Vec::new(),
        grading: ImplementationVersion {
            id: "native".to_string(),
            version: "acceptance-1".to_string(),
        },
        rendered_question_sha256: "database-attempt-support-render".to_string(),
    };
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.support_attempt,
                run: run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 20,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(presentation_binding),
                presentation_snapshot: Some(presentation.clone()),
                grading_envelope: Some(grading_envelope),
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                webwork_replay: None,
                parameter_hash: "database-attempt-support-parameters".to_string(),
                provenance: provenance.clone(),
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing force-submit acceptance attempt")?;
    let force_action = AttemptSupportActionId::from_uuid(derived_uuid(
        context.tenant_id(),
        "support-force-action",
    ));
    let forced = store
        .force_submit_attempt(
            context,
            ForceSubmitAttemptCommand {
                action: force_action,
                actor: instructor,
                attempt: attempt.id,
            },
        )
        .await
        .context("force-submitting active database attempt")?;
    if forced.previous_status != AttemptStatus::InProgress
        || forced.resulting_status != AttemptStatus::NeedsManualGrading
    {
        bail!("force-submit stored an invalid status transition");
    }
    if store
        .force_submit_attempt(
            context,
            ForceSubmitAttemptCommand {
                action: force_action,
                actor: instructor,
                attempt: attempt.id,
            },
        )
        .await
        .context("replaying database force-submit")?
        != forced
    {
        bail!("force-submit retry did not return the original audit record");
    }
    let current = store
        .get_question_attempt(context, attempt.id)
        .await
        .context("reading force-submitted database attempt")?
        .ok_or_else(|| anyhow::anyhow!("force-submitted database attempt disappeared"))?;
    if current.status != AttemptStatus::NeedsManualGrading
        || current.response.is_some()
        || current.result.is_some()
        || current.timer.submitted_at != Some(forced.occurred_at)
    {
        bail!("force-submit fabricated work or failed to expose current status");
    }
    if store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: attempt.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("database-submit-after-force")?,
            },
        )
        .await
        != Err(StoreError::Conflict)
    {
        bail!("ordinary submission remained open after force-submit");
    }

    let clear_forced_action = AttemptSupportActionId::from_uuid(derived_uuid(
        context.tenant_id(),
        "support-clear-forced-action",
    ));
    store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_forced_action,
                actor: instructor,
                attempt: attempt.id,
            },
        )
        .await
        .context("clearing force-submitted database attempt")?;
    let (replacement_presentation_binding, replacement_presentation, replacement_grading_envelope) =
        issued_scoring_presentation(ids, 21)?;
    let replacement = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.support_replacement,
                run: run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 21,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(replacement_presentation_binding),
                presentation_snapshot: Some(replacement_presentation.clone()),
                grading_envelope: Some(replacement_grading_envelope),
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                webwork_replay: None,
                parameter_hash: "database-attempt-support-replacement".to_string(),
                provenance,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing replacement after database clear")?;
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: replacement.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse(
                    "database-attempt-support-replacement",
                )?,
            },
        )
        .await
        .context("submitting database support replacement")?;
    let clear_scored_action = AttemptSupportActionId::from_uuid(derived_uuid(
        context.tenant_id(),
        "support-clear-scored-action",
    ));
    let cleared = store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_scored_action,
                actor: instructor,
                attempt: replacement.id,
            },
        )
        .await
        .context("clearing scored database attempt")?;
    if cleared.previous_status != AttemptStatus::Submitted
        || cleared.resulting_status != AttemptStatus::Cleared
    {
        bail!("clear stored an invalid submitted-attempt transition");
    }
    if store
        .clear_attempt(
            context,
            ClearAttemptCommand {
                action: clear_scored_action,
                actor: instructor,
                attempt: replacement.id,
            },
        )
        .await
        .context("replaying scored database clear")?
        != cleared
    {
        bail!("clear retry did not return the original audit record");
    }
    let pending = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading assignment after database clear")?
        .ok_or_else(|| anyhow::anyhow!("assignment disappeared after database clear"))?;
    if pending.scoring_status != question_model::ScoringStatus::Recalculating {
        bail!("clearing a scored attempt did not fence the current grade");
    }
    commit_next_seed_scoring_job(store, context, ids.assignment, pending.scoring_generation)
        .await?;
    if store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(30)?)
        .await
        .context("checking exact clear retry queue effects")?
        .is_some()
    {
        bail!("an exact clear retry queued duplicate scoring work");
    }
    if !store
        .get_run_summary_page(
            context,
            student,
            run.id,
            PageRequest::first(PageSize::new(10)?),
        )
        .await
        .context("reading student summary after database clear")?
        .outcomes
        .items
        .is_empty()
    {
        bail!("student summary exposed cleared database evidence");
    }
    if store
        .get_run_summary_page(
            context,
            instructor,
            run.id,
            PageRequest::first(PageSize::new(10)?),
        )
        .await
        .context("reading instructor evidence after database clear")?
        .outcomes
        .items
        .len()
        != 2
    {
        bail!("instructor database summary lost cleared evidence");
    }
    Ok(())
}

pub(super) async fn exercise_delete_and_regrade(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    student: UserId,
    ids: SeedIds,
) -> Result<()> {
    let (presentation_binding, presentation, grading_envelope) =
        issued_scoring_presentation(ids, 19)?;
    let run = store
        .start_or_resume_run(context, student, ids.assignment, ids.retirement_run)
        .await
        .context("starting Delete and Regrade acceptance run")?;
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.retirement_attempt,
                run: run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 19,
                presentation_capability: PresentationCapability::EnvelopeV1,
                presentation: Some(presentation_binding),
                presentation_snapshot: Some(presentation.clone()),
                grading_envelope: Some(grading_envelope),
                flat_grading: None,
                flat_grading_capability: learning_data_access::FlatGradingCapability::NotApplicable,
                webwork_grading: None,
                webwork_grading_capability:
                    learning_data_access::WebworkGradingCapability::NotApplicable,
                webwork_replay: None,
                parameter_hash: "database-delete-and-regrade-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: ImplementationVersion {
                        id: "native".to_string(),
                        version: "acceptance-1".to_string(),
                    },
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: ImplementationVersion {
                        id: "native".to_string(),
                        version: "acceptance-1".to_string(),
                    },
                    rendered_question_sha256: "database-delete-and-regrade-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing Delete and Regrade acceptance attempt")?;
    let current = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading Delete and Regrade acceptance assignment")?
        .ok_or_else(|| anyhow::anyhow!("Delete and Regrade assignment disappeared"))?;
    let command = DeleteAndRegradeAssignmentItemCommand {
        course: ids.course,
        assignment: ids.assignment,
        item: ids.assignment_item,
        expected_revision: current.revision,
    };
    if store
        .delete_and_regrade_assignment_item(context, command)
        .await
        != Err(StoreError::Conflict)
    {
        bail!("Delete and Regrade did not block an affected active attempt");
    }
    store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                attempt: attempt.id,
                response: question_model::StudentResponse::MultipleChoice {
                    selected: vec![ChoiceId::new("amide")],
                },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("database-delete-and-regrade")?,
            },
        )
        .await
        .context("submitting Delete and Regrade acceptance attempt")?;
    let retired = store
        .delete_and_regrade_assignment_item(context, command)
        .await
        .context("retiring submitted Delete and Regrade item")?;
    let item = retired
        .record
        .items
        .iter()
        .find(|item| item.id == ids.assignment_item)
        .ok_or_else(|| anyhow::anyhow!("retired item tombstone disappeared"))?;
    if item.delivery_state != AssignmentDeliveryState::Retired
        || item.scoring_mode != AssignmentScoringMode::Excluded
        || retired.scoring_status != question_model::ScoringStatus::Recalculating
    {
        bail!("Delete and Regrade did not persist an excluded tombstone");
    }
    let replay = store
        .delete_and_regrade_assignment_item(
            context,
            DeleteAndRegradeAssignmentItemCommand {
                expected_revision: retired.revision,
                ..command
            },
        )
        .await
        .context("replaying Delete and Regrade acceptance command")?;
    if replay != retired {
        bail!("Delete and Regrade replay created another revision");
    }
    commit_next_seed_scoring_job(store, context, ids.assignment, retired.scoring_generation)
        .await?;
    if !store
        .get_run_summary_page(
            context,
            student,
            run.id,
            PageRequest::first(PageSize::new(10)?),
        )
        .await
        .context("reading student summary after Delete and Regrade")?
        .outcomes
        .items
        .is_empty()
    {
        bail!("student summary exposed retired response or feedback");
    }
    let future = store
        .start_or_resume_run(context, student, ids.assignment, ids.post_retirement_run)
        .await
        .context("starting post-retirement acceptance run")?;
    if !store
        .assignment_run_items(context, future.id)
        .await
        .context("reading post-retirement acceptance run items")?
        .is_empty()
    {
        bail!("future run still delivered a retired assignment item");
    }
    Ok(())
}

pub(super) async fn recalculate_seed_item(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    ids: SeedIds,
    points: PointValue,
    mode: AssignmentScoringMode,
) -> Result<()> {
    let current = store
        .get_assignment_for_edit(context, ids.assignment)
        .await
        .context("reading assignment before scoring-mode acceptance edit")?
        .ok_or_else(|| anyhow::anyhow!("scoring-mode acceptance assignment disappeared"))?;
    let mut items = current.record.items.clone();
    items[0].points_possible = points;
    items[0].scoring_mode = mode;
    let changed = store
        .replace_assignment_preserving_timing(
            context,
            ids.course,
            ids.assignment,
            current.revision,
            AssignmentUpdate {
                title: current.record.title,
                items,
                selection_groups: current.record.selection_groups,
                policies: current.record.policies,
            },
        )
        .await
        .context("changing scoring mode for database acceptance")?;
    if changed.scoring_status != question_model::ScoringStatus::Recalculating {
        bail!("scoring-mode edit did not enter recalculation");
    }
    commit_next_seed_scoring_job(store, context, ids.assignment, changed.scoring_generation).await
}

pub(super) async fn commit_next_seed_scoring_job(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    expected_assignment: AssignmentId,
    expected_generation: question_model::ScoringGeneration,
) -> Result<()> {
    let claimed = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(60)?)
        .await
        .context("claiming database scoring acceptance job")?
        .ok_or_else(|| anyhow::anyhow!("database scoring edit did not queue work"))?;
    let JobPayload::RecalculateAssignment {
        assignment,
        generation,
    } = claimed.payload
    else {
        bail!("database scoring acceptance claimed another job family");
    };
    if assignment != expected_assignment || generation != expected_generation {
        bail!("database scoring acceptance claimed the wrong generation");
    }
    let command = AssignmentScoringWorkerCommand {
        job: claimed.id,
        lease: claimed.lease_token,
        assignment,
        generation,
    };
    store
        .prepare_assignment_scoring(context, command)
        .await
        .context("staging database scoring acceptance generation")?;
    if store
        .commit_assignment_scoring(context, command)
        .await
        .context("committing database scoring acceptance generation")?
        != AssignmentScoringCommitOutcome::Committed
    {
        bail!("database scoring acceptance generation did not commit");
    }
    Ok(())
}

pub(super) async fn assert_seed_summary_score(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    student: UserId,
    run: RunId,
    expected: f64,
) -> Result<()> {
    let page = store
        .get_run_summary_page(context, student, run, PageRequest::first(PageSize::new(1)?))
        .await
        .context("reading database scoring acceptance summary")?;
    let actual = page
        .summary
        .current_score
        .ok_or_else(|| anyhow::anyhow!("database scoring acceptance summary has no score"))?;
    if (actual - expected).abs() > f64::EPSILON {
        bail!("database scoring acceptance expected summary {expected}, got {actual}");
    }
    Ok(())
}
