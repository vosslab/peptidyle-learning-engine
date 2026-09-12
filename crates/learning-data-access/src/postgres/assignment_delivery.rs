//! PostgreSQL adapter for Student Assignment Access and initial issue.
use super::{Pool, connection::map_sqlx_error};
use crate::assignment_delivery::ReadyQuestionAssetRendition;
use crate::{
    IssuedQuestionPresentation, LiveAssignmentAccess, LiveAssignmentAttempt,
    LiveAssignmentDeliveryStore, NativeAssignmentIssuanceBatch, NativePleIssuanceSource,
    NativePresentationInput, NativeWebworkIssuanceSource, SessionTokenHash, StoreError,
    StudentAssignmentAttemptFinalization, StudentAssignmentAttemptHistoryEvidence,
    StudentAssignmentAttemptHistoryResponseSource, StudentAssignmentAttemptPresentationEvidence,
    StudentAssignmentAttemptSavedResponse,
};
use async_trait::async_trait;
use question_model::{
    AssignmentAttemptReference, AssignmentReference, CourseInstanceReference,
    StudentAssignmentAttemptPosition, StudentAssignmentAttemptProgress,
    StudentAssignmentAttemptResponseState, StudentResponse,
};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

pub(super) use super::assignment_delivery_source::{
    ready_question_asset_renditions, source_from_row,
};
/// PostgreSQL Store for the Student delivery boundary.
#[derive(Clone)]
pub struct PostgresLiveAssignmentDeliveryStore {
    pub(super) pool: Pool,
}
impl PostgresLiveAssignmentDeliveryStore {
    /// Binds the attested API pool to Student Assignment Access procedures.
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

    pub(super) async fn optional_active_attempt_reference(
        tx: &mut Transaction<'_, Postgres>,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<Option<AssignmentAttemptReference>, StoreError> {
        let row = sqlx::query(
            "SELECT assignment_attempt_reference_number \
             FROM ple_api.read_active_student_assignment_attempt_reference($1, $2)",
        )
        .bind(i64::from(course.number()))
        .bind(i64::from(assignment.number()))
        .fetch_optional(&mut **tx)
        .await
        .map_err(map_sqlx_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let number = row
            .try_get::<i64, _>("assignment_attempt_reference_number")
            .map_err(map_sqlx_error)?;
        let number = u64::try_from(number).map_err(|_| {
            StoreError::InvalidRecord("Assignment Attempt reference is invalid".to_string())
        })?;
        AssignmentAttemptReference::new(number)
            .map(Some)
            .ok_or_else(|| {
                StoreError::InvalidRecord("Assignment Attempt reference is invalid".to_string())
            })
    }

    async fn start_current_assignment_attempt(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<crate::AssignmentAttemptStartResult, StoreError> {
        super::assignment_delivery_start::start_current_assignment_attempt(
            self, token, course, assignment,
        )
        .await
    }
}

async fn read_committed_assignment_attempt(
    tx: &mut Transaction<'_, Postgres>,
    assignment_attempt_id: Uuid,
) -> Result<LiveAssignmentAttempt, StoreError> {
    let header = sqlx::query(
        "SELECT assignment_attempt_reference_number, course_reference_number, assignment_reference_number, \
         attempt_number, assignment_title, assignment_instructions \
         FROM ple_api.read_started_student_assignment_attempt($1)",
    )
    .bind(assignment_attempt_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let rows = sqlx::query(
        "SELECT assignment_entry_id::text, issued_position, question_id, revision_number, question_seed::text, \
         presentation_nonce, presentation_checksum \
         FROM ple_api.read_student_assignment_attempt_presentation_evidence_set($1)",
    )
    .bind(assignment_attempt_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    let assignment_attempt = AssignmentAttemptReference::new(
        u64::try_from(
            header
                .try_get::<i64, _>("assignment_attempt_reference_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| {
            StoreError::InvalidRecord("Assignment Attempt reference is invalid".to_string())
        })?,
    )
    .ok_or_else(|| {
        StoreError::InvalidRecord("Assignment Attempt reference is invalid".to_string())
    })?;
    let assignment = AssignmentReference::new(
        u64::try_from(
            header
                .try_get::<i64, _>("assignment_reference_number")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| StoreError::InvalidRecord("Assignment reference is invalid".to_string()))?,
    )
    .ok_or_else(|| StoreError::InvalidRecord("Assignment reference is invalid".to_string()))?;
    let questions = rows
        .iter()
        .map(|row| {
            Ok(IssuedQuestionPresentation {
                assignment_entry_id: row.try_get("assignment_entry_id").map_err(map_sqlx_error)?,
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
                question_seed: row
                    .try_get::<String, _>("question_seed")
                    .map_err(map_sqlx_error)?
                    .parse()
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question seed is invalid".to_string())
                    })?,
                presentation_nonce: row.try_get("presentation_nonce").map_err(map_sqlx_error)?,
                presentation_checksum: row
                    .try_get("presentation_checksum")
                    .map_err(map_sqlx_error)?,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(LiveAssignmentAttempt {
        assignment_attempt,
        assignment,
        attempt_number: positive_i32(&header, "attempt_number", "Assignment Attempt number")?,
        title: header.try_get("assignment_title").map_err(map_sqlx_error)?,
        instructions: header
            .try_get("assignment_instructions")
            .map_err(map_sqlx_error)?,
        questions,
    })
}

fn presentation_payloads<'a>(
    values: impl Iterator<
        Item = (
            &'a String,
            &'a String,
            &'a serde_json::Value,
            &'a serde_json::Value,
            &'a String,
            &'a String,
            &'a [ReadyQuestionAssetRendition],
            Option<&'a serde_json::Value>,
            &'a [question_model::presentation::DurableResponseItemBinding],
        ),
    >,
) -> Result<serde_json::Value, StoreError> {
    values.map(|(issued_question_id, parameter_hash, details, presentation, nonce, checksum, assets, replay, response_item_bindings)| {
        let details: question_model::QuestionAttemptReproductionDetails = serde_json::from_value(details.clone())
            .map_err(|_| StoreError::InvalidRecord("Question reproduction details are invalid".to_string()))?;
        let mut payload = serde_json::json!({
            "question_attempt_id": crate::random_uuid::random_uuid_v4(|error| StoreError::Unavailable(format!("Question Attempt ID randomness unavailable: {error}")))?,
            "issued_question_id": issued_question_id,
            "generated_parameter_sha256": parameter_hash,
            "backend_version": details.backend.version,
            "renderer_name": details.renderer_version.as_ref().map(|renderer| renderer.name.as_str()),
            "renderer_version": details.renderer_version.as_ref().map(|renderer| renderer.version.as_str()),
            "grader_name": details.grader.name,
            "grader_version": details.grader.version,
            "rendered_question_sha256": details.rendered_question_sha256,
            "issued_capability": if replay.is_some() { "webwork_presentation" } else { "ple_question_json_presentation" },
            "presentation_nonce": nonce,
            "presentation_checksum": checksum,
            "presentation": presentation,
            "response_item_bindings": response_item_bindings.iter().map(|binding| serde_json::json!({
                "presentation_response_item_reference": binding.presentation_response_item_reference.as_str(),
                "response_item_reference": binding.response_item_reference.as_str(),
            })).collect::<Vec<_>>(),
            "question_assets": assets.iter().map(|asset| serde_json::json!({
                "asset_id": asset.question_asset.as_uuid(),
                "question_asset_checksum": asset.question_asset_checksum,
                "rendition_checksum": asset.rendition_checksum,
                "intrinsic_width": asset.intrinsic_width,
                "intrinsic_height": asset.intrinsic_height,
            })).collect::<Vec<_>>(),
        });
        if let Some(replay) = replay {
            payload["webwork_replay"] = replay.clone();
        }
        Ok(payload)
    }).collect::<Result<Vec<_>, StoreError>>().map(serde_json::Value::Array)
}

async fn current_ready_question_asset_renditions(
    tx: &mut Transaction<'_, Postgres>,
    question_id: &str,
    revision_number: u32,
) -> Result<Vec<ReadyQuestionAssetRendition>, StoreError> {
    let rows = sqlx::query(
        "SELECT asset_id::text, question_asset_checksum, rendition_checksum, intrinsic_width, intrinsic_height \
         FROM ple_api.select_ready_question_asset_renditions($1, $2)",
    )
    .bind(question_id)
    .bind(i32::try_from(revision_number).map_err(|_| {
        StoreError::InvalidRecord("Question Revision number is invalid".to_string())
    })?)
    .fetch_all(&mut **tx)
    .await
    .map_err(map_sqlx_error)?;
    rows.into_iter()
        .map(|row| {
            let asset_id = row
                .try_get::<String, _>("asset_id")
                .map_err(map_sqlx_error)?;
            Ok(ReadyQuestionAssetRendition {
                question_asset: uuid::Uuid::parse_str(&asset_id)
                    .map(question_model::QuestionAssetId::from_uuid)
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Asset ID is invalid".to_string())
                    })?,
                question_asset_checksum: row
                    .try_get("question_asset_checksum")
                    .map_err(map_sqlx_error)?,
                rendition_checksum: row.try_get("rendition_checksum").map_err(map_sqlx_error)?,
                intrinsic_width: u32::try_from(
                    row.try_get::<i32, _>("intrinsic_width")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| {
                    StoreError::InvalidRecord("Question Asset width is invalid".to_string())
                })?,
                intrinsic_height: u32::try_from(
                    row.try_get::<i32, _>("intrinsic_height")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| {
                    StoreError::InvalidRecord("Question Asset height is invalid".to_string())
                })?,
            })
        })
        .collect()
}

#[async_trait]
impl LiveAssignmentDeliveryStore for PostgresLiveAssignmentDeliveryStore {
    async fn student_assignment_attempt_context(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<crate::StudentAssignmentAttemptContext, StoreError> {
        self.read_student_assignment_attempt_context(token, assignment_attempt)
            .await
    }

    async fn save_student_assignment_attempt_response(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
        position: u32,
        response: StudentResponse,
    ) -> Result<StudentAssignmentAttemptSavedResponse, StoreError> {
        let position = positive_position(position)?;
        let position_u32 = u32::try_from(position).map_err(|_| {
            StoreError::InvalidRecord("Issued Question position is invalid".to_string())
        })?;
        let response = serde_json::to_value(response).map_err(|_| {
            StoreError::InvalidRecord("Student response cannot be serialized".to_string())
        })?;
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT assignment_attempt_reference_number, issued_position, response_state \
             FROM ple_api.save_student_assignment_attempt_response($1, $2, $3)",
        )
        .bind(i64::from(assignment_attempt.number()))
        .bind(position)
        .bind(response)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let returned_attempt = assignment_attempt_reference(&row)?;
        if returned_attempt != assignment_attempt
            || positive_i32(&row, "issued_position", "Issued position")? != position_u32
            || row
                .try_get::<String, _>("response_state")
                .map_err(map_sqlx_error)?
                != "saved"
        {
            return Err(StoreError::InvalidRecord(
                "Student response save result is invalid".to_string(),
            ));
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(StudentAssignmentAttemptSavedResponse {
            assignment_attempt,
            position: position_u32,
        })
    }

    async fn student_assignment_attempt_saved_response(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
        position: u32,
    ) -> Result<Option<StudentResponse>, StoreError> {
        let position = positive_position(position)?;
        let position_u32 = u32::try_from(position).map_err(|_| {
            StoreError::InvalidRecord("Issued Question position is invalid".to_string())
        })?;
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT issued_position, student_response \
             FROM ple_api.read_student_assignment_attempt_saved_response($1, $2)",
        )
        .bind(i64::from(assignment_attempt.number()))
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

    async fn finalize_student_assignment_attempt(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<StudentAssignmentAttemptFinalization, StoreError> {
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT submission_state, missing_positions \
             FROM ple_api.finalize_student_assignment_attempt($1)",
        )
        .bind(i64::from(assignment_attempt.number()))
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        let state: String = row.try_get("submission_state").map_err(map_sqlx_error)?;
        let missing_positions = row
            .try_get::<Vec<i32>, _>("missing_positions")
            .map_err(map_sqlx_error)?
            .into_iter()
            .map(|position| {
                u32::try_from(position)
                    .ok()
                    .filter(|position| *position > 0)
                    .ok_or_else(|| {
                        StoreError::InvalidRecord(
                            "Missing Student response position is invalid".to_string(),
                        )
                    })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let finalization = match state.as_str() {
            "submitted" if missing_positions.is_empty() => {
                StudentAssignmentAttemptFinalization::Submitted
            }
            "missing_responses" if !missing_positions.is_empty() => {
                StudentAssignmentAttemptFinalization::MissingResponses {
                    positions: missing_positions,
                }
            }
            _ => {
                return Err(StoreError::InvalidRecord(
                    "Assignment Attempt submission result is invalid".to_string(),
                ));
            }
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(finalization)
    }

    async fn student_assignment_attempt_progress(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<StudentAssignmentAttemptProgress, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query(
            "SELECT assignment_attempt_reference_number, question_count, recommended_position, issued_position, response_state \
             FROM ple_api.read_student_assignment_attempt_progress($1)",
        )
        .bind(i64::from(assignment_attempt.number()))
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
                Ok(StudentAssignmentAttemptPosition {
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
        Ok(StudentAssignmentAttemptProgress {
            assignment_attempt,
            question_count,
            recommended_position,
            positions,
        })
    }

    async fn student_assignment_attempt_presentation_evidence(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
        position: u32,
    ) -> Result<StudentAssignmentAttemptPresentationEvidence, StoreError> {
        let position = i32::try_from(position).map_err(|_| StoreError::NotFound)?;
        if position < 1 {
            return Err(StoreError::NotFound);
        }
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT question_id, revision_number, question_seed::text, presentation_nonce, presentation_checksum, presentation, \
             question_asset_renditions, response_item_bindings \
             FROM ple_api.read_student_assignment_attempt_presentation_evidence($1, $2)",
        )
        .bind(i64::from(assignment_attempt.number()))
        .bind(position)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let evidence = super::assignment_delivery_source::presentation_evidence_from_row(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(evidence)
    }
    async fn prepare_native_assignment_issuance(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<NativeAssignmentIssuanceBatch, StoreError> {
        let started = self
            .start_current_assignment_attempt(token, course, assignment)
            .await?;
        let mut tx = self.begin(token).await?;
        let retained_rows = sqlx::query(
            "SELECT assignment_entry_id::text, issued_position, question_id, revision_number, question_seed::text, presentation_nonce, presentation_checksum, presentation, question_asset_renditions, response_item_bindings \
             FROM ple_api.read_student_assignment_attempt_presentation_evidence_set($1)",
        )
        .bind(started.assignment_attempt.as_uuid())
        .fetch_all(&mut *tx)
        .await
        .map_err(map_sqlx_error)?;
        if !retained_rows.is_empty() {
            let retained_presentations = retained_rows
                .iter()
                .map(super::assignment_delivery_source::presentation_evidence_from_row)
                .collect::<Result<Vec<_>, _>>()?;
            let committed_attempt =
                read_committed_assignment_attempt(&mut tx, started.assignment_attempt.as_uuid())
                    .await?;
            tx.commit().await.map_err(map_sqlx_error)?;
            return Ok(NativeAssignmentIssuanceBatch {
                assignment_attempt_id: started.assignment_attempt.as_uuid(),
                attempt_was_resumed: started.resumed,
                presentation_is_committed: true,
                committed_attempt: Some(committed_attempt),
                retained_presentations,
                ple_sources: Vec::new(),
                webwork_sources: Vec::new(),
            });
        }
        let rows = sqlx::query("SELECT issued_question_id, assignment_entry_id::text, issued_position, question_id, revision_number, backend, source_object_id::text, source_object_address, source_object_checksum, webwork_pg_path, question_seed::text, question_attempt_id, presentation_nonce, presentation_checksum, presentation, question_asset_renditions FROM ple_api.prepare_student_assignment_attempt_presentation($1)")
            .bind(started.assignment_attempt.as_uuid())
            .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let first = rows.first().ok_or_else(|| {
            StoreError::InvalidRecord("Assignment Attempt has no native Questions".to_string())
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
                    "Assignment Attempt presentation state is incomplete".to_string(),
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
                row.try_get("assignment_entry_id").map_err(map_sqlx_error)?,
                position,
                question_id,
                revision_number,
                row.try_get("source_object_id").map_err(map_sqlx_error)?,
                row.try_get("source_object_checksum")
                    .map_err(map_sqlx_error)?,
                ready_question_asset_renditions(&row)?,
            );
            let retained_presentation =
                super::assignment_delivery_source::optional_presentation_evidence_from_row(&row)?;
            match row
                .try_get::<String, _>("backend")
                .map_err(map_sqlx_error)?
                .as_str()
            {
                "ple" => ple_sources.push(NativePleIssuanceSource {
                    issued_question_id: Some(common.0),
                    assignment_entry_id: common.1,
                    position: common.2,
                    question_id: common.3,
                    revision_number: common.4,
                    source_object_id: common.5,
                    source_object_address: row
                        .try_get("source_object_address")
                        .map_err(map_sqlx_error)?,
                    source_object_checksum: common.6,
                    question_seed: row
                        .try_get::<Option<String>, _>("question_seed")
                        .map_err(map_sqlx_error)?
                        .map(|seed| {
                            seed.parse().map_err(|_| {
                                StoreError::InvalidRecord("Question Seed is invalid".to_string())
                            })
                        })
                        .transpose()?,
                    presentation_nonce: row
                        .try_get("presentation_nonce")
                        .map_err(map_sqlx_error)?,
                    presentation_checksum: row
                        .try_get("presentation_checksum")
                        .map_err(map_sqlx_error)?,
                    retained_presentation,
                    question_asset_renditions: common.7,
                }),
                "webwork" => webwork_sources.push(NativeWebworkIssuanceSource {
                    issued_question_id: Some(common.0),
                    assignment_entry_id: common.1,
                    position: common.2,
                    question_id: common.3,
                    revision_number: common.4,
                    source_object_id: common.5,
                    source_object_checksum: common.6,
                    webwork_pg_path: row.try_get("webwork_pg_path").map_err(map_sqlx_error)?,
                    question_seed: row
                        .try_get::<String, _>("question_seed")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| {
                            StoreError::InvalidRecord("Question Seed is invalid".to_string())
                        })?,
                    retained_presentation,
                    question_asset_renditions: common.7,
                }),
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
                    source.question_id.as_compact_str(),
                    source.revision_number,
                )
                .await?;
            }
            for source in &mut webwork_sources {
                source.question_asset_renditions = current_ready_question_asset_renditions(
                    &mut tx,
                    source.question_id.as_compact_str(),
                    source.revision_number,
                )
                .await?;
            }
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(NativeAssignmentIssuanceBatch {
            assignment_attempt_id: started.assignment_attempt.as_uuid(),
            attempt_was_resumed: started.resumed,
            presentation_is_committed,
            committed_attempt: None,
            retained_presentations: Vec::new(),
            ple_sources,
            webwork_sources,
        })
    }

    async fn commit_native_assignment_issuance(
        &self,
        token: SessionTokenHash,
        assignment_attempt_id: Uuid,
        presentations: Vec<NativePresentationInput>,
    ) -> Result<LiveAssignmentAttempt, StoreError> {
        let payload = presentation_payloads(presentations.iter().map(|value| {
            (
                &value.issued_question_id,
                &value.parameter_hash,
                &value.reproduction_details,
                &value.presentation,
                &value.presentation_nonce,
                &value.presentation_checksum,
                value.question_asset_renditions.as_slice(),
                value.replay_details.as_ref(),
                value.response_item_bindings.as_slice(),
            )
        }))?;
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT issued_question_id::text, issued_position, presentation_nonce, presentation_checksum, resumed FROM ple_api.commit_student_assignment_attempt_presentation($1, $2)")
            .bind(assignment_attempt_id).bind(&payload)
            .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        rows.first().ok_or_else(|| {
            StoreError::Unavailable("Assignment presentation commit returned no rows".to_string())
        })?;
        let result = read_committed_assignment_attempt(&mut tx, assignment_attempt_id).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn live_assignment_access(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<LiveAssignmentAccess, StoreError> {
        super::assignment_delivery_access::read(self, token, course, assignment).await
    }

    async fn student_assignment_attempt_history(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<StudentAssignmentAttemptHistoryEvidence, StoreError> {
        super::assignment_delivery_history::read(self, token, assignment_attempt).await
    }

    async fn student_assignment_attempt_history_response_sources(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
    ) -> Result<Vec<StudentAssignmentAttemptHistoryResponseSource>, StoreError> {
        super::assignment_delivery_history_response::read(self, token, assignment_attempt).await
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

fn assignment_attempt_reference(
    row: &sqlx::postgres::PgRow,
) -> Result<AssignmentAttemptReference, StoreError> {
    let number = row
        .try_get::<i64, _>("assignment_attempt_reference_number")
        .map_err(map_sqlx_error)?;
    let number = u64::try_from(number).map_err(|_| {
        StoreError::InvalidRecord("Assignment Attempt reference is invalid".to_string())
    })?;
    AssignmentAttemptReference::new(number).ok_or_else(|| {
        StoreError::InvalidRecord("Assignment Attempt reference is invalid".to_string())
    })
}

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

fn response_state(value: &str) -> Result<StudentAssignmentAttemptResponseState, StoreError> {
    match value {
        "unanswered" => Ok(StudentAssignmentAttemptResponseState::Unanswered),
        "saved" => Ok(StudentAssignmentAttemptResponseState::Saved),
        "submitted" => Ok(StudentAssignmentAttemptResponseState::Submitted),
        "closed" => Ok(StudentAssignmentAttemptResponseState::Closed),
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
            StudentAssignmentAttemptResponseState::Saved
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
            "sourceObjectReference": null,
            "sourceObjectChecksum": null,
            "assetObjects": [],
            "grader": { "name": "ple", "version": "test" },
            "renderedQuestionSha256": "0".repeat(64),
        });
        let payload = presentation_payloads(std::iter::once((
            &issued_question_id,
            &"parameter-hash".to_string(),
            &details,
            &serde_json::json!({}),
            &"nonce".to_string(),
            &"checksum".to_string(),
            &[][..],
            None,
            &[][..],
        )))
        .expect("commit payload");

        assert_eq!(payload[0]["issued_question_id"], issued_question_id);
        assert!(payload[0].get("webwork_replay").is_none());
    }
}
