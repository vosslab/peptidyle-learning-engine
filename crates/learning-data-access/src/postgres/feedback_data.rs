use super::*;

#[cfg(feature = "postgres")]
pub(super) struct FeedbackColumns {
    pub(super) hint: Option<Value>,
    pub(super) correct_response: Option<Value>,
    pub(super) rationale: Option<Value>,
}

#[cfg(feature = "postgres")]
type FeedbackTuple = (
    Option<Vec<ContentBlock>>,
    Option<Vec<ContentBlock>>,
    Option<Vec<ContentBlock>>,
);

/// Decodes private feedback from its immutable canonical source and query
/// projection. The tuple representation is intentionally private to this
/// persistence boundary; browser contracts receive only the derived record.
#[cfg(feature = "postgres")]
pub(super) fn decode_feedback_content(
    version: i16,
    source: String,
    hint: Option<Value>,
    correct_response: Option<Value>,
    rationale: Option<Value>,
    digest: String,
) -> Result<AttemptFeedbackRecord, StoreError> {
    let projection = Value::Array(vec![
        hint.unwrap_or(Value::Null),
        correct_response.unwrap_or(Value::Null),
        rationale.unwrap_or(Value::Null),
    ]);
    let (hint, correct_response, rationale): FeedbackTuple = super::decode_canonical_json_parts(
        "private feedback",
        version,
        source,
        projection,
        digest,
    )?;
    private_feedback_record(FeedbackContent {
        hint,
        correct_response,
        rationale,
    })
    .map_err(|_| StoreError::Unavailable("stored private feedback is invalid".to_string()))
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
        "SELECT hint, correct_response, rationale, content_canonical_json, \
                content_canonical_json_version, content_sha256 \
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
    decode_feedback_content(
        row.try_get("content_canonical_json_version")
            .map_err(map_sqlx_error)?,
        row.try_get("content_canonical_json")
            .map_err(map_sqlx_error)?,
        row.try_get("hint").map_err(map_sqlx_error)?,
        row.try_get("correct_response").map_err(map_sqlx_error)?,
        row.try_get("rationale").map_err(map_sqlx_error)?,
        row.try_get("content_sha256").map_err(map_sqlx_error)?,
    )
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::decode_feedback_content;
    use objects::Sha256Digest;
    use serde_json::Value;

    #[test]
    fn canonical_feedback_reader_preserves_private_empty_tuple() {
        let source = "[null,null,null]".to_string();
        let feedback = decode_feedback_content(
            1,
            source.clone(),
            None,
            None,
            None,
            Sha256Digest::compute(source.as_bytes()).to_string(),
        )
        .expect("canonical private feedback decodes");

        assert_eq!(feedback.content().hint, None);
        assert_eq!(feedback.content().correct_response, None);
        assert_eq!(feedback.content().rationale, None);
    }

    #[test]
    fn canonical_feedback_reader_rejects_projection_disagreement() {
        let source = "[null,null,null]".to_string();
        assert!(
            decode_feedback_content(
                1,
                source.clone(),
                Some(Value::Array(Vec::new())),
                None,
                None,
                Sha256Digest::compute(source.as_bytes()).to_string(),
            )
            .is_err()
        );
    }
}
