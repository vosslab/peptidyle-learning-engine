//! PostgreSQL adapter for Student Assessment Access and initial issue.
use super::{Pool, connection::map_sqlx_error};
use crate::{
    IssuedQuestionPresentation, LiveAssessmentAccess, LiveAssessmentAttempt,
    LiveAssessmentDeliveryStore, NativeAssessmentIssuanceBatch, NativePleIssuanceSource,
    NativePresentationInput, NativeWebworkIssuanceSource, SessionTokenHash, StoreError,
    StudentAssessmentAttemptBackendDocument, StudentAssessmentAttemptBackendDocumentResume,
    StudentAssessmentAttemptFinalization, StudentAssessmentAttemptFinalizationEvaluation,
    StudentAssessmentAttemptFinalizationPreparation,
    StudentAssessmentAttemptFinalizationPreparationOutcome,
    StudentAssessmentAttemptHistoryEvidence, StudentAssessmentAttemptHistoryResponseSource,
    StudentAssessmentAttemptPresentationEvidence, StudentAssessmentAttemptSavedResponse,
};
use async_trait::async_trait;
use question_model::{
    AssessmentAttemptId, AssessmentId, CourseInstanceId, QuestionRevisionNumber,
    StudentAssessmentAttemptPosition, StudentAssessmentAttemptProgress,
    StudentAssessmentAttemptResponseState, StudentResponse,
};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub(super) use super::assessment_delivery_source::{
    issuance_reproduction_from_row, ready_question_asset_renditions, reproduction_from_row,
    source_from_row,
};
#[path = "assessment_delivery_renditions.rs"]
mod assessment_delivery_renditions;
use assessment_delivery_renditions::{
    current_ready_question_asset_renditions, presentation_payloads,
};

/// PostgreSQL Store for the Student delivery boundary.
#[derive(Clone)]
pub struct PostgresLiveAssessmentDeliveryStore {
    pub(super) pool: Pool,
}
impl PostgresLiveAssessmentDeliveryStore {
    /// Binds the attested API pool to Student Assessment Access procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub(super) async fn begin(
        &self,
        token: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        Ok(tx)
    }

    pub(super) async fn optional_active_attempt_id(
        tx: &mut Transaction<'_, Postgres>,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<Option<AssessmentAttemptId>, StoreError> {
        let row = sqlx::query(
            "SELECT assessment_attempt_id \
             FROM ple_api.read_active_student_assessment_attempt_id($1, $2)",
        )
        .bind(course.as_string())
        .bind(assessment.as_string())
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(decode_assessment_attempt_id(&row)?))
    }

    async fn start_current_assessment_attempt(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<crate::AssessmentAttemptStartResult, StoreError> {
        super::assessment_delivery_start::start_current_assessment_attempt(
            self, token, course, assessment,
        )
        .await
    }
}

async fn read_committed_assessment_attempt(
    tx: &mut Transaction<'_, Postgres>,
    assessment_attempt_id: Uuid,
) -> Result<LiveAssessmentAttempt, StoreError> {
    let header = sqlx::query(
        "SELECT assessment_attempt_id, course_instance_id, assessment_id, \
         assessment_attempt_number, assessment_title, assessment_instructions \
         FROM ple_api.read_started_student_assessment_attempt($1)",
    )
    .bind(assessment_attempt_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let rows = sqlx::query(
        "SELECT assessment_entry_id::text, issued_position, question_id, revision_number, question_seed::text, generated_parameter_sha256, \
         presentation_nonce, presentation_checksum \
         FROM ple_api.read_student_assessment_attempt_presentation_evidence_set($1)",
    )
    .bind(assessment_attempt_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let assessment_attempt = decode_assessment_attempt_id(&header)?;
    let assessment = AssessmentId::new(
        header
            .try_get::<String, _>("assessment_id")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Assessment ID is invalid".to_string()))?;
    let questions = rows
        .iter()
        .map(|row| {
            Ok(IssuedQuestionPresentation {
                assessment_entry_id: row.try_get("assessment_entry_id").map_err(map_sqlx_error)?,
                question_id: row
                    .try_get::<String, _>("question_id")
                    .map_err(map_sqlx_error)?
                    .parse()
                    .map_err(|_| {
                        StoreError::InvalidRecord("Issued Question ID is invalid".to_string())
                    })?,
                description: String::new(),
                position: positive_i32(row, "issued_position", "Issued Question position")?,
                revision_number: positive_i32(row, "revision_number", "Question Revision")?,
                reproduction: reproduction_from_row(row)?,
                presentation_nonce: row.try_get("presentation_nonce").map_err(map_sqlx_error)?,
                presentation_checksum: row
                    .try_get("presentation_checksum")
                    .map_err(map_sqlx_error)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(LiveAssessmentAttempt {
        assessment_attempt,
        assessment,
        attempt_number: positive_i32(
            &header,
            "assessment_attempt_number",
            "Assessment Attempt number",
        )?,
        title: header.try_get("assessment_title").map_err(map_sqlx_error)?,
        instructions: header
            .try_get("assessment_instructions")
            .map_err(map_sqlx_error)?,
        questions,
    })
}

#[async_trait]
impl LiveAssessmentDeliveryStore for PostgresLiveAssessmentDeliveryStore {
    async fn student_assessment_attempt_context(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<crate::StudentAssessmentAttemptContext, StoreError> {
        self.read_student_assessment_attempt_context(token, assessment_attempt)
            .await
    }

    async fn save_student_assessment_attempt_response(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        position: u32,
        response: StudentResponse,
    ) -> Result<StudentAssessmentAttemptSavedResponse, StoreError> {
        let position = positive_position(position)?;
        let position_u32 = u32::try_from(position).map_err(|_| {
            StoreError::InvalidRecord("Issued Question position is invalid".to_string())
        })?;
        let response = serde_json::to_value(response).map_err(|_| {
            StoreError::InvalidRecord("Student response cannot be serialized".to_string())
        })?;
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT assessment_attempt_id, issued_position, response_state \
             FROM ple_api.save_student_assessment_attempt_response($1, $2, $3)",
        )
        .bind(assessment_attempt.as_uuid())
        .bind(position)
        .bind(response)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let returned_attempt = decode_assessment_attempt_id(&row)?;
        let response_state = row
            .try_get::<String, _>("response_state")
            .map_err(map_sqlx_error)?;
        if returned_attempt != assessment_attempt
            || positive_i32(&row, "issued_position", "Issued position")? != position_u32
        {
            return Err(StoreError::InvalidRecord(
                "Student response save result is invalid".to_string(),
            ));
        }
        if response_state == "expired" {
            // The expiry transition must commit even though this late payload
            // is refused at the API boundary.
            tx.commit().await.map_err(map_sqlx_error)?;
            return Err(StoreError::Forbidden);
        }
        if response_state != "saved" {
            return Err(StoreError::InvalidRecord(
                "Student response save result is invalid".to_string(),
            ));
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(StudentAssessmentAttemptSavedResponse {
            assessment_attempt,
            position: position_u32,
        })
    }

    async fn student_assessment_attempt_saved_response(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        position: u32,
    ) -> Result<Option<StudentResponse>, StoreError> {
        let position = positive_position(position)?;
        let position_u32 = u32::try_from(position).map_err(|_| {
            StoreError::InvalidRecord("Issued Question position is invalid".to_string())
        })?;
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT issued_position, student_response \
             FROM ple_api.read_student_assessment_attempt_saved_response($1, $2)",
        )
        .bind(assessment_attempt.as_uuid())
        .bind(position)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        if positive_i32(&row, "issued_position", "Issued position")? != position_u32 {
            return Err(StoreError::InvalidRecord(
                "Saved Student response position is invalid".to_string(),
            ));
        }
        let response = row
            .try_get::<Option<serde_json::Value>, _>("student_response")
            .map_err(map_sqlx_error)?
            .map(|value| {
                serde_json::from_value(value).map_err(|_| {
                    StoreError::InvalidRecord("Saved Student response is invalid".to_string())
                })
            })
            .transpose()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(response)
    }

    async fn prepare_student_assessment_attempt_finalization(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<StudentAssessmentAttemptFinalizationPreparationOutcome, StoreError> {
        super::assessment_delivery_finalization::prepare(self, token, assessment_attempt).await
    }

    async fn commit_student_assessment_attempt_finalization(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        preparation: StudentAssessmentAttemptFinalizationPreparation,
        evaluations: Vec<StudentAssessmentAttemptFinalizationEvaluation>,
    ) -> Result<StudentAssessmentAttemptFinalization, StoreError> {
        super::assessment_delivery_finalization::commit(
            self,
            token,
            assessment_attempt,
            preparation,
            evaluations,
        )
        .await
    }

    async fn student_assessment_attempt_progress(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<StudentAssessmentAttemptProgress, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query(
            "SELECT assessment_attempt_id, question_count, recommended_position, issued_position, response_state \
             FROM ple_api.read_student_assessment_attempt_progress($1)",
        )
        .bind(assessment_attempt.as_uuid())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let first = rows.first().ok_or(StoreError::NotFound)?;
        let question_count = positive_i32(first, "question_count", "Question count")?;
        let recommended_position =
            optional_positive_i32(first, "recommended_position", "Recommended position")?;
        let positions = rows
            .iter()
            .map(|row| {
                Ok(StudentAssessmentAttemptPosition {
                    position: positive_i32(row, "issued_position", "Issued position")?,
                    response_state: response_state(
                        &row.try_get::<String, _>("response_state")
                            .map_err(map_sqlx_error)?,
                    )?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        if positions.len()
            != usize::try_from(question_count)
                .map_err(|_| StoreError::InvalidRecord("Question count is invalid".to_string()))?
            || positions.iter().enumerate().any(|(index, value)| {
                value.position != u32::try_from(index + 1).unwrap_or_default()
            })
        {
            return Err(StoreError::InvalidRecord(
                "Issued Question positions are invalid".to_string(),
            ));
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(StudentAssessmentAttemptProgress {
            assessment_attempt,
            question_count,
            recommended_position,
            positions,
        })
    }

    async fn student_assessment_attempt_presentation_evidence(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        position: u32,
    ) -> Result<StudentAssessmentAttemptPresentationEvidence, StoreError> {
        let position = i32::try_from(position).map_err(|_| StoreError::NotFound)?;
        if position < 1 {
            return Err(StoreError::NotFound);
        }
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT question_id, revision_number, question_seed::text, generated_parameter_sha256, presentation_nonce, presentation_checksum, presentation, author_content, \
             question_asset_renditions, response_item_bindings \
             FROM ple_api.read_student_assessment_attempt_presentation_evidence($1, $2)",
        )
        .bind(assessment_attempt.as_uuid())
        .bind(position)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let evidence = super::assessment_delivery_source::presentation_evidence_from_row(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(evidence)
    }

    async fn student_assessment_attempt_backend_document(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
        position: u32,
    ) -> Result<StudentAssessmentAttemptBackendDocument, StoreError> {
        let position = i32::try_from(position).map_err(|_| StoreError::NotFound)?;
        if position < 1 {
            return Err(StoreError::NotFound);
        }
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT backend_document, issued_question_id, assessment_entry_id, issued_position, \
                    question_id, revision_number, question_seed::text, generated_parameter_sha256, source_object_id, \
                    source_object_checksum, webwork_pg_path, student_response \
             FROM ple_api.read_student_assessment_attempt_backend_document($1, $2)",
        )
        .bind(assessment_attempt.as_uuid())
        .bind(position)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let backend_document = row.try_get("backend_document").map_err(map_sqlx_error)?;
        let saved_response = row
            .try_get::<Option<serde_json::Value>, _>("student_response")
            .map_err(map_sqlx_error)?
            .map(|value| {
                serde_json::from_value(value).map_err(|_| {
                    StoreError::InvalidRecord("Saved Student response is invalid".to_string())
                })
            })
            .transpose()?;
        let resume = match saved_response {
            None => None,
            // ASVS 2.2.1: only the opaque backend-owned shape may be sent
            // back to this backend's resume renderer.
            Some(saved_response @ StudentResponse::BackendOwned { .. }) => {
                let position = positive_i32(&row, "issued_position", "Issued position")?;
                let revision_number = u32::try_from(
                    row.try_get::<i32, _>("revision_number")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| {
                    StoreError::InvalidRecord("Question Revision number is invalid".to_string())
                })?;
                QuestionRevisionNumber::new(revision_number).map_err(|_| {
                    StoreError::InvalidRecord("Question Revision number is invalid".to_string())
                })?;
                let question_id = row
                    .try_get::<String, _>("question_id")
                    .map_err(map_sqlx_error)?
                    .parse()
                    .map_err(|_| {
                        StoreError::InvalidRecord("Issued Question ID is invalid".to_string())
                    })?;
                let reproduction = issuance_reproduction_from_row(&row, "webwork")?;
                let source_object_id = row
                    .try_get::<Uuid, _>("source_object_id")
                    .map_err(map_sqlx_error)?;
                let source_object_checksum = row
                    .try_get::<String, _>("source_object_checksum")
                    .map_err(map_sqlx_error)?;
                let webwork_pg_path = row
                    .try_get::<String, _>("webwork_pg_path")
                    .map_err(map_sqlx_error)?;
                let issued_question_id = row
                    .try_get::<Uuid, _>("issued_question_id")
                    .map_err(map_sqlx_error)?;
                let assessment_entry_id = row
                    .try_get::<Uuid, _>("assessment_entry_id")
                    .map_err(map_sqlx_error)?;
                Some(StudentAssessmentAttemptBackendDocumentResume {
                    source: NativeWebworkIssuanceSource {
                        issued_question_id: Some(issued_question_id),
                        assessment_entry_id: assessment_entry_id.to_string(),
                        position,
                        question_id,
                        revision_number,
                        source_object_id: source_object_id.to_string(),
                        source_object_checksum,
                        webwork_pg_path,
                        reproduction,
                        retained_presentation: None,
                        question_asset_renditions: Vec::new(),
                    },
                    saved_response,
                })
            }
            Some(_) => {
                return Err(StoreError::InvalidRecord(
                    "Saved Student response is invalid".to_string(),
                ));
            }
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(StudentAssessmentAttemptBackendDocument {
            backend_document,
            resume,
        })
    }
    async fn prepare_native_assessment_issuance(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<NativeAssessmentIssuanceBatch, StoreError> {
        let started = self
            .start_current_assessment_attempt(token, course, assessment)
            .await?;
        let mut tx = self.begin(token).await?;
        let retained_rows = sqlx::query(
            "SELECT assessment_entry_id::text, issued_position, question_id, revision_number, question_seed::text, generated_parameter_sha256, presentation_nonce, presentation_checksum, presentation, author_content, question_asset_renditions, response_item_bindings \
             FROM ple_api.read_student_assessment_attempt_presentation_evidence_set($1)",
        )
        .bind(started.assessment_attempt.as_uuid())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !retained_rows.is_empty() {
            let retained_presentations = retained_rows
                .iter()
                .map(super::assessment_delivery_source::presentation_evidence_from_row)
                .collect::<Result<Vec<_>, _>>()?;
            let committed_attempt =
                read_committed_assessment_attempt(&mut tx, started.assessment_attempt.as_uuid())
                    .await?;
            tx.commit().await.map_err(map_sqlx_error)?;
            return Ok(NativeAssessmentIssuanceBatch {
                assessment_attempt_id: started.assessment_attempt.as_uuid(),
                attempt_was_resumed: started.resumed,
                presentation_is_committed: true,
                committed_attempt: Some(committed_attempt),
                retained_presentations,
                ple_sources: Vec::new(),
                webwork_sources: Vec::new(),
            });
        }
        let rows = sqlx::query("SELECT issued_question_id, assessment_entry_id::text, issued_position, question_id, revision_number, backend, source_object_id::text, source_object_address, source_object_checksum, webwork_pg_path, question_seed::text, generated_parameter_sha256, question_attempt_id, presentation_nonce, presentation_checksum, presentation, author_content, question_asset_renditions FROM ple_api.prepare_student_assessment_attempt_presentation($1)")
            .bind(started.assessment_attempt.as_uuid())
            .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let first = rows.first().ok_or_else(|| {
            StoreError::InvalidRecord("Assessment Attempt has no native Questions".to_string())
        })?;
        let presentation_is_committed = presentation_is_resumed(
            first
                .try_get("question_attempt_id")
                .map_err(map_sqlx_error)?,
        );
        let mut ple_sources = Vec::new();
        let mut webwork_sources = Vec::new();
        for row in rows {
            if presentation_is_resumed(row.try_get("question_attempt_id").map_err(map_sqlx_error)?)
                != presentation_is_committed
            {
                return Err(StoreError::InvalidRecord(
                    "Assessment Attempt presentation state is incomplete".to_string(),
                ));
            }
            let position = positive_i32(&row, "issued_position", "Issued Question position")?;
            let question_id = row
                .try_get::<String, _>("question_id")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| {
                    StoreError::InvalidRecord("Issued Question ID is invalid".to_string())
                })?;
            let revision_number = u32::try_from(
                row.try_get::<i32, _>("revision_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| {
                StoreError::InvalidRecord("Question Revision number is invalid".to_string())
            })?;
            let common = (
                row.try_get("issued_question_id").map_err(map_sqlx_error)?,
                row.try_get("assessment_entry_id").map_err(map_sqlx_error)?,
                position,
                question_id,
                revision_number,
                row.try_get("source_object_id").map_err(map_sqlx_error)?,
                row.try_get("source_object_checksum")
                    .map_err(map_sqlx_error)?,
                ready_question_asset_renditions(&row)?,
            );
            let retained_presentation =
                super::assessment_delivery_source::optional_presentation_evidence_from_row(&row)?;
            match row
                .try_get::<String, _>("backend")
                .map_err(map_sqlx_error)?
                .as_str()
            {
                "ple" => {
                    let reproduction = issuance_reproduction_from_row(&row, "ple")?;
                    ple_sources.push(NativePleIssuanceSource {
                        issued_question_id: Some(common.0),
                        assessment_entry_id: common.1,
                        position: common.2,
                        question_id: common.3,
                        revision_number: common.4,
                        source_object_id: common.5,
                        source_object_address: row
                            .try_get("source_object_address")
                            .map_err(map_sqlx_error)?,
                        source_object_checksum: common.6,
                        reproduction,
                        presentation_nonce: row
                            .try_get("presentation_nonce")
                            .map_err(map_sqlx_error)?,
                        presentation_checksum: row
                            .try_get("presentation_checksum")
                            .map_err(map_sqlx_error)?,
                        retained_presentation,
                        question_asset_renditions: common.7,
                    });
                }
                "webwork" => {
                    let reproduction = issuance_reproduction_from_row(&row, "webwork")?;
                    webwork_sources.push(NativeWebworkIssuanceSource {
                        issued_question_id: Some(common.0),
                        assessment_entry_id: common.1,
                        position: common.2,
                        question_id: common.3,
                        revision_number: common.4,
                        source_object_id: common.5,
                        source_object_checksum: common.6,
                        webwork_pg_path: row.try_get("webwork_pg_path").map_err(map_sqlx_error)?,
                        reproduction,
                        retained_presentation,
                        question_asset_renditions: common.7,
                    });
                }
                _ => {
                    return Err(StoreError::InvalidRecord(
                        "Question backend is unavailable".to_string(),
                    ));
                }
            }
        }
        if !presentation_is_committed {
            for source in &mut ple_sources {
                source.question_asset_renditions = current_ready_question_asset_renditions(
                    &mut tx,
                    source.question_id.as_str(),
                    source.revision_number,
                )
                .await?;
            }
            for source in &mut webwork_sources {
                source.question_asset_renditions = current_ready_question_asset_renditions(
                    &mut tx,
                    source.question_id.as_str(),
                    source.revision_number,
                )
                .await?;
            }
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(NativeAssessmentIssuanceBatch {
            assessment_attempt_id: started.assessment_attempt.as_uuid(),
            attempt_was_resumed: started.resumed,
            presentation_is_committed,
            committed_attempt: None,
            retained_presentations: Vec::new(),
            ple_sources,
            webwork_sources,
        })
    }

    async fn commit_native_assessment_issuance(
        &self,
        token: SessionTokenHash,
        assessment_attempt_id: Uuid,
        presentations: Vec<NativePresentationInput>,
    ) -> Result<LiveAssessmentAttempt, StoreError> {
        let payload = presentation_payloads(presentations.iter().map(|value| {
            (
                &value.issued_question_id,
                &value.reproduction,
                &value.reproduction_details,
                &value.presentation,
                &value.presentation_nonce,
                &value.presentation_checksum,
                value.author_content.as_ref(),
                value.question_asset_renditions.as_slice(),
                value.issued_capability.as_str(),
                value.backend_document.as_ref(),
                value.response_item_bindings.as_slice(),
            )
        }))?;
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT issued_question_id::text, issued_position, presentation_nonce, presentation_checksum, resumed FROM ple_api.commit_student_assessment_attempt_presentation($1, $2)")
            .bind(assessment_attempt_id).bind(&payload)
            .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        rows.first().ok_or_else(|| {
            StoreError::Unavailable("Assessment presentation commit returned no rows".to_string())
        })?;
        let result = read_committed_assessment_attempt(&mut tx, assessment_attempt_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn live_assessment_access(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceId,
        assessment: AssessmentId,
    ) -> Result<LiveAssessmentAccess, StoreError> {
        super::assessment_delivery_access::read(self, token, course, assessment).await
    }

    async fn student_assessment_attempt_history(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<StudentAssessmentAttemptHistoryEvidence, StoreError> {
        super::assessment_delivery_history::read(self, token, assessment_attempt).await
    }

    async fn student_assessment_attempt_history_response_sources(
        &self,
        token: SessionTokenHash,
        assessment_attempt: AssessmentAttemptId,
    ) -> Result<Vec<StudentAssessmentAttemptHistoryResponseSource>, StoreError> {
        super::assessment_delivery_history_response::read(self, token, assessment_attempt).await
    }
}

pub(super) fn positive_i32(
    row: &sqlx::postgres::PgRow,
    column: &str,
    label: &str,
) -> Result<u32, StoreError> {
    u32::try_from(row.try_get::<i32, _>(column).map_err(map_sqlx_error)?)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
}

fn positive_position(position: u32) -> Result<i32, StoreError> {
    i32::try_from(position)
        .ok()
        .filter(|position| *position > 0)
        .ok_or_else(|| StoreError::InvalidRecord("Issued Question position is invalid".to_string()))
}

fn decode_assessment_attempt_id(
    row: &sqlx::postgres::PgRow,
) -> Result<AssessmentAttemptId, StoreError> {
    row.try_get::<Uuid, _>("assessment_attempt_id")
        .map(AssessmentAttemptId::from_uuid)
        .map_err(map_sqlx_error)
}

pub(super) use super::assessment_delivery_finalization::finalization_source_from_row;

pub(super) fn optional_positive_i32(
    row: &sqlx::postgres::PgRow,
    column: &str,
    label: &str,
) -> Result<Option<u32>, StoreError> {
    row.try_get::<Option<i32>, _>(column)
        .map_err(map_sqlx_error)?
        .map(|value| {
            u32::try_from(value)
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| StoreError::InvalidRecord(format!("{label} is invalid")))
        })
        .transpose()
}

fn response_state(value: &str) -> Result<StudentAssessmentAttemptResponseState, StoreError> {
    match value {
        "unanswered" => Ok(StudentAssessmentAttemptResponseState::Unanswered),
        "saved" => Ok(StudentAssessmentAttemptResponseState::Saved),
        "submitted" => Ok(StudentAssessmentAttemptResponseState::Submitted),
        "closed" => Ok(StudentAssessmentAttemptResponseState::Closed),
        _ => Err(StoreError::InvalidRecord(
            "Student response state is invalid".to_string(),
        )),
    }
}

fn presentation_is_resumed(question_attempt_id: Option<Uuid>) -> bool {
    question_attempt_id.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_response_state_maps_to_the_shared_navigation_state() {
        assert_eq!(
            response_state("saved").expect("saved state maps"),
            StudentAssessmentAttemptResponseState::Saved
        );
    }

    #[test]
    fn an_unpresented_attempt_is_not_a_renderer_resume_for_a_sibling_backend() {
        assert!(!presentation_is_resumed(None));
        assert!(presentation_is_resumed(Some(Uuid::nil())));
    }

    #[test]
    fn public_issued_question_positions_start_at_one() {
        assert_eq!(positive_position(1).expect("first position is public"), 1);
        assert!(matches!(
            positive_position(0),
            Err(StoreError::InvalidRecord(_))
        ));
    }

    #[test]
    fn prepared_issued_question_identity_is_preserved_in_commit_payload() {
        let issued_question_id = Uuid::from_u128(42).to_string();
        let details = serde_json::json!({
            "backend": { "name": "ple", "version": "test" },
            "rendererVersion": null,
            "sourceObjectId": null,
            "sourceObjectChecksum": null,
            "assetObjects": [],
            "grader": { "name": "ple", "version": "test" },
            "renderedQuestionSha256": "0".repeat(64),
        });
        let reproduction = question_model::QuestionReproduction::Static;
        let payload = presentation_payloads(std::iter::once((
            &issued_question_id,
            &reproduction,
            &details,
            &serde_json::json!({}),
            &"nonce".to_string(),
            &"checksum".to_string(),
            None,
            &[][..],
            "ple_question_json_presentation",
            None,
            &[][..],
        )))
        .expect("commit payload");

        assert_eq!(payload[0]["issued_question_id"], issued_question_id);
        assert_eq!(
            payload[0]["issued_capability"],
            "ple_question_json_presentation"
        );
        assert!(payload[0].get("backend_document").is_none());
        assert!(payload[0]["question_seed"].is_null());
        assert!(payload[0]["generated_parameter_sha256"].is_null());
    }
}
