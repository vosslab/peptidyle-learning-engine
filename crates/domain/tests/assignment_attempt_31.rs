//! WP-C3 acceptance: post-completion practice remains valid through Assignment Attempt 31.

use domain::assignment_activity::assignment_attempt_continuation_allows_assignment_attempt;
use domain::scoring::{AssignmentActivityTransition, project_summary};
use question_model::{
    ActivityTimestamp, AssignmentAttemptContinuationRule, AssignmentAttemptGradeRule, AssignmentId,
    AssignmentProgressRecord, StudentRecordId,
};
use uuid::Uuid;

#[test]
fn thirty_first_assignment_attempt_updates_the_transactional_summary() {
    let mut summary = AssignmentProgressRecord::empty(
        StudentRecordId::from_uuid(Uuid::from_u128(2)),
        AssignmentId::from_uuid(Uuid::from_u128(3)),
    );

    for attempt_number in 1_u32..=31 {
        assert!(assignment_attempt_continuation_allows_assignment_attempt(
            &summary,
            AssignmentAttemptContinuationRule::Unlimited
        ));

        for attempt_number in 1_i64..=3 {
            summary = project_summary(
                &summary,
                AssignmentActivityTransition::QuestionAttemptRecorded {
                    at: ActivityTimestamp::from_unix_millis(attempt_number * 100 + attempt_number),
                },
                AssignmentAttemptGradeRule::Highest,
            )
            .expect("question attempt should update the summary");
        }

        summary = project_summary(
            &summary,
            AssignmentActivityTransition::Completed {
                score: f64::from(attempt_number) / 31.0,
                at: ActivityTimestamp::from_unix_millis(i64::from(attempt_number) * 100 + 4),
            },
            AssignmentAttemptGradeRule::Highest,
        )
        .expect("completed run should update the summary");
    }

    let expected = AssignmentProgressRecord {
        student_record: StudentRecordId::from_uuid(Uuid::from_u128(2)),
        assignment: AssignmentId::from_uuid(Uuid::from_u128(3)),
        current_score: Some(1.0),
        best_score: Some(1.0),
        latest_score: Some(1.0),
        completed_assignment_attempt_count: 31,
        total_question_attempts: 93,
        last_activity_at: Some(ActivityTimestamp::from_unix_millis(3_104)),
    };

    assert_eq!(summary, expected);
}
