use super::*;

#[cfg(feature = "postgres")]
pub(super) struct FeedbackColumns {
    pub(super) hint: Option<Value>,
    pub(super) correct_response: Option<Value>,
    pub(super) rationale: Option<Value>,
}

#[cfg(feature = "postgres")]
pub(super) fn encode_feedback_columns(
    content: &FeedbackContent,
) -> Result<FeedbackColumns, StoreError> {
    fn field(value: Option<&Vec<ContentBlock>>) -> Result<Option<Value>, StoreError> {
        value
            .map(|blocks| {
                serde_json::to_value(blocks).map_err(|error| {
                    StoreError::InvalidRecord(format!("feedback encoding failed: {error}"))
                })
            })
            .transpose()
    }
    Ok(FeedbackColumns {
        hint: field(content.hint.as_ref())?,
        correct_response: field(content.correct_response.as_ref())?,
        rationale: field(content.rationale.as_ref())?,
    })
}

#[cfg(feature = "postgres")]
pub(super) async fn load_attempt_feedback(
    transaction: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    attempt: QuestionAttemptId,
) -> Result<AttemptFeedbackRecord, StoreError> {
    let row = sqlx::query(
        "SELECT hint, correct_response, rationale, content_sha256 \
         FROM attempt_feedback WHERE tenant_id = $1 AND attempt_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(attempt.as_uuid())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?
    .ok_or_else(|| {
        StoreError::InvalidRecord("submission is missing private feedback".to_string())
    })?;
    fn field(row: &PgRow, name: &str) -> Result<Option<Vec<ContentBlock>>, StoreError> {
        let value: Option<Value> = row.try_get(name).map_err(map_sqlx_error)?;
        value
            .map(|value| {
                serde_json::from_value(value).map_err(|error| {
                    StoreError::InvalidRecord(format!("stored feedback decode failed: {error}"))
                })
            })
            .transpose()
    }
    let content = FeedbackContent {
        hint: field(&row, "hint")?,
        correct_response: field(&row, "correct_response")?,
        rationale: field(&row, "rationale")?,
    };
    let feedback = private_feedback_record(content)?;
    let stored_digest: String = row.try_get("content_sha256").map_err(map_sqlx_error)?;
    if stored_digest != feedback.content_sha256().to_string() {
        return Err(StoreError::InvalidRecord(
            "stored feedback digest mismatch".to_string(),
        ));
    }
    Ok(feedback)
}
