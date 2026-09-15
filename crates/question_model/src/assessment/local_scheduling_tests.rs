use std::num::NonZeroU32;

use chrono::{TimeZone, Utc};

use super::*;
use crate::CourseTerm;

fn course_term() -> CourseTerm {
    CourseTerm::from_parts("2026-01-01", "2026-12-31").expect("valid Course term")
}

fn local(value: &str) -> LocalDateAndTime {
    LocalDateAndTime::parse(value).expect("valid local wall-clock value")
}

fn local_settings(
    available_at: Option<LocalDateAndTime>,
    due_at: Option<LocalDateAndTime>,
    closes_at: Option<LocalDateAndTime>,
) -> InstructorAssessmentAuthoredContentLocal {
    InstructorAssessmentAuthoredContentLocal::new(
        AssessmentInstructions::try_new("Read the diagram.".to_string())
            .expect("valid instructions"),
        available_at,
        due_at,
        closes_at,
        NonZeroU32::new(900),
        NonZeroU32::new(2),
        LateWorkRule::MarkLate,
    )
    .expect("valid local settings")
}

#[test]
fn local_assessment_authored_content_round_trips_exact_milliseconds() {
    let utc_zone = crate::AccountTimeZone::parse("UTC").expect("UTC Account zone");
    let chicago_zone =
        crate::AccountTimeZone::parse("America/Chicago").expect("Chicago Account zone");
    let timestamp = Timestamp::from_unix_millis(
        Utc.with_ymd_and_hms(2026, 9, 1, 15, 4, 5)
            .single()
            .expect("valid UTC time")
            .timestamp_millis()
            + 123,
    );
    let settings = AssessmentAuthoredContent {
        instructions: AssessmentInstructions::try_new("Read the diagram.".to_string())
            .expect("valid instructions"),
        base_policy: BaseAssessmentPolicy {
            available_at: Some(timestamp),
            due_at: Some(timestamp),
            closes_at: Some(timestamp),
            assessment_attempt_time_limit_seconds: NonZeroU32::new(900),
            attempt_limit: NonZeroU32::new(2),
            late_work_rule: LateWorkRule::MarkLate,
        },
        activity_rules: AssessmentActivityRules {
            assessment_attempt_resume_rule: crate::AssessmentAttemptResumeRule::SingleSession,
            assessment_question_display_rule:
                crate::AssessmentQuestionDisplayRule::OneQuestionAtATime,
            assessment_navigation_rule: crate::AssessmentNavigationRule::ForwardOnly,
            assessment_question_order_rule: crate::AssessmentQuestionOrderRule::Shuffled,
            ..AssessmentActivityRules::default()
        },
    };
    let term = course_term();
    let utc = InstructorAssessmentAuthoredContentLocal::from_absolute(&term, &utc_zone, &settings)
        .expect("UTC projection");
    assert_eq!(
        utc.available_at.as_ref().map(LocalDateAndTime::as_str),
        Some("2026-09-01T15:04:05.123")
    );
    assert_eq!(
        utc.into_absolute(&term, &utc_zone, settings.activity_rules)
            .expect("UTC resolution"),
        settings
    );

    let chicago =
        InstructorAssessmentAuthoredContentLocal::from_absolute(&term, &chicago_zone, &settings)
            .expect("Chicago projection");
    assert_eq!(
        chicago.available_at.as_ref().map(LocalDateAndTime::as_str),
        Some("2026-09-01T10:04:05.123")
    );
    assert_eq!(
        chicago
            .into_absolute(&term, &chicago_zone, settings.activity_rules)
            .expect("Chicago resolution"),
        settings
    );
}

#[test]
fn local_assessment_authored_content_refuses_dst_gap_and_ambiguity() {
    let term = course_term();
    let account_zone =
        crate::AccountTimeZone::parse("America/Chicago").expect("authenticated Account zone");
    let gap = local_settings(Some(local("2026-03-08T02:30:00.000")), None, None);
    assert_eq!(
        gap.into_absolute(&term, &account_zone, AssessmentActivityRules::default()),
        Err(AssessmentAuthoredContentLocalError::NonexistentLocalTime(
            AssessmentAuthoredContentField::AvailableAt
        ))
    );
    let ambiguity = local_settings(Some(local("2026-11-01T01:30:00.000")), None, None);
    assert_eq!(
        ambiguity.into_absolute(&term, &account_zone, AssessmentActivityRules::default()),
        Err(AssessmentAuthoredContentLocalError::AmbiguousLocalTime(
            AssessmentAuthoredContentField::AvailableAt
        ))
    );
}

#[test]
fn zone_free_local_input_resolves_in_the_authenticated_account_zone() {
    let local = LocalDateAndTime::parse("2026-09-01T10:00:00.000").expect("local input");
    let term = course_term();
    let account_zone =
        crate::AccountTimeZone::parse("America/New_York").expect("authenticated Account zone");
    let resolved = local
        .resolve_in_account_time_zone(&term, &account_zone, AssessmentAuthoredContentField::DueAt)
        .expect("Account-zone resolution");
    assert_eq!(resolved.as_unix_millis(), 1_788_271_200_000);
    assert_eq!(
        LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
            resolved,
            &term,
            &account_zone,
            AssessmentAuthoredContentField::DueAt,
        )
        .expect("Account-zone projection")
        .as_str(),
        "2026-09-01T10:00:00.000"
    );
}

#[test]
fn account_zone_resolver_refuses_bounds_and_dst_without_changing_local_order() {
    let term = course_term();
    let account_zone =
        crate::AccountTimeZone::parse("America/New_York").expect("authenticated Account zone");
    for (value, expected) in [
        (
            "2025-12-31T23:59:59.999",
            AssessmentAuthoredContentLocalError::OutsideCourseTerm(
                AssessmentAuthoredContentField::DueAt,
            ),
        ),
        (
            "2026-03-08T02:30:00.000",
            AssessmentAuthoredContentLocalError::NonexistentLocalTime(
                AssessmentAuthoredContentField::DueAt,
            ),
        ),
        (
            "2026-11-01T01:30:00.000",
            AssessmentAuthoredContentLocalError::AmbiguousLocalTime(
                AssessmentAuthoredContentField::DueAt,
            ),
        ),
    ] {
        assert_eq!(
            LocalDateAndTime::parse(value)
                .expect("exact local time")
                .resolve_in_account_time_zone(
                    &term,
                    &account_zone,
                    AssessmentAuthoredContentField::DueAt,
                ),
            Err(expected),
            "{value}",
        );
    }

    let available = LocalDateAndTime::parse("2026-09-01T09:00:00.000").expect("available");
    let due = LocalDateAndTime::parse("2026-09-01T10:00:00.000").expect("due");
    assert!(available < due, "local wall-clock order remains explicit");
    let available_at = available
        .resolve_in_account_time_zone(
            &term,
            &account_zone,
            AssessmentAuthoredContentField::AvailableAt,
        )
        .expect("available Account-zone resolution");
    let due_at = due
        .resolve_in_account_time_zone(&term, &account_zone, AssessmentAuthoredContentField::DueAt)
        .expect("due Account-zone resolution");
    assert!(
        available_at < due_at,
        "resolved instants preserve local order"
    );

    let chicago = crate::AccountTimeZone::parse("America/Chicago").expect("Account zone");
    for instant in [
        Utc.with_ymd_and_hms(2026, 11, 1, 6, 30, 0)
            .single()
            .expect("first fall-back instant"),
        Utc.with_ymd_and_hms(2026, 11, 1, 7, 30, 0)
            .single()
            .expect("second fall-back instant"),
    ] {
        assert_eq!(
            LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                Timestamp::from_unix_millis(instant.timestamp_millis()),
                &term,
                &chicago,
                AssessmentAuthoredContentField::DueAt,
            )
            .expect("stored fall-back instant remains renderable")
            .as_str(),
            "2026-11-01T01:30:00.000",
        );
    }
}
