use super::*;

/// Decodes the five assignment-owned Student disclosure timings from their
/// normalized PostgreSQL columns. Every field is mandatory and malformed
/// stored values fail closed through the shared timing parser.
#[cfg(feature = "postgres")]
pub(super) fn decode_student_disclosure_policy(
    row: &PgRow,
) -> Result<question_model::StudentDisclosurePolicy, StoreError> {
    let score: String = row.try_get("score_disclosure").map_err(map_sqlx_error)?;
    let per_item_correctness: String = row
        .try_get("per_item_correctness_disclosure")
        .map_err(map_sqlx_error)?;
    let feedback_text: String = row
        .try_get("feedback_text_disclosure")
        .map_err(map_sqlx_error)?;
    let solution: String = row.try_get("solution_disclosure").map_err(map_sqlx_error)?;
    let class_statistics: String = row
        .try_get("class_statistics_disclosure")
        .map_err(map_sqlx_error)?;

    Ok(question_model::StudentDisclosurePolicy {
        score: parse_student_disclosure_timing(&score)?,
        per_item_correctness: parse_student_disclosure_timing(&per_item_correctness)?,
        feedback_text: parse_student_disclosure_timing(&feedback_text)?,
        solution: parse_student_disclosure_timing(&solution)?,
        class_statistics: parse_student_disclosure_timing(&class_statistics)?,
    })
}
