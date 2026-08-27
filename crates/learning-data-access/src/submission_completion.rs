//! Pure projection of one authoritative submitted attempt into completion state.
//!
//! Persistence adapters validate and gather immutable records first, then use
//! this planner before mutating their aggregate. Keeping the scoring lifecycle
//! here makes synchronous and worker-owned submission paths converge without
//! giving this module access to responses, receipts, clocks, or stores.

use question_model::{
    ActivityTimestamp, AssignmentEnrollment, AssignmentRun, AssignmentRunItem, QuestionAttempt,
    StudentAssignmentSummary,
};

use crate::{
    AcceptedSubmissionGrade, AssignmentRecord, CompletedSubmissionReceipt,
    ReceiptPresentationSnapshot, StatisticsContribution, StoreError, completed_run_score,
    current_attempt_points, current_run_questions, derive_statistics_contributions, grade_policy,
    private_feedback_record, project_enrollment_completion,
};

/// Inputs already validated and coherently loaded by the persistence boundary.
pub(crate) struct AcceptedSubmissionCompletionInput {
    pub(crate) base_attempt: QuestionAttempt,
    pub(crate) grade: AcceptedSubmissionGrade,
    pub(crate) assignment: AssignmentRecord,
    pub(crate) run: AssignmentRun,
    pub(crate) enrollment: AssignmentEnrollment,
    pub(crate) previous_summary: StudentAssignmentSummary,
    pub(crate) run_items: Vec<AssignmentRunItem>,
    pub(crate) attempts: Vec<QuestionAttempt>,
    pub(crate) accepted_at: ActivityTimestamp,
    pub(crate) presentation: Option<ReceiptPresentationSnapshot>,
}

/// Complete, immutable projections ready for one atomic persistence commit.
pub(crate) struct AcceptedSubmissionCompletionPlan {
    pub(crate) receipt: CompletedSubmissionReceipt,
    pub(crate) enrollment: AssignmentEnrollment,
    pub(crate) statistics: Option<Vec<StatisticsContribution>>,
}

pub(crate) fn plan_accepted_submission_completion(
    input: AcceptedSubmissionCompletionInput,
) -> Result<AcceptedSubmissionCompletionPlan, StoreError> {
    let AcceptedSubmissionCompletionInput {
        base_attempt,
        grade,
        assignment,
        mut run,
        mut enrollment,
        previous_summary,
        run_items,
        attempts,
        accepted_at,
        presentation,
    } = input;
    let result = grade.evidence.result;
    crate::validate_attempt_result(result)?;
    let mut submitted_attempt = base_attempt;
    submitted_attempt.response = None;
    submitted_attempt.status = question_model::AttemptStatus::Submitted;
    submitted_attempt.result = Some(result);
    submitted_attempt.timer.submitted_at = Some(accepted_at);
    let attempts = attempts
        .into_iter()
        .map(|attempt| {
            if attempt.id == submitted_attempt.id {
                submitted_attempt.clone()
            } else {
                attempt
            }
        })
        .collect::<Vec<_>>();
    let mut summary = domain::scoring::project_summary(
        &previous_summary,
        domain::scoring::RunTransition::QuestionAttemptRecorded { at: accepted_at },
        grade_policy(&assignment),
    )?;
    let questions = current_run_questions(&assignment, &run_items, &attempts, &submitted_attempt)?;
    let results = questions
        .iter()
        .map(|question| question.map(|question| question.result))
        .collect::<Vec<_>>();
    let submitted_item = run_items
        .iter()
        .find(|item| item.issued_position == submitted_attempt.assignment_position)
        .ok_or_else(|| {
            StoreError::Unavailable("submitted attempt has no immutable run item".to_string())
        })?;
    let _ = current_attempt_points(
        &assignment,
        submitted_item.assignment_item,
        submitted_attempt.status,
        result,
    )?;
    let mut statistics = None;
    if let Some(score) = completed_run_score(&questions, assignment.policies.completion)? {
        summary = domain::scoring::project_summary(
            &summary,
            domain::scoring::RunTransition::Completed {
                score,
                at: accepted_at,
            },
            grade_policy(&assignment),
        )?;
        run.completed_at = Some(accepted_at);
        run.score = Some(score);
        project_enrollment_completion(
            &mut enrollment,
            &previous_summary,
            grade_policy(&assignment),
            run.id,
            score,
            accepted_at,
        );
        if run.mode == question_model::RunMode::Assigned
            && previous_summary.completed_run_count == 0
        {
            statistics = Some(derive_statistics_contributions(
                &run_items, &results, &attempts,
            )?);
        }
    }
    Ok(AcceptedSubmissionCompletionPlan {
        receipt: CompletedSubmissionReceipt {
            attempt: submitted_attempt,
            feedback: private_feedback_record(grade.feedback)?,
            run: run.clone(),
            summary: summary.clone(),
            presentation,
        },
        enrollment,
        statistics,
    })
}
