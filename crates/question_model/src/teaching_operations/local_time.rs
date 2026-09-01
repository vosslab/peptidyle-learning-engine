//! Course-local conversion for teaching-operation modifier transport.

use crate::{
    AssignmentAuthoredContentField, AssignmentAuthoredContentLocalError, CourseLocalDateAndTime,
    CourseTerm, Timestamp,
};

use super::{AssignmentPolicySource, TeachingPreviewTimeField};

/// Projects a resolved server timestamp for an allowed teaching preview.
///
/// The preview wire contract contains only the resulting course-local wall
/// clock value. The server remains responsible for resolving policy and for
/// attaching the course's authoritative IANA zone to the allowed preview.
pub fn project_teaching_preview_time_field(
    value: Option<Timestamp>,
    source: AssignmentPolicySource,
    course_term: &CourseTerm,
    field: AssignmentAuthoredContentField,
) -> Result<TeachingPreviewTimeField, AssignmentAuthoredContentLocalError> {
    Ok(TeachingPreviewTimeField {
        value: value
            .map(|value| CourseLocalDateAndTime::from_activity_timestamp(value, course_term, field))
            .transpose()?,
        source,
    })
}

/// Resolves the local `set` payload used by the three teaching schedule fields.
///
/// Kept as a small convenience at the teaching-operations boundary so callers
/// cannot accidentally use a browser or machine-local time zone. Inherit and
/// unrestricted adjustment states intentionally remain the server's ordinary
/// policy mapping responsibility.
pub fn resolve_teaching_local_time(
    value: &CourseLocalDateAndTime,
    course_term: &CourseTerm,
    field: AssignmentAuthoredContentField,
) -> Result<Timestamp, AssignmentAuthoredContentLocalError> {
    value.resolve_for_course(course_term, field)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AccommodationAdjustmentView, AssignmentDeadlineRule, LateWorkRule,
        TeachingAssignmentAttemptTimeLimitFieldPatch, TeachingAssignmentStartDecision,
        TeachingAttemptLimitFieldPatch, TeachingDisplayLabel,
        TeachingPreviewAssignmentDeadlineRuleField, TeachingPreviewLateWorkRuleField,
        TeachingPreviewLimitField, TeachingPreviewView, TeachingStudentLateWorkStatus,
        TeachingTimeFieldPatch,
    };
    use chrono::{TimeZone, Utc};

    fn local(value: &str) -> CourseLocalDateAndTime {
        CourseLocalDateAndTime::parse(value).expect("valid exact local time")
    }

    fn chicago_term() -> CourseTerm {
        CourseTerm::from_parts("2026-01-01", "2026-12-31", "America/Chicago")
            .expect("valid course term")
    }

    fn base_source() -> AssignmentPolicySource {
        AssignmentPolicySource::Base {
            label: TeachingDisplayLabel::try_from("Assignment policy".to_owned())
                .expect("valid display label"),
        }
    }

    #[test]
    fn teaching_time_patch_uses_exact_local_wire_values_not_epochs() {
        let adjustment = AccommodationAdjustmentView {
            available_at: TeachingTimeFieldPatch::Set {
                value: local("2026-09-01T10:04:05.123"),
            },
            due_at: TeachingTimeFieldPatch::Inherit,
            closes_at: TeachingTimeFieldPatch::Unrestricted,
            assignment_attempt_time_limit_seconds:
                TeachingAssignmentAttemptTimeLimitFieldPatch::Inherit,
            attempt_limit: TeachingAttemptLimitFieldPatch::Inherit,
        };
        let value = serde_json::to_value(&adjustment).expect("time adjustment serializes");
        assert_eq!(
            value["availableAt"],
            serde_json::json!({"kind":"set","value":"2026-09-01T10:04:05.123"})
        );
        assert!(serde_json::from_value::<AccommodationAdjustmentView>(value).is_ok());
        for invalid in [
            r#"{"availableAt":{"kind":"set","value":1788275045123},"dueAt":{"kind":"inherit"},"closesAt":{"kind":"inherit"},"assignmentAttemptTimeLimitSeconds":{"kind":"inherit"},"attemptLimit":{"kind":"inherit"}}"#,
            r#"{"availableAt":{"kind":"set","value":"2026-09-01T10:04"},"dueAt":{"kind":"inherit"},"closesAt":{"kind":"inherit"},"assignmentAttemptTimeLimitSeconds":{"kind":"inherit"},"attemptLimit":{"kind":"inherit"}}"#,
        ] {
            assert!(serde_json::from_str::<AccommodationAdjustmentView>(invalid).is_err());
        }
    }

    #[test]
    fn teaching_local_conversion_refuses_dst_and_term_escapes_by_field() {
        let term = chicago_term();
        assert_eq!(
            resolve_teaching_local_time(
                &local("2026-03-08T02:30:00.000"),
                &term,
                AssignmentAuthoredContentField::AvailableAt,
            ),
            Err(AssignmentAuthoredContentLocalError::NonexistentLocalTime(
                AssignmentAuthoredContentField::AvailableAt
            ))
        );
        assert_eq!(
            resolve_teaching_local_time(
                &local("2026-11-01T01:30:00.000"),
                &term,
                AssignmentAuthoredContentField::DueAt,
            ),
            Err(AssignmentAuthoredContentLocalError::AmbiguousLocalTime(
                AssignmentAuthoredContentField::DueAt
            ))
        );
        assert_eq!(
            resolve_teaching_local_time(
                &local("2027-01-01T10:00:00.000"),
                &term,
                AssignmentAuthoredContentField::ClosesAt,
            ),
            Err(AssignmentAuthoredContentLocalError::OutsideCourseTerm(
                AssignmentAuthoredContentField::ClosesAt
            ))
        );
    }

    #[test]
    fn allowed_preview_carries_course_zone_and_exact_local_projections() {
        let term = chicago_term();
        let timestamp = Timestamp::from_unix_millis(
            Utc.with_ymd_and_hms(2026, 9, 1, 15, 4, 5)
                .single()
                .expect("valid UTC time")
                .timestamp_millis()
                + 123,
        );
        let available_at = project_teaching_preview_time_field(
            Some(timestamp),
            base_source(),
            &term,
            AssignmentAuthoredContentField::AvailableAt,
        )
        .expect("exact course-local projection");
        assert_eq!(
            available_at
                .value
                .as_ref()
                .map(CourseLocalDateAndTime::as_str),
            Some("2026-09-01T10:04:05.123")
        );
        let preview = TeachingPreviewView::Allowed {
            time_zone: term.time_zone().clone(),
            start: TeachingAssignmentStartDecision::MayStart {
                late: TeachingStudentLateWorkStatus::OnTime,
            },
            available_at,
            due_at: project_teaching_preview_time_field(
                None,
                base_source(),
                &term,
                AssignmentAuthoredContentField::DueAt,
            )
            .expect("empty projection"),
            closes_at: project_teaching_preview_time_field(
                None,
                base_source(),
                &term,
                AssignmentAuthoredContentField::ClosesAt,
            )
            .expect("empty projection"),
            assignment_attempt_time_limit_seconds: TeachingPreviewLimitField {
                value: None,
                source: base_source(),
            },
            attempt_limit: TeachingPreviewLimitField {
                value: None,
                source: base_source(),
            },
            late_work_rule: TeachingPreviewLateWorkRuleField {
                value: LateWorkRule::Accept,
                source: base_source(),
            },
            assignment_deadline_rule: TeachingPreviewAssignmentDeadlineRuleField {
                value: AssignmentDeadlineRule::AutoSubmit,
                source: base_source(),
            },
        };
        let value = serde_json::to_value(preview).expect("allowed preview serializes");
        assert_eq!(value["timeZone"], "America/Chicago");
        assert_eq!(value["availableAt"]["value"], "2026-09-01T10:04:05.123");
    }
}
