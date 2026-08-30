//! Deterministic tests for the answer-free PostgreSQL item-analysis reducer.

use super::*;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn context() -> AnalysisReportContext {
    AnalysisReportContext {
        course: CourseId::from_uuid(uuid(2)),
        assignment: AssignmentId::from_uuid(uuid(3)),
        generation: ScoringGeneration::INITIAL,
        analyzed_at: ActivityTimestamp::from_unix_millis(4),
        completed_run_count: 1,
        in_progress_run_count: 0,
    }
}

struct EvaluationState<'a> {
    status: &'a str,
    execution_state: &'a str,
    grading_status: &'a str,
    has_completion_receipt: bool,
}

struct AutomatedScore {
    credit: BigDecimal,
    correct: bool,
    earned_points: BigDecimal,
    possible_points: BigDecimal,
}

fn delivered(
    assignment_item: u128,
    evaluation: EvaluationState<'_>,
    score: Option<AutomatedScore>,
) -> DeliveredItem {
    DeliveredItem {
        assignment_item: AssignmentItemId::from_uuid(uuid(assignment_item)),
        reference: ProblemVersionRef {
            problem: ProblemId::from_uuid(uuid(assignment_item + 100)),
            version: VersionId::from_uuid(uuid(assignment_item + 200)),
        },
        run: uuid(5),
        completed: true,
        completion_millis: Some(6),
        status: Some(evaluation.status.to_string()),
        has_response: true,
        execution_state: Some(evaluation.execution_state.to_string()),
        grading_status: Some(evaluation.grading_status.to_string()),
        has_completion_receipt: evaluation.has_completion_receipt,
        credit: score.as_ref().map(|score| score.credit.clone()),
        correct: score.as_ref().map(|score| score.correct),
        earned_points: score.as_ref().map(|score| score.earned_points.clone()),
        possible_points: score.as_ref().map(|score| score.possible_points.clone()),
    }
}

fn graded() -> EvaluationState<'static> {
    EvaluationState {
        status: "submitted",
        execution_state: "completed",
        grading_status: "graded",
        has_completion_receipt: true,
    }
}

fn full_credit() -> AutomatedScore {
    AutomatedScore {
        credit: BigDecimal::from(1),
        correct: true,
        earned_points: BigDecimal::from(1),
        possible_points: BigDecimal::from(1),
    }
}

#[test]
fn assignment_average_accepts_authored_points_above_normalized_credit_range() {
    let report = course_report_from_deliveries(
        context(),
        vec![delivered(
            10,
            graded(),
            Some(AutomatedScore {
                credit: "0.8".parse().expect("valid credit"),
                correct: false,
                earned_points: BigDecimal::from(1_600),
                possible_points: BigDecimal::from(2_000),
            }),
        )],
    )
    .expect("large exact point values remain valid");

    assert_eq!(report.assignment_average_score, Some(0.8));
    assert_eq!(report.items[0].average_credit, Some(0.8));
}

#[test]
fn unscored_automated_evaluations_suppress_assignment_metrics_without_response_buckets() {
    let report = course_report_from_deliveries(
        context(),
        vec![
            delivered(10, graded(), Some(full_credit())),
            delivered(
                11,
                EvaluationState {
                    status: "submitted",
                    execution_state: "retry_wait",
                    grading_status: "automated_pending",
                    has_completion_receipt: false,
                },
                None,
            ),
        ],
    )
    .expect("pending automated item remains an aggregate flag");

    assert_eq!(report.assignment_average_score, None);
    assert!(report.incomplete_scoring);
    assert_eq!(report.items[1].unscored_attempt_count, 1);
    assert_eq!(report.items[1].response_distribution.unanswered, 0);
    assert_eq!(report.items[1].response_distribution.correct, 0);
    assert_eq!(report.items[1].response_distribution.partial, 0);
    assert_eq!(report.items[1].response_distribution.incorrect, 0);
}

#[test]
fn contradictory_automated_evaluation_tuples_fail_closed() {
    let result = course_report_from_deliveries(
        context(),
        vec![delivered(
            10,
            EvaluationState {
                has_completion_receipt: false,
                ..graded()
            },
            Some(full_credit()),
        )],
    );

    assert!(matches!(result, Err(StoreError::Unavailable(_))));
}

#[test]
fn automated_exception_and_answer_free_auto_submission_have_distinct_safe_aggregates() {
    let mut unanswered = delivered(
        11,
        EvaluationState {
            status: "auto_submitted",
            ..graded()
        },
        Some(full_credit()),
    );
    unanswered.has_response = false;
    let report = course_report_from_deliveries(
        context(),
        vec![
            delivered(
                10,
                EvaluationState {
                    status: "submitted",
                    execution_state: "exception",
                    grading_status: "automated_exception",
                    has_completion_receipt: false,
                },
                None,
            ),
            unanswered,
        ],
    )
    .expect("closed exception and force-submit shapes aggregate safely");

    assert!(report.incomplete_scoring);
    assert_eq!(report.assignment_average_score, None);
    assert_eq!(report.items[0].unscored_attempt_count, 1);
    assert_eq!(report.items[0].response_distribution.unanswered, 0);
    assert_eq!(report.items[1].unscored_attempt_count, 0);
    assert_eq!(report.items[1].response_distribution.unanswered, 1);
}
