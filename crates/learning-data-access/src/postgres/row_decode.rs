use super::*;

#[cfg(feature = "postgres")]
pub(super) fn decode_presentation_binding_row(
    row: &PgRow,
) -> Result<Option<PresentationBindingV1>, StoreError> {
    let version: Option<i16> = row
        .try_get("presentation_descriptor_version")
        .map_err(map_sqlx_error)?;
    let nonce: Option<Vec<u8>> = row.try_get("presentation_nonce").map_err(map_sqlx_error)?;
    let digest: Option<Vec<u8>> = row.try_get("presentation_digest").map_err(map_sqlx_error)?;
    match (version, nonce, digest) {
        (None, None, None) => Ok(None),
        (Some(1), Some(nonce), Some(digest)) => {
            let nonce: [u8; 16] = nonce.try_into().map_err(|_| {
                StoreError::Unavailable("stored presentation nonce has invalid length".to_string())
            })?;
            let digest: [u8; 32] = digest.try_into().map_err(|_| {
                StoreError::Unavailable("stored presentation digest has invalid length".to_string())
            })?;
            Ok(Some(PresentationBindingV1::new(
                PresentationNonceV1::from_bytes(nonce),
                PresentationDigestV1::from_bytes(digest),
            )))
        }
        _ => Err(StoreError::Unavailable(
            "stored presentation binding is incomplete or unsupported".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn decode_catalog_payload_row(
    row: &PgRow,
) -> Result<PublishedProblemRecord, StoreError> {
    let mut record: PublishedProblemRecord = decode_payload_row(row)?;
    let stored_problem = ProblemId::from_uuid(row.try_get("problem_id").map_err(map_sqlx_error)?);
    let stored_version = VersionId::from_uuid(row.try_get("version_id").map_err(map_sqlx_error)?);
    let stored_question_id = decode_question_id(
        row.try_get::<String, _>("question_id")
            .map_err(map_sqlx_error)?,
    )?;
    if record.problem != stored_problem
        || record.question_id != stored_question_id
        || record.version != stored_version
    {
        return Err(StoreError::Unavailable(
            "stored catalog payload identity disagrees with its row".to_string(),
        ));
    }
    let lifecycle: String = row.try_get("lifecycle").map_err(map_sqlx_error)?;
    let reason: Option<String> = row.try_get("lifecycle_reason").map_err(map_sqlx_error)?;
    let Json(author_ids): Json<Vec<UserId>> = row.try_get("author_ids").map_err(map_sqlx_error)?;
    let byline_names: Vec<String> = row.try_get("public_byline").map_err(map_sqlx_error)?;
    let byline = question_model::PublicByline::new(
        byline_names
            .into_iter()
            .map(question_model::PublicAuthorName::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Unavailable("stored public byline is invalid".to_string()))?,
    )
    .map_err(|_| StoreError::Unavailable("stored public byline is invalid".to_string()))?;
    if record.author_ids != author_ids || record.byline != byline {
        return Err(StoreError::Unavailable(
            "stored catalog payload attribution disagrees with its row".to_string(),
        ));
    }
    record.lifecycle = parse_catalog_lifecycle(&lifecycle, reason)?;
    validate_published(&record).map_err(|error| {
        StoreError::Unavailable(format!("stored catalog payload is invalid: {error}"))
    })?;
    Ok(record)
}

#[cfg(feature = "postgres")]
pub(super) fn decode_catalog_summary_row(row: &PgRow) -> Result<CatalogProblemSummary, StoreError> {
    let question_id = decode_question_id(
        row.try_get::<String, _>("question_id")
            .map_err(map_sqlx_error)?,
    )?;
    let backend: String = row.try_get("backend").map_err(map_sqlx_error)?;
    let Json(capabilities): Json<BackendCapabilities> =
        row.try_get("capabilities").map_err(map_sqlx_error)?;
    let Json(metadata): Json<QuestionMetadata> = row.try_get("metadata").map_err(map_sqlx_error)?;
    let names: Vec<String> = row.try_get("public_byline").map_err(map_sqlx_error)?;
    let byline = question_model::PublicByline::new(
        names
            .into_iter()
            .map(question_model::PublicAuthorName::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StoreError::Unavailable("stored public byline is invalid".to_string()))?,
    )
    .map_err(|_| StoreError::Unavailable("stored public byline is invalid".to_string()))?;
    let publication_scope: String = row.try_get("publication_scope").map_err(map_sqlx_error)?;
    let lifecycle: String = row.try_get("lifecycle").map_err(map_sqlx_error)?;
    let lifecycle_reason: Option<String> =
        row.try_get("lifecycle_reason").map_err(map_sqlx_error)?;
    let published_at_millis: i64 = row.try_get("published_at_millis").map_err(map_sqlx_error)?;
    Ok(CatalogProblemSummary {
        question_id,
        backend: parse_question_backend(&backend)?,
        capabilities,
        metadata,
        byline,
        scope: parse_publication_scope(&publication_scope)?,
        lifecycle: parse_catalog_lifecycle(&lifecycle, lifecycle_reason)?,
        published_at: ActivityTimestamp::from_unix_millis(published_at_millis),
    })
}

#[cfg(feature = "postgres")]
pub(super) fn decode_question_id(value: String) -> Result<question_model::QuestionId, StoreError> {
    value
        .parse()
        .map_err(|_| StoreError::Unavailable("stored Question ID is invalid".to_string()))
}

#[cfg(feature = "postgres")]
pub(super) fn postgres_search_page_request(
    query: &CatalogSearchQuery,
) -> Result<PageRequest, StoreError> {
    let size = PageSize::new(query.page_size.unwrap_or(50))
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    match query.cursor.clone() {
        Some(cursor) => Cursor::parse(cursor)
            .map(|cursor| PageRequest::after(cursor, size))
            .map_err(|error| StoreError::InvalidRecord(error.to_string())),
        None => Ok(PageRequest::first(size)),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn postgres_catalog_search_fingerprint(query: &CatalogSearchQuery) -> String {
    let mut canonical = String::new();
    canonical.push_str(query.text.as_deref().unwrap_or(""));
    canonical.push('\u{1f}');
    for term in &query.taxonomy {
        canonical.push_str(&term.scheme);
        canonical.push('\u{1e}');
        canonical.push_str(&term.code);
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for capability in &query.capabilities {
        canonical.push_str(capability.as_str());
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    for license in &query.licenses {
        canonical.push_str(&format!("{license:?}"));
        canonical.push('\u{1f}');
    }
    canonical.push('|');
    canonical.push_str(&format!("{:?}", query.statistics));
    Sha256Digest::compute(canonical.as_bytes()).to_string()
}

#[cfg(feature = "postgres")]
pub(super) fn decode_catalog_taxonomy_facet(
    row: PgRow,
) -> Result<CatalogTaxonomyFacet, StoreError> {
    let Json(term): Json<TaxonomyTerm> = row.try_get("taxonomy_term").map_err(map_sqlx_error)?;
    let count: i64 = row.try_get("facet_count").map_err(map_sqlx_error)?;
    Ok(CatalogTaxonomyFacet {
        term,
        count: u64::try_from(count)
            .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
    })
}

#[cfg(feature = "postgres")]
pub(super) fn decode_catalog_capability_facet(
    row: PgRow,
) -> Result<CatalogCapabilityFacet, StoreError> {
    let capability: String = row.try_get("capability").map_err(map_sqlx_error)?;
    let count: i64 = row.try_get("facet_count").map_err(map_sqlx_error)?;
    let capability = serde_json::from_value(Value::String(capability)).map_err(|_| {
        StoreError::Unavailable("stored catalog capability facet is invalid".to_string())
    })?;
    Ok(CatalogCapabilityFacet {
        capability,
        count: u64::try_from(count)
            .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
    })
}

#[cfg(feature = "postgres")]
pub(super) fn decode_catalog_license_facet(row: PgRow) -> Result<CatalogLicenseFacet, StoreError> {
    let license: String = row.try_get("license").map_err(map_sqlx_error)?;
    let count: i64 = row.try_get("facet_count").map_err(map_sqlx_error)?;
    let license = serde_json::from_value(Value::String(license)).map_err(|_| {
        StoreError::Unavailable("stored catalog license facet is invalid".to_string())
    })?;
    Ok(CatalogLicenseFacet {
        license,
        count: u64::try_from(count)
            .map_err(|_| StoreError::Unavailable("catalog count overflow".to_string()))?,
    })
}

#[cfg(feature = "postgres")]
pub(super) fn catalog_summary_page_from_rows(
    rows: Vec<PgRow>,
    page_size: u16,
) -> Result<Page<CatalogProblemSummary>, StoreError> {
    let mut records = rows
        .iter()
        .map(|row| {
            let key = row
                .try_get::<String, _>("stable_key")
                .map_err(map_sqlx_error)?;
            Ok((key, decode_catalog_summary_row(row)?))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    page_from_keyed_records(&mut records, page_size)
}

#[cfg(feature = "postgres")]
pub(super) fn taxonomy_page_from_rows(
    rows: Vec<PgRow>,
    page_size: u16,
) -> Result<Page<TaxonomyTerm>, StoreError> {
    let mut records = rows
        .iter()
        .map(|row| {
            let key = row
                .try_get::<String, _>("stable_key")
                .map_err(map_sqlx_error)?;
            let Json(term): Json<TaxonomyTerm> =
                row.try_get("taxonomy_term").map_err(map_sqlx_error)?;
            Ok((key, term))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    page_from_keyed_records(&mut records, page_size)
}

#[cfg(feature = "postgres")]
pub(super) fn page_from_keyed_records<T>(
    records: &mut Vec<(String, T)>,
    page_size: u16,
) -> Result<Page<T>, StoreError> {
    let has_more = records.len() > usize::from(page_size);
    if has_more {
        records.pop();
    }
    let next_cursor = if has_more {
        records
            .last()
            .map(|(key, _)| Cursor::from_stable_key(key.clone()))
    } else {
        None
    };
    Ok(Page {
        items: records.drain(..).map(|(_, record)| record).collect(),
        next_cursor,
    })
}

/// Converts the `LIMIT page_size + 1` native UUID tuple result into one page.
///
/// The SQL order and continuation key deliberately use the same tuple, unlike
/// the generic string-key pages above. That alignment keeps the gradebook
/// query eligible for its assignment/enrollment page indexes.
#[cfg(feature = "postgres")]
pub(super) fn gradebook_page_from_records<T>(
    records: &mut Vec<(GradebookCursor, T)>,
    page_size: u16,
) -> Page<T> {
    let has_more = records.len() > usize::from(page_size);
    if has_more {
        records.pop();
    }
    let next_cursor = has_more.then(|| {
        records
            .last()
            .map(|(key, _)| key.encode())
            .expect("a nonempty page precedes a following page")
    });
    Page {
        items: records.drain(..).map(|(_, record)| record).collect(),
        next_cursor,
    }
}

#[cfg(feature = "postgres")]
pub(super) fn validated_deprecation_reason(reason: String) -> Result<String, StoreError> {
    const MAX_REASON_CHARS: usize = 1_000;
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(StoreError::InvalidRecord(
            "deprecation requires a nonempty reason".to_string(),
        ));
    }
    if reason.chars().count() > MAX_REASON_CHARS {
        return Err(StoreError::InvalidRecord(format!(
            "deprecation reason must contain at most {MAX_REASON_CHARS} characters"
        )));
    }
    Ok(reason.to_string())
}

#[cfg(feature = "postgres")]
pub(super) fn encode_payload<T: Serialize>(
    record: &T,
) -> Result<(Json<Value>, String), StoreError> {
    let value = serde_json::to_value(record)
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let bytes =
        serde_json::to_vec(&value).map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
    let checksum = Sha256Digest::compute(&bytes).to_string();
    Ok((Json(value), checksum))
}

#[cfg(feature = "postgres")]
pub(super) fn decode_payload_row<T: DeserializeOwned>(row: &PgRow) -> Result<T, StoreError> {
    let Json(value): Json<Value> = row.try_get("payload").map_err(map_sqlx_error)?;
    let expected: String = row.try_get("payload_sha256").map_err(map_sqlx_error)?;
    decode_payload_parts(value, expected)
}

#[cfg(feature = "postgres")]
pub(super) fn decode_payload_parts<T: DeserializeOwned>(
    value: Value,
    expected: String,
) -> Result<T, StoreError> {
    let bytes =
        serde_json::to_vec(&value).map_err(|error| StoreError::Unavailable(error.to_string()))?;
    if Sha256Digest::compute(&bytes).to_string() != expected {
        return Err(StoreError::Unavailable(
            "stored JSON payload checksum mismatch".to_string(),
        ));
    }
    serde_json::from_value(value).map_err(|error| StoreError::Unavailable(error.to_string()))
}

#[cfg(feature = "postgres")]
pub(super) fn decode_payload_row_named<T: DeserializeOwned>(
    row: &PgRow,
    payload_name: &str,
    checksum_name: &str,
) -> Result<T, StoreError> {
    let Json(value): Json<Value> = row.try_get(payload_name).map_err(map_sqlx_error)?;
    let expected: String = row.try_get(checksum_name).map_err(map_sqlx_error)?;
    let bytes =
        serde_json::to_vec(&value).map_err(|error| StoreError::Unavailable(error.to_string()))?;
    if Sha256Digest::compute(&bytes).to_string() != expected {
        return Err(StoreError::Unavailable(
            "stored JSON payload checksum mismatch".to_string(),
        ));
    }
    serde_json::from_value(value).map_err(|error| StoreError::Unavailable(error.to_string()))
}

#[cfg(feature = "postgres")]
pub(super) fn attempt_status_name(status: AttemptStatus) -> &'static str {
    match status {
        AttemptStatus::InProgress => "in_progress",
        AttemptStatus::Submitted => "submitted",
        AttemptStatus::AutoSubmitted => "auto_submitted",
        AttemptStatus::NeedsManualGrading => "needs_manual_grading",
        AttemptStatus::Cleared => "cleared",
        AttemptStatus::Exempt => "exempt",
    }
}

#[cfg(feature = "postgres")]
pub(super) fn decode_attempt_status(value: &str) -> Result<AttemptStatus, StoreError> {
    match value {
        "in_progress" => Ok(AttemptStatus::InProgress),
        "submitted" => Ok(AttemptStatus::Submitted),
        "auto_submitted" => Ok(AttemptStatus::AutoSubmitted),
        "needs_manual_grading" => Ok(AttemptStatus::NeedsManualGrading),
        "cleared" => Ok(AttemptStatus::Cleared),
        "exempt" => Ok(AttemptStatus::Exempt),
        _ => Err(StoreError::Unavailable(
            "stored attempt status is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn decode_current_attempt_row_named(
    row: &PgRow,
    payload_name: &str,
    checksum_name: &str,
) -> Result<QuestionAttempt, StoreError> {
    let mut attempt: QuestionAttempt = decode_payload_row_named(row, payload_name, checksum_name)?;
    let status: String = row
        .try_get("current_attempt_status")
        .map_err(map_sqlx_error)?;
    attempt.status = decode_attempt_status(&status)?;
    if let Some(submitted_at) = row
        .try_get::<Option<i64>, _>("current_submitted_at")
        .map_err(map_sqlx_error)?
    {
        attempt.timer.submitted_at = Some(ActivityTimestamp::from_unix_millis(submitted_at));
    } else if attempt.status == AttemptStatus::InProgress {
        attempt.timer.submitted_at = None;
    }
    match row.try_get::<Option<i64>, _>("current_deadline_at") {
        Ok(deadline) => {
            attempt.timer.deadline = deadline.map(ActivityTimestamp::from_unix_millis);
        }
        Err(sqlx::Error::ColumnNotFound(_)) => {}
        Err(error) => return Err(map_sqlx_error(error)),
    }
    Ok(attempt)
}

#[cfg(feature = "postgres")]
pub(super) fn decode_current_attempt_row(row: &PgRow) -> Result<QuestionAttempt, StoreError> {
    decode_current_attempt_row_named(row, "payload", "payload_sha256")
}

#[cfg(feature = "postgres")]
pub(super) fn decode_current_attempt_with_evaluation_row_named(
    row: &PgRow,
    payload_name: &str,
    checksum_name: &str,
) -> Result<QuestionAttempt, StoreError> {
    let mut attempt = decode_current_attempt_row_named(row, payload_name, checksum_name)?;
    let status: Option<String> = row
        .try_get("evaluation_grading_status")
        .map_err(map_sqlx_error)?;
    let Some(status) = status else {
        return Ok(attempt);
    };
    let Json(payload): Json<Value> = row.try_get("evaluation_payload").map_err(map_sqlx_error)?;
    let checksum: String = row
        .try_get("evaluation_payload_sha256")
        .map_err(map_sqlx_error)?;
    let bytes =
        serde_json::to_vec(&payload).map_err(|error| StoreError::Unavailable(error.to_string()))?;
    if Sha256Digest::compute(&bytes).to_string() != checksum {
        return Err(StoreError::Unavailable(
            "stored evaluation payload checksum mismatch".to_string(),
        ));
    }
    match status.as_str() {
        "needs_manual_grading" => {
            attempt.result = None;
            Ok(attempt)
        }
        "graded" | "exempt" => {
            let result = serde_json::from_value(payload)
                .map_err(|error| StoreError::Unavailable(error.to_string()))?;
            crate::validate_attempt_result(result)?;
            attempt.result = Some(result);
            Ok(attempt)
        }
        _ => Err(StoreError::Unavailable(
            "stored evaluation grading status is invalid".to_string(),
        )),
    }
}

#[cfg(feature = "postgres")]
pub(super) fn decode_current_attempt_with_evaluation_row(
    row: &PgRow,
) -> Result<QuestionAttempt, StoreError> {
    decode_current_attempt_with_evaluation_row_named(row, "payload", "payload_sha256")
}

#[cfg(feature = "postgres")]
pub(super) fn feedback_from_summary_row(
    row: &PgRow,
) -> Result<Option<AttemptFeedbackRecord>, StoreError> {
    let digest: Option<String> = row.try_get("content_sha256").map_err(map_sqlx_error)?;
    let Some(digest) = digest else {
        return Ok(None);
    };
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
    let feedback = private_feedback_record(FeedbackContent {
        hint: field(row, "hint")?,
        correct_response: field(row, "correct_response")?,
        rationale: field(row, "rationale")?,
    })?;
    if feedback.content_sha256().to_string() != digest {
        return Err(StoreError::InvalidRecord(
            "stored feedback digest mismatch".to_string(),
        ));
    }
    Ok(Some(feedback))
}

#[cfg(feature = "postgres")]
pub(super) fn page_from_rows<T: DeserializeOwned>(
    rows: Vec<PgRow>,
    page_size: u16,
) -> Result<Page<T>, StoreError> {
    page_from_rows_with(rows, page_size, decode_payload_row)
}

#[cfg(feature = "postgres")]
pub(super) fn page_from_rows_with<T>(
    rows: Vec<PgRow>,
    page_size: u16,
    decode: impl Fn(&PgRow) -> Result<T, StoreError>,
) -> Result<Page<T>, StoreError> {
    let mut records = rows
        .iter()
        .map(|row| {
            let key = row
                .try_get::<String, _>("stable_key")
                .map_err(map_sqlx_error)?;
            let record = decode(row)?;
            Ok((key, record))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let has_more = records.len() > usize::from(page_size);
    if has_more {
        records.pop();
    }
    let next_cursor = if has_more {
        records
            .last()
            .map(|(key, _)| Cursor::from_stable_key(key.clone()))
    } else {
        None
    };
    Ok(Page {
        items: records.into_iter().map(|(_, record)| record).collect(),
        next_cursor,
    })
}
