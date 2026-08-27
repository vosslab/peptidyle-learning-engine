//! Pure, answer-free successor eligibility for completed learner work.

use question_model::{
    AssignmentRun, AssignmentRunItem, AttemptStatus, ProblemVersionRef, QuestionAttempt,
    QuestionDefinition,
};

use crate::StoreError;

/// Reports whether a completed receipt has an unissued successor that the
/// learner's run plan still permits.  Callers supply only immutable run-item
/// definitions, current attempt projections, and exact published question
/// policies; issuing remains a separate mutation owned by run start/resume.
pub(crate) fn successor_is_eligible(
    run: &AssignmentRun,
    predecessor_resolved: bool,
    run_items: &[AssignmentRunItem],
    attempts: &[QuestionAttempt],
    questions: &[(ProblemVersionRef, QuestionDefinition)],
) -> Result<bool, StoreError> {
    if run.completed_at.is_some() || run.score.is_some() || predecessor_resolved {
        return Ok(false);
    }
    if attempts
        .iter()
        .any(|attempt| attempt.status == AttemptStatus::InProgress)
    {
        return Ok(false);
    }
    for item in run_items {
        let position_attempts = attempts
            .iter()
            .filter(|attempt| attempt.assignment_position == item.issued_position)
            .collect::<Vec<_>>();
        if position_attempts.is_empty() {
            return Ok(true);
        }
        if position_attempts
            .iter()
            .filter_map(|attempt| attempt.result)
            .any(|result| result.correct)
        {
            continue;
        }
        let question = questions
            .iter()
            .find(|(reference, _)| *reference == item.reference)
            .map(|(_, question)| question)
            .ok_or(StoreError::Unavailable(
                "run item has no immutable published question".to_string(),
            ))?;
        if question.problem != item.reference.problem || question.version != item.reference.version
        {
            return Err(StoreError::Unavailable(
                "run item question identity is incoherent".to_string(),
            ));
        }
        let count = u32::try_from(position_attempts.len()).map_err(|_| StoreError::Conflict)?;
        if question
            .attempt_policy
            .max_attempts
            .is_none_or(|maximum| count < maximum)
        {
            return Ok(true);
        }
    }
    Ok(false)
}
