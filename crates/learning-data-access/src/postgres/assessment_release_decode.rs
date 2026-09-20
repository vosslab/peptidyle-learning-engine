//! Decode and parse helpers for Assessment Workspace rows.

use question_model::{
    AssessmentAuthoredContentField, AssessmentEditNumber, AssessmentEntry,
    AssessmentEntryAvailability, AssessmentEntryId, AssessmentEntryScoringRule, AssessmentId,
    AssessmentInstructions, AssessmentOrigin, AssessmentPointValue, AssessmentStatus,
    AssessmentTitle, AssessmentType, BlueprintAssessmentId, BlueprintAssessmentSource,
    BlueprintCourseId, BlueprintRevisionNumber, BlueprintRevisionTuple, CourseInstanceId,
    FixedQuestionAssessmentEntry, LateWorkRule, LocalDateAndTime, QuestionAttemptLimit,
    QuestionAttemptTimeLimit, QuestionId, QuestionPoolAssessmentEntry, QuestionPoolEditNumber,
    QuestionPoolSelectedQuestionOrder, QuestionPoolSelectionRule, QuestionRevisionNumber,
    QuestionRevisionTuple, Timestamp,
};
use sqlx::Row;

use super::super::connection::map_sqlx_error;
use super::AssessmentScheduleContext;
use crate::StoreError;

pub(super) fn question_id(value: String) -> Result<QuestionId, StoreError> {
    value.parse().map_err(|_| invalid("Question ID"))
}

pub(super) fn question_revision_tuple(
    question: String,
    revision_number: i32,
) -> Result<QuestionRevisionTuple, StoreError> {
    Ok(QuestionRevisionTuple {
        question_id: question_id(question)?,
        revision_number: QuestionRevisionNumber::new(
            u32::try_from(revision_number).map_err(|_| invalid("Question Revision Number"))?,
        )
        .map_err(|_| invalid("Question Revision Number"))?,
    })
}
#[rustfmt::skip]
pub(super) fn decode_entries(rows: &[sqlx::postgres::PgRow]) -> Result<Vec<AssessmentEntry>, StoreError> {
    let mut entries = Vec::new(); let mut index = 0;
    while let Some(row) = rows.get(index) {
        let Some(raw_id) = row.try_get::<Option<uuid::Uuid>, _>("assessment_entry_id").map_err(map_sqlx_error)? else { break };
        let id = AssessmentEntryId::from_uuid(raw_id);
        let kind: String = row.try_get("entry_kind").map_err(map_sqlx_error)?;
        let availability = parse_entry_availability(row.try_get("entry_availability").map_err(map_sqlx_error)?)?;
        let scoring_rule = entry_scoring_rule(row.try_get("scoring_rule").map_err(map_sqlx_error)?)?;
        let policy = question_policy(row)?;
        if kind == "fixed_question" {
            entries.push(AssessmentEntry::FixedQuestion(FixedQuestionAssessmentEntry {
                id, question_revision_tuple: row_question_revision_tuple(row)?, points_possible: point_value(row, "points_possible")?, availability, scoring_rule,
                question_attempt_limit: policy.0, question_attempt_time_limit: policy.1,
            })); index += 1; continue;
        }
        if kind != "question_pool" { return Err(invalid("Assessment Entry kind")); }
        let selection_count = u32::try_from(row.try_get::<i32, _>("selection_count").map_err(map_sqlx_error)?).map_err(|_| invalid("Question Pool selection count"))?;
        let selection_rule = QuestionPoolSelectionRule { selected_question_order: selected_question_order_from_row(row.try_get("selected_question_order").map_err(map_sqlx_error)?)? };
        let question_pool_id = question_id(row.try_get("question_pool_id").map_err(map_sqlx_error)?)?;
        let question_pool_edit_number = QuestionPoolEditNumber::new(
            u64::try_from(row.try_get::<i64, _>("question_pool_edit_number").map_err(map_sqlx_error)?)
                .map_err(|_| invalid("Question Pool Edit Number"))?,
        ).map_err(|_| invalid("Question Pool Edit Number"))?;
        entries.push(AssessmentEntry::QuestionPool(QuestionPoolAssessmentEntry {
            id, question_pool_id, question_pool_edit_number, availability, scoring_rule,
            selection_count: std::num::NonZeroU32::new(selection_count)
                .ok_or_else(|| invalid("Question Pool selection count"))?,
            points_per_item: point_value(row, "points_per_item")?, selection_rule,
            question_attempt_limit: policy.0, question_attempt_time_limit: policy.1,
        }));
        while let Some(member_row) = rows.get(index) {
            let member_entry_id = member_row
                .try_get::<Option<uuid::Uuid>, _>("assessment_entry_id")
                .map_err(map_sqlx_error)?;
            if member_entry_id != Some(raw_id) {
                break;
            }
            index += 1;
        }
    }
    Ok(entries)
}
#[rustfmt::skip]
pub(super) fn row_question_revision_tuple(row: &sqlx::postgres::PgRow) -> Result<QuestionRevisionTuple, StoreError> {
    question_revision_tuple(row.try_get("question_id").map_err(map_sqlx_error)?, row.try_get("question_revision_number").map_err(map_sqlx_error)?)
}

#[rustfmt::skip]
pub(super) fn point_value(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<AssessmentPointValue, StoreError> {
    row.try_get::<bigdecimal::BigDecimal, _>(column).map_err(map_sqlx_error)?.to_string().parse().map_err(|_| invalid("Assessment point value"))
}

#[rustfmt::skip]
pub(super) fn question_policy(
    row: &sqlx::postgres::PgRow,
) -> Result<(QuestionAttemptLimit, QuestionAttemptTimeLimit), StoreError> {
    let limit = row.try_get::<Option<i32>, _>("question_attempt_limit").map_err(map_sqlx_error)?.map(|v| u32::try_from(v).map_err(|_| invalid("Question Attempt Limit"))).transpose()?;
    let time = match row.try_get::<Option<i32>, _>("question_attempt_time_limit_seconds").map_err(map_sqlx_error)? { None => QuestionAttemptTimeLimit::Unlimited, Some(v) => QuestionAttemptTimeLimit::Limited { seconds: u32::try_from(v).map_err(|_| invalid("Question Attempt Time Limit"))?, grace_seconds: u32::try_from(row.try_get::<Option<i32>, _>("question_attempt_grace_seconds").map_err(map_sqlx_error)?.ok_or_else(|| invalid("Question Attempt Grace"))?).map_err(|_| invalid("Question Attempt Grace"))? } };
    Ok((QuestionAttemptLimit { max_attempts: limit }, time))
}

#[rustfmt::skip]
pub(super) fn parse_entry_availability(value: String) -> Result<AssessmentEntryAvailability, StoreError> {
    match value.as_str() { "available" => Ok(AssessmentEntryAvailability::Available), "retired" => Ok(AssessmentEntryAvailability::Retired), _ => Err(invalid("Assessment Entry availability")) }
}
pub(super) fn assessment_type(value: String) -> Result<AssessmentType, StoreError> {
    AssessmentType::parse(&value).ok_or_else(|| invalid("Assessment Type"))
}
#[rustfmt::skip]
pub(super) fn entry_scoring_rule(value: String) -> Result<AssessmentEntryScoringRule, StoreError> {
    match value.as_str() { "normal" => Ok(AssessmentEntryScoringRule::Normal), "full_credit" => Ok(AssessmentEntryScoringRule::FullCredit), "extra_credit" => Ok(AssessmentEntryScoringRule::ExtraCredit), "excluded" => Ok(AssessmentEntryScoringRule::Excluded), _ => Err(invalid("Assessment Entry scoring rule")) }
}
#[rustfmt::skip]
pub(super) fn selected_question_order_from_row(value: String) -> Result<QuestionPoolSelectedQuestionOrder, StoreError> {
    match value.as_str() { "question_pool_order" => Ok(QuestionPoolSelectedQuestionOrder::QuestionPoolOrder), "random_order" => Ok(QuestionPoolSelectedQuestionOrder::RandomOrder), _ => Err(invalid("Question Pool selected question order")) }
}

pub(super) fn assessment_origin(
    row: &sqlx::postgres::PgRow,
) -> Result<AssessmentOrigin, StoreError> {
    // ASVS 2.2.1 and 2.2.3: accept only the two complete persisted origin shapes.
    let kind: String = row.try_get("origin_kind").map_err(map_sqlx_error)?;
    let course_id: Option<String> = row
        .try_get("source_blueprint_course_id")
        .map_err(map_sqlx_error)?;
    let revision: Option<i64> = row
        .try_get("source_blueprint_revision_number")
        .map_err(map_sqlx_error)?;
    let assessment_id: Option<uuid::Uuid> = row
        .try_get("source_blueprint_assessment_id")
        .map_err(map_sqlx_error)?;
    match (kind.as_str(), course_id, revision, assessment_id) {
        ("direct", None, None, None) => Ok(AssessmentOrigin::Direct),
        ("adopted", Some(course_id), Some(revision), Some(assessment_id)) => {
            let course_id =
                BlueprintCourseId::new(course_id).map_err(|_| invalid("Assessment Origin"))?;
            let revision = u64::try_from(revision)
                .ok()
                .and_then(BlueprintRevisionNumber::new)
                .ok_or_else(|| invalid("Assessment Origin"))?;
            Ok(AssessmentOrigin::Adopted {
                source: BlueprintAssessmentSource::new(
                    BlueprintRevisionTuple {
                        blueprint_course_id: course_id,
                        revision_number: revision,
                    },
                    BlueprintAssessmentId::from_uuid(assessment_id),
                ),
            })
        }
        _ => Err(invalid("Assessment Origin")),
    }
}

pub(super) fn local_timestamp_from_row(
    row: &sqlx::postgres::PgRow,
    column: &str,
    context: &AssessmentScheduleContext,
    field: AssessmentAuthoredContentField,
) -> Result<Option<LocalDateAndTime>, StoreError> {
    row.try_get::<Option<i64>, _>(column)
        .map_err(map_sqlx_error)?
        .map(|millis| {
            LocalDateAndTime::from_activity_timestamp_in_account_time_zone(
                Timestamp::from_unix_millis(millis),
                &context.term,
                &context.account_time_zone,
                field,
            )
            .map_err(|_| invalid("Account-local Assessment time"))
        })
        .transpose()
}
pub(in crate::postgres) fn assessment_id(value: String) -> Result<AssessmentId, StoreError> {
    AssessmentId::new(value).map_err(|_| invalid("Assessment ID"))
}
pub(super) fn course_instance_id(value: String) -> Result<CourseInstanceId, StoreError> {
    CourseInstanceId::new(value).map_err(|_| invalid("Course Instance ID"))
}
pub(super) fn course_name(value: String) -> Result<String, StoreError> {
    if value.is_empty() || value != value.trim() || value.chars().count() > 200 {
        return Err(invalid("Course Name"));
    }
    Ok(value)
}
pub(super) fn edit(value: i64) -> Result<AssessmentEditNumber, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(AssessmentEditNumber::new)
        .ok_or_else(|| invalid("Assessment Edit Number"))
}
pub(super) fn title(value: String) -> Result<AssessmentTitle, StoreError> {
    value.try_into().map_err(|_| invalid("Assessment Title"))
}
pub(super) fn instructions(value: String) -> Result<AssessmentInstructions, StoreError> {
    value
        .try_into()
        .map_err(|_| invalid("Assessment Instructions"))
}
pub(super) fn status(value: String) -> Result<AssessmentStatus, StoreError> {
    match value.as_str() {
        "unreleased" => Ok(AssessmentStatus::Unreleased),
        "released" => Ok(AssessmentStatus::Released),
        "closed" => Ok(AssessmentStatus::Closed),
        "archived" => Ok(AssessmentStatus::Archived),
        _ => Err(invalid("Assessment Status")),
    }
}
pub(super) fn parse_late_work_rule(value: String) -> Result<LateWorkRule, StoreError> {
    match value.as_str() {
        "accept" => Ok(LateWorkRule::Accept),
        "mark_late" => Ok(LateWorkRule::MarkLate),
        "reject" => Ok(LateWorkRule::Reject),
        _ => Err(invalid("Late Work Rule")),
    }
}
pub(in crate::postgres) fn late_work_rule(value: &LateWorkRule) -> &'static str {
    match value {
        LateWorkRule::Accept => "accept",
        LateWorkRule::MarkLate => "mark_late",
        LateWorkRule::Reject => "reject",
    }
}
pub(super) fn nonzero_optional(
    value: Option<i32>,
    label: &str,
) -> Result<Option<std::num::NonZeroU32>, StoreError> {
    value
        .map(|value| {
            u32::try_from(value)
                .ok()
                .and_then(std::num::NonZeroU32::new)
                .ok_or_else(|| invalid(label))
        })
        .transpose()
}
pub(super) fn resolve_local_timestamp(
    value: Option<&LocalDateAndTime>,
    context: &AssessmentScheduleContext,
    field: AssessmentAuthoredContentField,
) -> Result<Option<i64>, StoreError> {
    value
        .map(|value| {
            value
                .resolve_in_account_time_zone(&context.term, &context.account_time_zone, field)
                .map(|timestamp| timestamp.as_unix_millis())
                .map_err(|_| invalid("Account-local Assessment time"))
        })
        .transpose()
}

pub(in crate::postgres) fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}
pub(super) fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Assessment Workspace UUID randomness unavailable".to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::question_revision_tuple;

    #[test]
    fn question_revision_tuple_takes_a_revision_number() {
        let tuple = question_revision_tuple("ABCD-XEFG".into(), 4).expect("tuple");
        assert_eq!(tuple.revision_number.get(), 4);
    }
}
