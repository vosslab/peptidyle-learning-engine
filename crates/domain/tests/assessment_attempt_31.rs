//! Post-completion practice remains valid through the thirty-first Assessment Attempt.

use domain::assessment_activity::assessment_attempt_continuation_allows_assessment_attempt;
use domain::scoring::{AssessmentActivityTransition, project_assessment_activity};
use question_model::{
    AssessmentAttemptContinuationRule, AssessmentAttemptGradeRule, AssessmentAttemptId,
    AssessmentGrade, AssessmentId, AssessmentProgressRecord, StudentRecordId, Timestamp,
};
use uuid::Uuid;

#[test]
fn thirty_first_assessment_attempt_updates_the_transactional_summary() {
    let student_record = StudentRecordId::from_uuid(Uuid::from_u128(2));
    let assessment = AssessmentId::from_uuid(Uuid::from_u128(3));
    let mut grade = AssessmentGrade::empty(student_record, assessment);
    let mut progress = AssessmentProgressRecord::empty(student_record, assessment);

    for attempt_number in 1_u32..=31 {
        assert!(assessment_attempt_continuation_allows_assessment_attempt(
            &progress,
            AssessmentAttemptContinuationRule::Unlimited
        ));

        for attempt_number in 1_i64..=3 {
            (grade, progress) = project_assessment_activity(
                &grade,
                &progress,
                AssessmentActivityTransition::QuestionAttemptRecorded {
                    at: Timestamp::from_unix_millis(attempt_number * 100 + attempt_number),
                },
                AssessmentAttemptGradeRule::Highest,
            )
            .expect("question attempt should update the summary");
        }

        (grade, progress) = project_assessment_activity(
            &grade,
            &progress,
            AssessmentActivityTransition::Completed {
                assessment_attempt: AssessmentAttemptId::from_uuid(Uuid::from_u128(
                    100 + u128::from(attempt_number),
                )),
                score: f64::from(attempt_number) / 31.0,
                at: Timestamp::from_unix_millis(i64::from(attempt_number) * 100 + 4),
            },
            AssessmentAttemptGradeRule::Highest,
        )
        .expect("completed Assessment Attempt should update the summary");
    }

    let expected_progress = AssessmentProgressRecord {
        student_record,
        assessment,
        completed_assessment_attempt_count: 31,
        total_question_attempts: 93,
        last_activity_at: Some(Timestamp::from_unix_millis(3_104)),
    };

    assert_eq!(progress, expected_progress);
    assert_eq!(grade.current_score, Some(1.0));
    assert_eq!(grade.best_score, Some(1.0));
    assert_eq!(grade.latest_score, Some(1.0));
}
