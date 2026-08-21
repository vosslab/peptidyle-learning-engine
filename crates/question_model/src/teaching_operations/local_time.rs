//! Course-local conversion for teaching-operation modifier transport.

use crate::{
    ActivityTimestamp, AssignmentTeachingSettingsField, AssignmentTeachingSettingsLocalError,
    CourseLocalDateTime, CourseTerm,
};

use super::{TeachingPreviewFieldSource, TeachingPreviewTimeField};

/// Projects a resolved server timestamp for an allowed teaching preview.
///
/// The preview wire contract contains only the resulting course-local wall
/// clock value. The server remains responsible for resolving policy and for
/// attaching the course's authoritative IANA zone to the allowed preview.
pub fn project_teaching_preview_time_field(
    value: Option<ActivityTimestamp>,
    source: TeachingPreviewFieldSource,
    course_term: &CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<TeachingPreviewTimeField, AssignmentTeachingSettingsLocalError> {
    Ok(TeachingPreviewTimeField {
        value: value
            .map(|value| CourseLocalDateTime::from_activity_timestamp(value, course_term, field))
            .transpose()?,
        source,
    })
}

/// Resolves the local `set` payload used by the three teaching schedule fields.
///
/// Kept as a small convenience at the teaching-operations boundary so callers
/// cannot accidentally use a browser or machine-local time zone. Inherit and
/// unrestricted patch states intentionally remain the server's ordinary
/// policy mapping responsibility.
pub fn resolve_teaching_local_time(
    value: &CourseLocalDateTime,
    course_term: &CourseTerm,
    field: AssignmentTeachingSettingsField,
) -> Result<ActivityTimestamp, AssignmentTeachingSettingsLocalError> {
    value.resolve_for_course(course_term, field)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AssignmentDeadlineBehavior, LateSubmissionPolicy, PolicyPatchView,
        TeachingAttemptLimitFieldPatch, TeachingDisplayLabel, TeachingLateVerdict,
        TeachingLimitFieldPatch, TeachingPreviewDeadlineBehaviorField,
        TeachingPreviewLateSubmissionField, TeachingPreviewLimitField, TeachingPreviewView,
        TeachingStartVerdict, TeachingTimeFieldPatch,
    };
    use chrono::{TimeZone, Utc};

    fn local(value: &str) -> CourseLocalDateTime {
        CourseLocalDateTime::parse(value).expect("valid exact local time")
    }

    fn chicago_term() -> CourseTerm {
        CourseTerm::from_parts("2026-01-01", "2026-12-31", "America/Chicago")
            .expect("valid course term")
    }

    fn base_source() -> TeachingPreviewFieldSource {
        TeachingPreviewFieldSource::Base {
            label: TeachingDisplayLabel::try_from("Assignment policy".to_owned())
                .expect("valid display label"),
        }
    }

    #[test]
    fn teaching_time_patch_uses_exact_local_wire_values_not_epochs() {
        let patch = PolicyPatchView {
            available_at: TeachingTimeFieldPatch::Set {
                value: local("2026-09-01T10:04:05.123"),
            },
            due_at: TeachingTimeFieldPatch::Inherit,
            closes_at: TeachingTimeFieldPatch::Unrestricted,
            time_limit_seconds: TeachingLimitFieldPatch::Inherit,
            attempt_limit: TeachingAttemptLimitFieldPatch::Inherit,
        };
        let value = serde_json::to_value(&patch).expect("time patch serializes");
        assert_eq!(
            value["availableAt"],
            serde_json::json!({"kind":"set","value":"2026-09-01T10:04:05.123"})
        );
        assert!(serde_json::from_value::<PolicyPatchView>(value).is_ok());
        for invalid in [
            r#"{"availableAt":{"kind":"set","value":1788275045123},"dueAt":{"kind":"inherit"},"closesAt":{"kind":"inherit"},"timeLimitSeconds":{"kind":"inherit"},"attemptLimit":{"kind":"inherit"}}"#,
            r#"{"availableAt":{"kind":"set","value":"2026-09-01T10:04"},"dueAt":{"kind":"inherit"},"closesAt":{"kind":"inherit"},"timeLimitSeconds":{"kind":"inherit"},"attemptLimit":{"kind":"inherit"}}"#,
        ] {
            assert!(serde_json::from_str::<PolicyPatchView>(invalid).is_err());
        }
    }

    #[test]
    fn teaching_local_conversion_refuses_dst_and_term_escapes_by_field() {
        let term = chicago_term();
        assert_eq!(
            resolve_teaching_local_time(
                &local("2026-03-08T02:30:00.000"),
                &term,
                AssignmentTeachingSettingsField::AvailableAt,
            ),
            Err(AssignmentTeachingSettingsLocalError::NonexistentLocalTime(
                AssignmentTeachingSettingsField::AvailableAt
            ))
        );
        assert_eq!(
            resolve_teaching_local_time(
                &local("2026-11-01T01:30:00.000"),
                &term,
                AssignmentTeachingSettingsField::DueAt,
            ),
            Err(AssignmentTeachingSettingsLocalError::AmbiguousLocalTime(
                AssignmentTeachingSettingsField::DueAt
            ))
        );
        assert_eq!(
            resolve_teaching_local_time(
                &local("2027-01-01T10:00:00.000"),
                &term,
                AssignmentTeachingSettingsField::ClosesAt,
            ),
            Err(AssignmentTeachingSettingsLocalError::OutsideCourseTerm(
                AssignmentTeachingSettingsField::ClosesAt
            ))
        );
    }

    #[test]
    fn allowed_preview_carries_course_zone_and_exact_local_projections() {
        let term = chicago_term();
        let timestamp = ActivityTimestamp::from_unix_millis(
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
            AssignmentTeachingSettingsField::AvailableAt,
        )
        .expect("exact course-local projection");
        assert_eq!(
            available_at.value.as_ref().map(CourseLocalDateTime::as_str),
            Some("2026-09-01T10:04:05.123")
        );
        let preview = TeachingPreviewView::Allowed {
            time_zone: term.time_zone().clone(),
            start: TeachingStartVerdict::MayStart {
                late: TeachingLateVerdict::OnTime,
            },
            available_at,
            due_at: project_teaching_preview_time_field(
                None,
                base_source(),
                &term,
                AssignmentTeachingSettingsField::DueAt,
            )
            .expect("empty projection"),
            closes_at: project_teaching_preview_time_field(
                None,
                base_source(),
                &term,
                AssignmentTeachingSettingsField::ClosesAt,
            )
            .expect("empty projection"),
            time_limit_seconds: TeachingPreviewLimitField {
                value: None,
                source: base_source(),
            },
            attempt_limit: TeachingPreviewLimitField {
                value: None,
                source: base_source(),
            },
            late_submission: TeachingPreviewLateSubmissionField {
                value: LateSubmissionPolicy::Accept,
                source: base_source(),
            },
            deadline_behavior: TeachingPreviewDeadlineBehaviorField {
                value: AssignmentDeadlineBehavior::AutoSubmit,
                source: base_source(),
            },
        };
        let value = serde_json::to_value(preview).expect("allowed preview serializes");
        assert_eq!(value["timeZone"], "America/Chicago");
        assert_eq!(value["availableAt"]["value"], "2026-09-01T10:04:05.123");
    }
}
