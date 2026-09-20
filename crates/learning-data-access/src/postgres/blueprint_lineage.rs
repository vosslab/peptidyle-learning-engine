//! PostgreSQL adapter for the Blueprint fork transaction.

use async_trait::async_trait;
use question_model::{
    BlueprintAssessmentId, BlueprintAvailability, BlueprintCourseId, BlueprintEditNumber,
    BlueprintModuleId, BlueprintRevisionNumber, BlueprintRevisionTuple, QuestionId,
    QuestionPoolEditNumber, QuestionRevisionNumber, QuestionRevisionTuple, RequestChecksum,
    Timestamp,
};
use sqlx::{Postgres, Row, Transaction, types::Json};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::{
    Pool,
    connection::{map_sqlx_error, parse_account_id},
};
use crate::blueprint_lineage::StoredKnownBlueprintFork;
use crate::{
    BlueprintComparisonSources, BlueprintForkSource, BlueprintLineageStore,
    CourseInstancePoolIdIssuer, ForkBlueprintCourseReceipt, SessionTokenHash, StoreError,
    StoredBlueprintAssessmentEntry, StoredBlueprintRevision,
};

/// PostgreSQL implementation of the closed Blueprint fork boundary.
#[derive(Clone)]
pub struct PostgresBlueprintLineageStore {
    pool: Pool,
    pool_id_issuer: Option<Arc<dyn CourseInstancePoolIdIssuer>>,
}

impl PostgresBlueprintLineageStore {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            pool_id_issuer: None,
        }
    }

    pub fn with_question_pool_id_issuer(
        mut self,
        issuer: Arc<dyn CourseInstancePoolIdIssuer>,
    ) -> Self {
        self.pool_id_issuer = Some(issuer);
        self
    }

    async fn begin(
        &self,
        session: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let resolved = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(session.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if resolved.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

#[async_trait]
impl BlueprintLineageStore for PostgresBlueprintLineageStore {
    async fn list_known_blueprint_forks(
        &self,
        session: SessionTokenHash,
        source_blueprint_course_id: BlueprintCourseId,
    ) -> Result<Vec<StoredKnownBlueprintFork>, StoreError> {
        let concealed = |error| match error {
            StoreError::Forbidden => StoreError::NotFound,
            other => other,
        };
        let mut transaction = self.begin(session).await.map_err(concealed)?;
        // ASVS 1.2.4, 8.2.2/3: bound source; trusted reader filters children and fields.
        let rows = sqlx::query(
            "SELECT blueprint_course_id, short_name, long_name, availability, \
             blueprint_revision_number, source_blueprint_revision_number, owner_display_name \
             FROM ple_api.list_known_blueprint_forks($1) ORDER BY blueprint_course_id COLLATE \"C\"",
        )
        .bind(source_blueprint_course_id.as_string())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)
        .map_err(concealed)?;
        let mut forks = Vec::with_capacity(rows.len());
        for row in rows {
            let availability: String = row.try_get("availability").map_err(map_sqlx_error)?;
            let owner_display_name: Option<String> =
                row.try_get("owner_display_name").map_err(map_sqlx_error)?;
            let fork_blueprint_course_id = parse_blueprint_course_id(
                row.try_get("blueprint_course_id").map_err(map_sqlx_error)?,
            )?;
            forks.push(StoredKnownBlueprintFork {
                id: fork_blueprint_course_id.clone(),
                short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
                long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
                availability: match availability.as_str() {
                    "private" => BlueprintAvailability::Private,
                    "public" => BlueprintAvailability::Public,
                    "archived" => BlueprintAvailability::Archived,
                    _ => {
                        return Err(StoreError::InvalidRecord(
                            "Blueprint availability is invalid".into(),
                        ));
                    }
                },
                current_revision_tuple: BlueprintRevisionTuple {
                    blueprint_course_id: fork_blueprint_course_id,
                    revision_number: blueprint_revision_number(
                        row.try_get("blueprint_revision_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                },
                source_revision_tuple: BlueprintRevisionTuple {
                    blueprint_course_id: source_blueprint_course_id.clone(),
                    revision_number: blueprint_revision_number(
                        row.try_get("source_blueprint_revision_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                },
                owner_display_name: owner_display_name.ok_or_else(|| {
                    StoreError::InvalidRecord(
                        "Blueprint fork owner verified display name is unavailable".into(),
                    )
                })?,
            });
        }
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(forks)
    }

    async fn load_blueprint_comparison_sources(
        &self,
        session: SessionTokenHash,
        left: BlueprintCourseId,
        right: BlueprintCourseId,
    ) -> Result<BlueprintComparisonSources, StoreError> {
        let mut transaction = self.begin(session).await.map_err(|error| match error {
            StoreError::Forbidden => StoreError::NotFound,
            other => other,
        })?;
        // ASVS 1.2.4, 8.2.2, 8.3.1/2: bound arguments and one trusted reader.
        let rows = sqlx::query(
            "SELECT * FROM ple_api.load_blueprint_comparison_sources($1, $2) ORDER BY source_position",
        )
        .bind(left.as_string())
        .bind(right.as_string())
        .fetch_all(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if rows.is_empty() {
            return Err(StoreError::NotFound);
        }
        if rows.len() != 2 {
            return Err(StoreError::InvalidRecord(
                "Blueprint fork review sources are invalid".into(),
            ));
        }
        let mut revisions = Vec::with_capacity(2);
        for (index, row) in rows.iter().enumerate() {
            let source_position: i32 = row.try_get("source_position").map_err(map_sqlx_error)?;
            if source_position != index as i32 {
                return Err(StoreError::InvalidRecord(
                    "Blueprint fork review source order is invalid".into(),
                ));
            }
            let Json(encoded): Json<serde_json::Value> =
                row.try_get("content").map_err(map_sqlx_error)?;
            revisions.push(StoredBlueprintRevision {
                blueprint_revision_tuple: BlueprintRevisionTuple {
                    blueprint_course_id: parse_blueprint_course_id(
                        row.try_get("blueprint_course_id").map_err(map_sqlx_error)?,
                    )?,
                    revision_number: blueprint_revision_number(
                        row.try_get("blueprint_revision_number")
                            .map_err(map_sqlx_error)?,
                    )?,
                },
                content: super::blueprint_course::decode_revision_content(
                    encoded,
                    &row.try_get::<Vec<u8>, _>("content_checksum")
                        .map_err(map_sqlx_error)?,
                )?,
            });
        }
        let pool_memberships = load_pool_memberships(&mut transaction, &revisions).await?;
        let row = &rows[0];
        let mut revisions = revisions.into_iter();
        let result = BlueprintComparisonSources {
            left: revisions.next().ok_or(StoreError::NotFound)?,
            right: revisions.next().ok_or(StoreError::NotFound)?,
            left_short_name: row.try_get("left_short_name").map_err(map_sqlx_error)?,
            left_long_name: row.try_get("left_long_name").map_err(map_sqlx_error)?,
            right_short_name: row.try_get("right_short_name").map_err(map_sqlx_error)?,
            right_long_name: row.try_get("right_long_name").map_err(map_sqlx_error)?,
            left_blueprint_edit_number: BlueprintEditNumber::from_edit_number(
                row.try_get("left_blueprint_edit_number")
                    .map_err(map_sqlx_error)?,
            ),
            right_blueprint_edit_number: BlueprintEditNumber::from_edit_number(
                row.try_get("right_blueprint_edit_number")
                    .map_err(map_sqlx_error)?,
            ),
            pool_memberships,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }

    async fn fork_blueprint_course(
        &self,
        session: SessionTokenHash,
        source: BlueprintForkSource,
        request_checksum: RequestChecksum,
        mut bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<ForkBlueprintCourseReceipt, StoreError> {
        let mut transaction = self.begin(session).await?;
        let actor = current_actor(&mut transaction).await?;
        let source_revision_number =
            i64::try_from(source.blueprint_revision_tuple.revision_number.value())
                .map_err(|_| StoreError::InvalidRecord("Blueprint Revision is invalid".into()))?;
        // Source authorization and lifecycle stay locked through the write. The
        // existing request receipt takes priority over re-reading its source.
        let snapshot = sqlx::query(
            "SELECT content, content_checksum FROM ple_api.load_blueprint_fork_source($1, $2, $3)",
        )
        .bind(
            source
                .blueprint_revision_tuple
                .blueprint_course_id
                .as_string(),
        )
        .bind(source_revision_number)
        .bind(request_checksum.into_bytes().to_vec())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let encoded: Option<Json<serde_json::Value>> =
            snapshot.try_get("content").map_err(map_sqlx_error)?;
        let (child_content, child_checksum) = if let Some(Json(encoded)) = encoded {
            let mut content = super::blueprint_course::decode_revision_content(
                encoded,
                &snapshot
                    .try_get::<Vec<u8>, _>("content_checksum")
                    .map_err(map_sqlx_error)?,
            )?;
            for module in &mut content.modules {
                module.blueprint_module_id = BlueprintModuleId::from_uuid(random_uuid()?);
                for assessment in &mut module.assessments {
                    assessment.blueprint_assessment_id =
                        BlueprintAssessmentId::from_uuid(random_uuid()?);
                }
            }
            super::blueprint_pools::materialize_imported_pools(
                &mut transaction,
                &mut content,
                self.pool_id_issuer.as_deref(),
                &mut bloom_receipts,
            )
            .await?;
            let checksum = content.checksum()?.as_bytes().to_vec();
            (
                Some(Json(super::blueprint_course::encode_content(&content)?)),
                Some(checksum),
            )
        } else {
            (None, None)
        };
        let row = sqlx::query(
            "SELECT blueprint_course_id, blueprint_revision_number, blueprint_edit_number, \
             (EXTRACT(EPOCH FROM accepted_at) * 1000)::bigint AS accepted_at_millis \
             FROM ple_api.fork_blueprint_course($1, $2, $3, $4, $5, $6)",
        )
        .bind(random_uuid()?)
        .bind(
            source
                .blueprint_revision_tuple
                .blueprint_course_id
                .as_string(),
        )
        .bind(source_revision_number)
        .bind(request_checksum.into_bytes().to_vec())
        .bind(child_content)
        .bind(child_checksum)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let blueprint_course_id: String =
            row.try_get("blueprint_course_id").map_err(map_sqlx_error)?;
        let revision_number: i64 = row
            .try_get("blueprint_revision_number")
            .map_err(map_sqlx_error)?;
        let accepted_at_millis: i64 = row.try_get("accepted_at_millis").map_err(map_sqlx_error)?;
        let receipt = ForkBlueprintCourseReceipt {
            blueprint_revision_tuple: BlueprintRevisionTuple {
                blueprint_course_id: parse_blueprint_course_id(blueprint_course_id)?,
                revision_number: blueprint_revision_number(revision_number)?,
            },
            source,
            blueprint_edit_number: BlueprintEditNumber::from_edit_number(
                row.try_get("blueprint_edit_number")
                    .map_err(map_sqlx_error)?,
            ),
            actor,
            request_checksum,
            accepted_at: Timestamp::from_unix_millis(accepted_at_millis),
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }
}

pub(super) async fn load_pool_memberships(
    transaction: &mut Transaction<'_, Postgres>,
    revisions: &[StoredBlueprintRevision],
) -> Result<BTreeMap<(QuestionId, QuestionPoolEditNumber), Vec<QuestionRevisionTuple>>, StoreError>
{
    let pins: BTreeSet<_> = revisions
        .iter()
        .flat_map(|revision| &revision.content.modules)
        .flat_map(|module| &module.assessments)
        .flat_map(|assessment| &assessment.content.entries)
        .filter_map(|entry| match entry {
            StoredBlueprintAssessmentEntry::Pool {
                question_pool_id,
                question_pool_edit_number,
                ..
            } => Some((question_pool_id.clone(), *question_pool_edit_number)),
            StoredBlueprintAssessmentEntry::Fixed { .. } => None,
        })
        .collect();
    let invalid =
        || StoreError::InvalidRecord("Blueprint comparison Pool membership is invalid".into());
    let mut result = BTreeMap::new();
    for (question_pool_id, question_pool_edit_number) in pins {
        // ASVS 1.2.4, 8.2.3: exact bound metadata only, never Question bodies/answers.
        let rows = sqlx::query("SELECT * FROM ple_api.read_current_published_question_pool($1) ORDER BY member_position")
            .bind(question_pool_id.as_str())
            .fetch_all(&mut **transaction).await.map_err(map_sqlx_error)?;
        if rows.is_empty() {
            return Err(invalid());
        }
        let mut members = Vec::with_capacity(rows.len());
        let mut unique = BTreeSet::new();
        for (index, row) in rows.iter().enumerate() {
            let pool_id: QuestionId = row
                .try_get::<String, _>("question_pool_id")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| invalid())?;
            if pool_id != question_pool_id
                || row
                    .try_get::<i64, _>("question_pool_edit_number")
                    .map_err(map_sqlx_error)?
                    != i64::try_from(question_pool_edit_number.get()).map_err(|_| invalid())?
                || row
                    .try_get::<i32, _>("member_position")
                    .map_err(map_sqlx_error)?
                    != i32::try_from(index + 1).map_err(|_| invalid())?
            {
                return Err(invalid());
            }
            let member = QuestionRevisionTuple {
                question_id: row
                    .try_get::<String, _>("published_question_id")
                    .map_err(map_sqlx_error)?
                    .parse()
                    .map_err(|_| invalid())?,
                revision_number: QuestionRevisionNumber::new(
                    u32::try_from(
                        row.try_get::<i32, _>("question_revision_number")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| invalid())?,
                )
                .map_err(|_| invalid())?,
            };
            if !unique.insert(member.clone()) {
                return Err(invalid());
            }
            members.push(member);
        }
        result.insert((question_pool_id, question_pool_edit_number), members);
    }
    Ok(result)
}

async fn current_actor(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<question_model::AccountId, StoreError> {
    let row = sqlx::query("SELECT ple_api.current_session_account_id() AS account_id")
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::Forbidden)?;
    parse_account_id(row.try_get("account_id").map_err(map_sqlx_error)?)
}

fn parse_blueprint_course_id(value: String) -> Result<BlueprintCourseId, StoreError> {
    value
        .parse()
        .map_err(|_| StoreError::InvalidRecord("Blueprint Course ID is invalid".to_string()))
}

fn blueprint_revision_number(value: i64) -> Result<BlueprintRevisionNumber, StoreError> {
    let number = u64::try_from(value)
        .map_err(|_| StoreError::InvalidRecord("Blueprint Revision is invalid".to_string()))?;
    BlueprintRevisionNumber::new(number)
        .ok_or_else(|| StoreError::InvalidRecord("Blueprint Revision is invalid".to_string()))
}

fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Blueprint fork UUID randomness unavailable".to_string())
    })
}
