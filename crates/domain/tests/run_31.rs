//! WP-C3 acceptance: post-completion practice remains valid through run 31.

use domain::run::continued_practice_allows_run;
use domain::scoring::{RunTransition, project_summary};
use question_model::{
    ActivityTimestamp, ContinuedPractice, EnrollmentId, GradePolicy, StudentAssignmentSummary,
};
use uuid::Uuid;

#[test]
fn thirty_first_run_updates_the_transactional_summary() {
    let mut summary = StudentAssignmentSummary::empty(EnrollmentId::from_uuid(Uuid::from_u128(2)));

    for run_number in 1_u32..=31 {
        assert!(continued_practice_allows_run(
            &summary,
            ContinuedPractice::Unlimited
        ));

        for attempt_number in 1_i64..=3 {
            summary = project_summary(
                &summary,
                RunTransition::QuestionAttemptRecorded {
                    at: ActivityTimestamp::from_unix_millis(
                        i64::from(run_number) * 100 + attempt_number,
                    ),
                },
                GradePolicy::Highest,
            )
            .expect("question attempt should update the summary");
        }

        summary = project_summary(
            &summary,
            RunTransition::Completed {
                score: f64::from(run_number) / 31.0,
                at: ActivityTimestamp::from_unix_millis(i64::from(run_number) * 100 + 4),
            },
            GradePolicy::Highest,
        )
        .expect("completed run should update the summary");
    }

    let expected = StudentAssignmentSummary {
        enrollment: EnrollmentId::from_uuid(Uuid::from_u128(2)),
        current_score: Some(1.0),
        best_score: Some(1.0),
        latest_score: Some(1.0),
        completed_run_count: 31,
        total_question_attempts: 93,
        last_activity_at: Some(ActivityTimestamp::from_unix_millis(3_104)),
    };

    assert_eq!(summary, expected);
}
