//! PostgreSQL adapter for M11 Student Assignment Access and initial issue.

use async_trait::async_trait;
use question_model::{AssignmentReference, CourseInstanceReference};
use sqlx::{Postgres, Row, Transaction};

use super::{Pool, connection::map_sqlx_error};
use crate::assignment_delivery::ReadyQuestionAssetRendition;
use crate::{
    IssuedQuestionPresentation, LiveAssignmentAccess, LiveAssignmentAttempt,
    LiveAssignmentDeliveryStore, LiveAssignmentStartDecision, NativePleIssuanceSource,
    NativePlePresentationInput, NativeWebworkIssuanceSource, NativeWebworkPresentationInput,
    SessionTokenHash, StoreError,
};

/// PostgreSQL Store for the Student delivery boundary.
#[derive(Clone)]
pub struct PostgresLiveAssignmentDeliveryStore {
    pool: Pool,
}

impl PostgresLiveAssignmentDeliveryStore {
    /// Binds the attested API pool to M11 procedures.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    async fn begin(
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
}

#[async_trait]
impl LiveAssignmentDeliveryStore for PostgresLiveAssignmentDeliveryStore {
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
        let result = LiveAssignmentAttempt {
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
        let result = LiveAssignmentAttempt {
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
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(LiveAssignmentAccess {
            start_decision: decision(&value)?,
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
        tx.commit().await.map_err(map_sqlx_error)?;
        Ok(LiveAssignmentAttempt {
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
