//! PostgreSQL adapter for Student Assignment Access and initial issue.

use async_trait::async_trait;
use question_model::{
    AssignmentAttemptReference, AssignmentReference, CourseInstanceReference,
    StudentAssignmentAttemptPosition, StudentAssignmentAttemptProgress,
    StudentAssignmentAttemptResponseState, StudentResponse,
};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::assignment_delivery::ReadyQuestionAssetRendition;
use crate::{
    IssuedQuestionPresentation, LiveAssignmentAccess, LiveAssignmentAttempt,
    LiveAssignmentDeliveryStore, LiveAssignmentStartDecision, NativePleIssuanceSource,
    NativePlePresentationInput, NativeWebworkIssuanceSource, NativeWebworkPresentationInput,
    SessionTokenHash, StoreError, StudentAssignmentAttemptFinalization,
    StudentAssignmentAttemptPresentationSource, StudentAssignmentAttemptSavedResponse,
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

    async fn active_attempt_reference(
        tx: &mut Transaction<'_, Postgres>,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<AssignmentAttemptReference, StoreError> {
        Self::optional_active_attempt_reference(tx, course, assignment)
            .await?
            .ok_or(StoreError::NotFound)
    }

    async fn optional_active_attempt_reference(
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

    async fn student_assignment_attempt_presentation_source(
        &self,
        token: SessionTokenHash,
        assignment_attempt: AssignmentAttemptReference,
        position: u32,
    ) -> Result<StudentAssignmentAttemptPresentationSource, StoreError> {
        let position = i32::try_from(position).map_err(|_| StoreError::NotFound)?;
        if position < 1 {
            return Err(StoreError::NotFound);
        }
        let mut tx = self.begin(token).await?;
        let row = sqlx::query(
            "SELECT backend, question_attempt_id::text, question_id, revision_number, source_object_id::text, source_object_address, \
             source_object_checksum, webwork_pg_path, question_seed::text, presentation_nonce, presentation_checksum, \
             question_asset_renditions \
             FROM ple_api.read_student_assignment_attempt_position($1, $2)",
        )
        .bind(i64::from(assignment_attempt.number()))
        .bind(position)
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
        let source = source_from_row(&row)?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(source)
    }
    async fn prepare_native_webwork_issuance(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<Vec<NativeWebworkIssuanceSource>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT assignment_entry_id::text, issued_position, question_id, revision_number, source_object_id::text, source_object_checksum, webwork_pg_path, resumed FROM ple_api.prepare_live_demo_native_webwork_issuance($1, $2)")
            .bind(i64::from(course.number())).bind(i64::from(assignment.number()))
            .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        let values = rows
            .into_iter()
            .map(|row| {
                Ok(NativeWebworkIssuanceSource {
                    assignment_entry_id: row
                        .try_get("assignment_entry_id")
                        .map_err(map_sqlx_error)?,
                    position: u32::try_from(
                        row.try_get::<i32, _>("issued_position")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Issued Question position is invalid".to_string())
                    })?,
                    question_id: row
                        .try_get::<String, _>("question_id")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| {
                            StoreError::InvalidRecord("Issued Question ID is invalid".to_string())
                        })?,
                    revision_number: u32::try_from(
                        row.try_get::<i32, _>("revision_number")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Revision number is invalid".to_string())
                    })?,
                    source_object_id: row.try_get("source_object_id").map_err(map_sqlx_error)?,
                    source_object_checksum: row
                        .try_get("source_object_checksum")
                        .map_err(map_sqlx_error)?,
                    webwork_pg_path: row.try_get("webwork_pg_path").map_err(map_sqlx_error)?,
                    resumed: row.try_get("resumed").map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(values)
    }

    async fn commit_native_webwork_issuance(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        presentations: Vec<NativeWebworkPresentationInput>,
    ) -> Result<LiveAssignmentAttempt, StoreError> {
        let payload = serde_json::to_value(presentations.iter().map(|value| serde_json::json!({
            "issued_question_id": value.issued_question_id, "assignment_entry_id": value.assignment_entry_id,
            "issued_position": value.position, "question_id": value.question_id.to_string(),
            "revision_number": value.revision_number, "question_seed": value.question_seed.to_string(),
            "parameter_hash": value.parameter_hash, "reproduction_details": value.reproduction_details,
            "presentation_nonce": value.presentation_nonce, "presentation_checksum": value.presentation_checksum,
            "question_assets": value.question_asset_renditions.iter().map(|asset| serde_json::json!({
                "asset_id": asset.question_asset.as_uuid(),
                "rendition_checksum": asset.rendition_checksum,
            })).collect::<Vec<_>>(),
            "replay_details": value.replay_details,
        })).collect::<Vec<_>>()).map_err(|_| StoreError::InvalidRecord("Native WeBWorK issue is invalid".to_string()))?;
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT attempt_number, resumed, assignment_title, assignment_instructions, question_id, revision_number, issued_position, question_seed::text, presentation_nonce, presentation_checksum FROM ple_api.start_live_demo_native_webwork_assignment($1, $2, $3)")
            .bind(i64::from(course.number())).bind(i64::from(assignment.number())).bind(&payload)
            .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        // A successful, authorized native-issuance call must return every
        // persisted Question Presentation. An empty result is therefore an
        // internal availability failure, not an absent browser resource.
        let first = rows.first().ok_or_else(|| {
            StoreError::Unavailable(
                "native assignment issuance returned no presentation rows".to_string(),
            )
        })?;
        let resumed: bool = first.try_get("resumed").map_err(map_sqlx_error)?;
        if !presentations.is_empty() && !resumed {
            sqlx::query("SELECT ple_api.bind_live_demo_native_ple_presentation_assets($1, $2, $3)")
                .bind(i64::from(course.number()))
                .bind(i64::from(assignment.number()))
                .bind(&payload)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        }
        let questions = rows
            .iter()
            .map(|row| {
                Ok(IssuedQuestionPresentation {
                    question_id: row
                        .try_get::<String, _>("question_id")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| {
                            StoreError::InvalidRecord("Issued Question ID is invalid".to_string())
                        })?,
                    description: String::new(),
                    position: u32::try_from(
                        row.try_get::<i32, _>("issued_position")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Issued Question position is invalid".to_string())
                    })?,
                    revision_number: u32::try_from(
                        row.try_get::<i32, _>("revision_number")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Revision is invalid".to_string())
                    })?,
                    question_seed: row
                        .try_get::<String, _>("question_seed")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| {
                            StoreError::InvalidRecord("Question Seed is invalid".to_string())
                        })?,
                    presentation_nonce: row
                        .try_get("presentation_nonce")
                        .map_err(map_sqlx_error)?,
                    presentation_checksum: row
                        .try_get("presentation_checksum")
                        .map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let assignment_attempt =
            Self::active_attempt_reference(&mut tx, course, assignment).await?;
        let result = LiveAssignmentAttempt {
            assignment_attempt,
            assignment,
            attempt_number: u32::try_from(
                first
                    .try_get::<i32, _>("attempt_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| {
                StoreError::InvalidRecord("Assignment Attempt number is invalid".to_string())
            })?,
            resumed: first.try_get("resumed").map_err(map_sqlx_error)?,
            title: first.try_get("assignment_title").map_err(map_sqlx_error)?,
            instructions: first
                .try_get("assignment_instructions")
                .map_err(map_sqlx_error)?,
            questions,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn prepare_native_ple_issuance(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<Vec<NativePleIssuanceSource>, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT assignment_entry_id::text, issued_position, question_id, revision_number, source_object_id::text, source_object_address, source_object_checksum, resumed, question_seed::text, presentation_nonce, presentation_checksum FROM ple_api.prepare_live_demo_native_ple_issuance($1, $2)")
            .bind(i64::from(course.number()))
            .bind(i64::from(assignment.number()))
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let mut values = rows
            .into_iter()
            .map(|row| {
                let position = u32::try_from(
                    row.try_get::<i32, _>("issued_position")
                        .map_err(map_sqlx_error)?,
                )
                .map_err(|_| {
                    StoreError::InvalidRecord("Issued Question position is invalid".to_string())
                })?;
                Ok(NativePleIssuanceSource {
                    assignment_entry_id: row
                        .try_get("assignment_entry_id")
                        .map_err(map_sqlx_error)?,
                    position,
                    question_id: row
                        .try_get::<String, _>("question_id")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| {
                            StoreError::InvalidRecord("Issued Question ID is invalid".to_string())
                        })?,
                    revision_number: u32::try_from(
                        row.try_get::<i32, _>("revision_number")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Revision number is invalid".to_string())
                    })?,
                    source_object_id: row.try_get("source_object_id").map_err(map_sqlx_error)?,
                    source_object_address: row
                        .try_get("source_object_address")
                        .map_err(map_sqlx_error)?,
                    source_object_checksum: row
                        .try_get("source_object_checksum")
                        .map_err(map_sqlx_error)?,
                    resumed: row.try_get("resumed").map_err(map_sqlx_error)?,
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
                    question_asset_renditions: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        for source in &mut values {
            let rows = sqlx::query("SELECT asset_id::text, question_asset_checksum, rendition_checksum, intrinsic_width, intrinsic_height FROM ple_api.select_live_demo_ready_question_asset_renditions($1, $2)")
                .bind(source.question_id.to_string())
                .bind(i32::try_from(source.revision_number).map_err(|_| StoreError::InvalidRecord("Question Revision number is invalid".to_string()))?)
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
            source.question_asset_renditions = rows
                .into_iter()
                .map(|row| {
                    let asset_id = row
                        .try_get::<String, _>("asset_id")
                        .map_err(map_sqlx_error)?;
                    Ok(ReadyQuestionAssetRendition {
                        question_asset: uuid::Uuid::parse_str(&asset_id)
                            .map(question_model::QuestionAssetId::from_uuid)
                            .map_err(|_| {
                                StoreError::InvalidRecord(
                                    "Question Asset ID is invalid".to_string(),
                                )
                            })?,
                        question_asset_checksum: row
                            .try_get("question_asset_checksum")
                            .map_err(map_sqlx_error)?,
                        rendition_checksum: row
                            .try_get("rendition_checksum")
                            .map_err(map_sqlx_error)?,
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
                            StoreError::InvalidRecord(
                                "Question Asset height is invalid".to_string(),
                            )
                        })?,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
        }
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(values)
    }

    async fn commit_native_ple_issuance(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
        presentations: Vec<NativePlePresentationInput>,
    ) -> Result<LiveAssignmentAttempt, StoreError> {
        let payload = serde_json::to_value(presentations.iter().map(|value| serde_json::json!({
            "issued_question_id": value.issued_question_id, "assignment_entry_id": value.assignment_entry_id,
            "issued_position": value.position, "question_id": value.question_id.to_string(),
            "revision_number": value.revision_number, "question_seed": value.question_seed.to_string(),
            "parameter_hash": value.parameter_hash, "reproduction_details": value.reproduction_details,
            "presentation_nonce": value.presentation_nonce, "presentation_checksum": value.presentation_checksum,
            "question_assets": value.question_asset_renditions.iter().map(|asset| serde_json::json!({
                "asset_id": asset.question_asset.as_uuid(),
                "rendition_checksum": asset.rendition_checksum,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>()).map_err(|_| StoreError::InvalidRecord("Native PLE issue is invalid".to_string()))?;
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT attempt_number, resumed, assignment_title, assignment_instructions, question_id, revision_number, issued_position, question_seed::text, presentation_nonce, presentation_checksum FROM ple_api.start_live_demo_native_ple_assignment($1, $2, $3)")
            .bind(i64::from(course.number())).bind(i64::from(assignment.number())).bind(&payload)
            .fetch_all(&mut *tx).await.map_err(map_sqlx_error)?;
        // A successful, authorized native-issuance call must return every
        // persisted Question Presentation. An empty result is therefore an
        // internal availability failure, not an absent browser resource.
        let first = rows.first().ok_or_else(|| {
            StoreError::Unavailable(
                "native assignment issuance returned no presentation rows".to_string(),
            )
        })?;
        let resumed: bool = first.try_get("resumed").map_err(map_sqlx_error)?;
        if !presentations.is_empty() && !resumed {
            sqlx::query("SELECT ple_api.bind_live_demo_native_ple_presentation_assets($1, $2, $3)")
                .bind(i64::from(course.number()))
                .bind(i64::from(assignment.number()))
                .bind(&payload)
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        }
        let questions = rows
            .iter()
            .map(|row| {
                Ok(IssuedQuestionPresentation {
                    question_id: row
                        .try_get::<String, _>("question_id")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| {
                            StoreError::InvalidRecord("Issued Question ID is invalid".to_string())
                        })?,
                    description: String::new(),
                    position: u32::try_from(
                        row.try_get::<i32, _>("issued_position")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Issued Question position is invalid".to_string())
                    })?,
                    revision_number: u32::try_from(
                        row.try_get::<i32, _>("revision_number")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Revision is invalid".to_string())
                    })?,
                    question_seed: row
                        .try_get::<String, _>("question_seed")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| {
                            StoreError::InvalidRecord("Question Seed is invalid".to_string())
                        })?,
                    presentation_nonce: row
                        .try_get("presentation_nonce")
                        .map_err(map_sqlx_error)?,
                    presentation_checksum: row
                        .try_get("presentation_checksum")
                        .map_err(map_sqlx_error)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let assignment_attempt =
            Self::active_attempt_reference(&mut tx, course, assignment).await?;
        let result = LiveAssignmentAttempt {
            assignment_attempt,
            assignment,
            attempt_number: u32::try_from(
                first
                    .try_get::<i32, _>("attempt_number")
                    .map_err(map_sqlx_error)?,
            )
            .map_err(|_| {
                StoreError::InvalidRecord("Assignment Attempt number is invalid".to_string())
            })?,
            resumed: first.try_get("resumed").map_err(map_sqlx_error)?,
            title: first.try_get("assignment_title").map_err(map_sqlx_error)?,
            instructions: first
                .try_get("assignment_instructions")
                .map_err(map_sqlx_error)?,
            questions,
        };
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn live_assignment_access(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<LiveAssignmentAccess, StoreError> {
        let mut tx = self.begin(token).await?;
        let row =
            sqlx::query("SELECT start_decision FROM ple_api.live_demo_assignment_access($1, $2)")
                .bind(i64::from(course.number()))
                .bind(i64::from(assignment.number()))
                .fetch_one(&mut *tx)
                .await
                .map_err(map_sqlx_error)?;
        let value: String = row.try_get("start_decision").map_err(map_sqlx_error)?;
        let active_assignment_attempt =
            Self::optional_active_attempt_reference(&mut tx, course, assignment).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(LiveAssignmentAccess {
            start_decision: decision(&value)?,
            active_assignment_attempt,
        })
    }

    async fn start_live_assignment(
        &self,
        token: SessionTokenHash,
        course: CourseInstanceReference,
        assignment: AssignmentReference,
    ) -> Result<LiveAssignmentAttempt, StoreError> {
        let mut tx = self.begin(token).await?;
        let rows = sqlx::query("SELECT * FROM ple_api.start_live_demo_assignment($1, $2)")
            .bind(i64::from(course.number()))
            .bind(i64::from(assignment.number()))
            .fetch_all(&mut *tx)
            .await
            .map_err(map_sqlx_error)?;
        let first = rows.first().ok_or(StoreError::NotFound)?;
        let attempt_number: i32 = first.try_get("attempt_number").map_err(map_sqlx_error)?;
        let title: String = first.try_get("assignment_title").map_err(map_sqlx_error)?;
        let instructions: String = first
            .try_get("assignment_instructions")
            .map_err(map_sqlx_error)?;
        let resumed: bool = first.try_get("resumed").map_err(map_sqlx_error)?;
        let questions = rows
            .iter()
            .map(|row| {
                Ok(IssuedQuestionPresentation {
                    question_id: row
                        .try_get::<String, _>("question_id")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| {
                            StoreError::InvalidRecord("Issued Question ID is invalid".to_string())
                        })?,
                    description: row
                        .try_get("question_description")
                        .map_err(map_sqlx_error)?,
                    position: u32::try_from(
                        row.try_get::<i32, _>("issued_position")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| {
                        StoreError::InvalidRecord("Issued Question position is invalid".to_string())
                    })?,
                    revision_number: 0,
                    question_seed: 0,
                    presentation_nonce: String::new(),
                    presentation_checksum: String::new(),
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let assignment_attempt =
            Self::active_attempt_reference(&mut tx, course, assignment).await?;
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(LiveAssignmentAttempt {
            assignment_attempt,
            assignment,
            attempt_number: u32::try_from(attempt_number).map_err(|_| {
                StoreError::InvalidRecord("Assignment Attempt number is invalid".to_string())
            })?,
            resumed,
            title,
            instructions,
            questions,
        })
    }
}

fn decision(value: &str) -> Result<LiveAssignmentStartDecision, StoreError> {
    match value {
        "may_start" => Ok(LiveAssignmentStartDecision::MayStart),
        "not_yet_available" => Ok(LiveAssignmentStartDecision::NotYetAvailable),
        "closed" => Ok(LiveAssignmentStartDecision::Closed),
        "attempt_limit_reached" => Ok(LiveAssignmentStartDecision::AttemptLimitReached),
        "late_work_refused" => Ok(LiveAssignmentStartDecision::LateWorkRefused),
        _ => Err(StoreError::InvalidRecord(
            "Assignment Access decision is invalid".to_string(),
        )),
    }
}

fn positive_i32(row: &sqlx::postgres::PgRow, column: &str, label: &str) -> Result<u32, StoreError> {
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

fn optional_positive_i32(
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
}

fn source_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<StudentAssignmentAttemptPresentationSource, StoreError> {
    let backend: String = row.try_get("backend").map_err(map_sqlx_error)?;
    let question_attempt = row
        .try_get::<String, _>("question_attempt_id")
        .map_err(map_sqlx_error)
        .and_then(|value| {
            uuid::Uuid::parse_str(&value)
                .map(question_model::QuestionAttemptId::from_uuid)
                .map_err(|_| {
                    StoreError::InvalidRecord("Question Attempt ID is invalid".to_string())
                })
        })?;
    let question_id = row
        .try_get::<String, _>("question_id")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Issued Question ID is invalid".to_string()))?;
    let revision_number = u32::try_from(
        row.try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Question Revision number is invalid".to_string()))?;
    let source_object_id: String = row.try_get("source_object_id").map_err(map_sqlx_error)?;
    let question_seed = row
        .try_get::<String, _>("question_seed")
        .map_err(map_sqlx_error)?
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Question Seed is invalid".to_string()))?;
    let nonce: String = row.try_get("presentation_nonce").map_err(map_sqlx_error)?;
    let checksum: String = row
        .try_get("presentation_checksum")
        .map_err(map_sqlx_error)?;
    match backend.as_str() {
        "ple" => Ok(StudentAssignmentAttemptPresentationSource::Ple {
            question_attempt,
            source: NativePleIssuanceSource {
                assignment_entry_id: String::new(),
                position: 0,
                question_id,
                revision_number,
                source_object_id,
                source_object_address: row
                    .try_get("source_object_address")
                    .map_err(map_sqlx_error)?,
                source_object_checksum: row
                    .try_get("source_object_checksum")
                    .map_err(map_sqlx_error)?,
                resumed: true,
                question_seed: Some(question_seed),
                presentation_nonce: Some(nonce),
                presentation_checksum: Some(checksum),
                question_asset_renditions: ready_question_asset_renditions(row)?,
            },
        }),
        "webwork" => Ok(StudentAssignmentAttemptPresentationSource::Webwork {
            question_attempt,
            source: NativeWebworkIssuanceSource {
                assignment_entry_id: String::new(),
                position: 0,
                question_id,
                revision_number,
                source_object_id,
                source_object_checksum: row
                    .try_get("source_object_checksum")
                    .map_err(map_sqlx_error)?,
                webwork_pg_path: row
                    .try_get::<Option<String>, _>("webwork_pg_path")
                    .map_err(map_sqlx_error)?
                    .ok_or_else(|| {
                        StoreError::InvalidRecord("WeBWorK path is missing".to_string())
                    })?,
                resumed: true,
            },
            question_seed,
            presentation_nonce: nonce,
            presentation_checksum: checksum,
        }),
        _ => Err(StoreError::InvalidRecord(
            "Question backend is unavailable".to_string(),
        )),
    }
}

#[derive(serde::Deserialize)]
struct ReadyQuestionAssetRenditionRow {
    asset_id: String,
    question_asset_checksum: String,
    rendition_checksum: String,
    intrinsic_width: i32,
    intrinsic_height: i32,
}

fn ready_question_asset_renditions(
    row: &sqlx::postgres::PgRow,
) -> Result<Vec<ReadyQuestionAssetRendition>, StoreError> {
    let values: Vec<ReadyQuestionAssetRenditionRow> = serde_json::from_value(
        row.try_get("question_asset_renditions")
            .map_err(map_sqlx_error)?,
    )
    .map_err(|_| StoreError::InvalidRecord("Question Asset renditions are invalid".to_string()))?;
    values
        .into_iter()
        .map(|value| {
            Ok(ReadyQuestionAssetRendition {
                question_asset: uuid::Uuid::parse_str(&value.asset_id)
                    .map(question_model::QuestionAssetId::from_uuid)
                    .map_err(|_| {
                        StoreError::InvalidRecord("Question Asset ID is invalid".to_string())
                    })?,
                question_asset_checksum: value.question_asset_checksum,
                rendition_checksum: value.rendition_checksum,
                intrinsic_width: u32::try_from(value.intrinsic_width).map_err(|_| {
                    StoreError::InvalidRecord("Question Asset width is invalid".to_string())
                })?,
                intrinsic_height: u32::try_from(value.intrinsic_height).map_err(|_| {
                    StoreError::InvalidRecord("Question Asset height is invalid".to_string())
                })?,
            })
        })
        .collect()
}
