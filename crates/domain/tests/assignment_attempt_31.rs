//! Post-completion practice remains valid through the thirty-first Assignment Attempt.

use domain::assignment_activity::assignment_attempt_continuation_allows_assignment_attempt;
use domain::scoring::{AssignmentActivityTransition, project_assignment_activity};
use question_model::{
    AssignmentAttemptContinuationRule, AssignmentAttemptGradeRule, AssignmentAttemptId,
    AssignmentGrade, AssignmentId, AssignmentProgressRecord, StudentRecordId, Timestamp,
};
use uuid::Uuid;

#[test]
fn thirty_first_assignment_attempt_updates_the_transactional_summary() {
    let student_record = StudentRecordId::from_uuid(Uuid::from_u128(2));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(3));
    let mut grade = AssignmentGrade::empty(student_record, assignment);
    let mut progress = AssignmentProgressRecord::empty(student_record, assignment);

    for attempt_number in 1_u32..=31 {
        assert!(assignment_attempt_continuation_allows_assignment_attempt(
            &progress,
            AssignmentAttemptContinuationRule::Unlimited
        ));

        for attempt_number in 1_i64..=3 {
            (grade, progress) = project_assignment_activity(
                &grade,
                &progress,
                AssignmentActivityTransition::QuestionAttemptRecorded {
                    at: Timestamp::from_unix_millis(attempt_number * 100 + attempt_number),
                },
                AssignmentAttemptGradeRule::Highest,
            )
            .expect("question attempt should update the summary");
        }

        (grade, progress) = project_assignment_activity(
            &grade,
            &progress,
            AssignmentActivityTransition::Completed {
                assignment_attempt: AssignmentAttemptId::from_uuid(Uuid::from_u128(
                    100 + u128::from(attempt_number),
                )),
                score: f64::from(attempt_number) / 31.0,
                at: Timestamp::from_unix_millis(i64::from(attempt_number) * 100 + 4),
            },
            AssignmentAttemptGradeRule::Highest,
        )
        .expect("completed Assignment Attempt should update the summary");
    }

    let expected_progress = AssignmentProgressRecord {
        student_record,
        assignment,
        completed_assignment_attempt_count: 31,
        total_question_attempts: 93,
        last_activity_at: Some(Timestamp::from_unix_millis(3_104)),
    };

    assert_eq!(progress, expected_progress);
    assert_eq!(grade.current_score, Some(1.0));
    assert_eq!(grade.best_score, Some(1.0));
    assert_eq!(grade.latest_score, Some(1.0));
}
