//! PostgreSQL persistence for Blueprint Revisions and lineage metadata.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use question_model::{
    AccountId, BlueprintAvailability, BlueprintCourseId, BlueprintCourseReadAccess,
    BlueprintEditNumber, BlueprintMetadataState, BlueprintRevision, BlueprintRevisionReference,
    CanonicalBlueprintCourse, CreateBlueprintCourseInput, CreateBlueprintCourseReceipt, QuestionId,
    QuestionPoolEditNumber, QuestionRevisionNumber, QuestionRevisionReference,
    RenameBlueprintCourseInput, ReplaceBlueprintCourseContentInput, RequestChecksum,
    SaveBlueprintCourseReceipt, Timestamp,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Row, Transaction, types::Json};

use super::Pool;
use super::connection::{map_sqlx_error, parse_account_id};
use crate::blueprint_course::StoredBlueprintRevision;
use crate::{
    BlueprintCourseStore, CourseInstancePoolIdIssuer, SessionTokenHash, StoreError,
    StoredBlueprintCourse, StoredBlueprintCourseContent, StoredBlueprintCourseSummary,
};

mod classification;
mod promotion;
mod search;

pub(in crate::postgres) use classification::{classification_tags, decode_classification};

/// PostgreSQL Store for Blueprint lineages visible to active Instructors.
#[derive(Clone)]
pub struct PostgresBlueprintCourseStore {
    pool: Pool,
    pub(super) pool_id_issuer: Option<Arc<dyn CourseInstancePoolIdIssuer>>,
}

impl PostgresBlueprintCourseStore {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            pool_id_issuer: None,
        }
    }

    /// Adds the same fresh Pool-fork identity capability used by initial adoption.
    pub fn with_question_pool_id_issuer(
        mut self,
        pool_id_issuer: Arc<dyn CourseInstancePoolIdIssuer>,
    ) -> Self {
        self.pool_id_issuer = Some(pool_id_issuer);
        self
    }

    pub(super) async fn begin_authenticated_application_transaction(
        &self,
        token_hash: SessionTokenHash,
    ) -> Result<Transaction<'_, Postgres>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(map_sqlx_error)?;
        sqlx::query("SET LOCAL ROLE ple_auth")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let session = sqlx::query(
            "SELECT session_id FROM ple_api.resolve_and_install_session(decode($1, 'hex'))",
        )
        .bind(token_hash.to_string())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        if session.is_none() {
            return Err(StoreError::Forbidden);
        }
        sqlx::query("SET LOCAL ROLE ple_app")
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        Ok(transaction)
    }
}

impl PostgresBlueprintCourseStore {
    // One ordinary Save with caller-owned transaction and already trusted content.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn save_trusted_content(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        reference_value: BlueprintCourseId,
        expected_revision: BlueprintRevision,
        request_checksum: RequestChecksum,
        actor: AccountId,
        prior: &StoredBlueprintCourseContent,
        content: &StoredBlueprintCourseContent,
        daughters: Vec<sqlx::postgres::PgRow>,
        bloom_receipts: &mut crate::PoolBloomPreparationReceipts,
    ) -> Result<SaveBlueprintCourseReceipt, StoreError> {
        let prior_sources: BTreeSet<_> = prior
            .modules
            .iter()
            .flat_map(|module| &module.assessments)
            .map(|assessment| assessment.blueprint_assessment_id)
            .collect();
        let mut additions = content.clone();
        for module in &mut additions.modules {
            module
                .assessments
                .retain(|assessment| !prior_sources.contains(&assessment.blueprint_assessment_id));
        }
        let mut materialized_daughters = Vec::with_capacity(daughters.len());
        for daughter in daughters {
            let course_id: String = daughter.try_get("course_id").map_err(map_sqlx_error)?;
            materialized_daughters.push(json!({
                "course_id": course_id,
                "assessments": super::course_blueprint_adoption::materialize(
                    &additions, self.pool_id_issuer.as_deref(), bloom_receipts
                )?,
            }));
        }
        let encoded = encode_content(content)?;
        let row = sqlx::query(
            "SELECT resulting_blueprint_revision_number, changed, \
             (EXTRACT(EPOCH FROM accepted_at) * 1000)::bigint AS accepted_at_millis \
             FROM ple_api.save_blueprint_course($1, $2, $3, $4, $5, $6)",
        )
        .bind(reference_value.as_string())
        .bind(revision_number(expected_revision)?)
        .bind(request_checksum.into_bytes().to_vec())
        .bind(encoded)
        .bind(content.checksum()?.as_bytes().to_vec())
        .bind(Json(Value::Array(materialized_daughters)))
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        let receipt = SaveBlueprintCourseReceipt {
            blueprint_revision: BlueprintRevisionReference {
                blueprint_course_id: reference_value,
                revision: revision(
                    row.try_get("resulting_blueprint_revision_number")
                        .map_err(map_sqlx_error)?,
                )?,
            },
            changed: row.try_get("changed").map_err(map_sqlx_error)?,
            actor,
            request_checksum,
            accepted_at: timestamp(row.try_get("accepted_at_millis").map_err(map_sqlx_error)?)?,
        };
        Ok(receipt)
    }

    pub(super) async fn rename_in_transaction(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
        reference_value: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        input: RenameBlueprintCourseInput,
    ) -> Result<BlueprintMetadataState, StoreError> {
        let row = sqlx::query(
            "SELECT * \
             FROM ple_api.rename_blueprint_course($1, $2, $3, $4)",
        )
        .bind(reference_value.as_string())
        .bind(expected_edit_number.as_i64())
        .bind(input.short_name)
        .bind(input.long_name)
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
        let state = decode_metadata_state(&row)?;
        Ok(state)
    }
}

#[async_trait]
impl BlueprintCourseStore for PostgresBlueprintCourseStore {
    async fn update_blueprint_classification(
        &self,
        session: SessionTokenHash,
        id: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        classification: question_model::CourseClassification,
    ) -> Result<BlueprintMetadataState, StoreError> {
        classification
            .validate()
            .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        sqlx::query("SELECT * FROM ple_api.update_blueprint_classification($1,$2,$3,$4,$5,$6,$7)")
            .bind(id.as_string())
            .bind(expected_edit_number.as_i64())
            .bind(classification.discipline_uuid)
            .bind(classification.subject_uuid)
            .bind(classification.topic_uuid)
            .bind(classification.subtopic_uuid)
            .bind(classification_tags(&classification))
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        // Read the complete accepted metadata under the same transaction.
        let row = sqlx::query("SELECT * FROM ple_api.load_blueprint_course($1)")
            .bind(id.as_string())
            .fetch_one(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let result = decode_metadata_state(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
    async fn load_blueprint_pool_members(
        &self,
        session: SessionTokenHash,
        id: BlueprintCourseId,
        assessment: question_model::BlueprintAssessmentId,
        question_pool_id: QuestionId,
    ) -> Result<crate::StoredBlueprintPoolMembers, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let rows =
            sqlx::query("SELECT * FROM ple_api.blueprint_pool_members($1,$2,$3,false,NULL,NULL)")
                .bind(id.as_string())
                .bind(assessment.as_uuid())
                .bind(question_pool_id.as_str())
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let number = rows
            .first()
            .ok_or(StoreError::NotFound)?
            .try_get::<i64, _>("question_pool_edit_number")
            .map_err(map_sqlx_error)?;
        let question_pool_edit_number =
            QuestionPoolEditNumber::new(number as u64).map_err(|_| invalid("Pool Edit Number"))?;
        let members = rows
            .into_iter()
            .map(|row| {
                Ok(QuestionRevisionReference {
                    question_id: row
                        .try_get::<String, _>("question_id")
                        .map_err(map_sqlx_error)?
                        .parse()
                        .map_err(|_| invalid("Question ID"))?,
                    revision_number: QuestionRevisionNumber::new(
                        row.try_get::<i32, _>("question_revision_number")
                            .map_err(map_sqlx_error)? as u32,
                    )
                    .map_err(|_| invalid("Question Revision"))?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(crate::StoredBlueprintPoolMembers {
            question_pool_id,
            question_pool_edit_number,
            members,
        })
    }
    async fn apply_blueprint_fork(
        &self,
        session: SessionTokenHash,
        input: crate::ApplyBlueprintForkInput,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<crate::ApplyBlueprintForkResult, StoreError> {
        self.apply_fork_in_transaction(session, input, bloom_receipts)
            .await
    }
    async fn list_blueprint_courses(
        &self,
        session: SessionTokenHash,
        request: crate::BlueprintCourseListRequest,
    ) -> Result<crate::Page<StoredBlueprintCourseSummary>, StoreError> {
        self.list_discovery_page(session, request).await
    }

    async fn load_blueprint_course(
        &self,
        session: SessionTokenHash,
        id: BlueprintCourseId,
    ) -> Result<StoredBlueprintCourse, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_blueprint_course($1)")
            .bind(id.as_string())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let record = row
            .as_ref()
            .map(decode_course)
            .transpose()?
            .ok_or(StoreError::NotFound)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(record)
    }

    async fn load_blueprint_revision(
        &self,
        session: SessionTokenHash,
        id: BlueprintRevisionReference,
    ) -> Result<StoredBlueprintRevision, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query("SELECT * FROM ple_api.load_blueprint_revision($1, $2)")
            .bind(id.blueprint_course_id.as_string())
            .bind(revision_number(id.revision)?)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let row = row.ok_or(StoreError::NotFound)?;
        let Json(content): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
        let content = decode_revision_content(
            content,
            &row.try_get::<Vec<u8>, _>("content_checksum")
                .map_err(map_sqlx_error)?,
        )?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(StoredBlueprintRevision {
            reference: id,
            content,
        })
    }

    async fn export_blueprint_course(
        &self,
        session: SessionTokenHash,
        id: BlueprintCourseId,
    ) -> Result<CanonicalBlueprintCourse, StoreError> {
        // ASVS 8.2.2 and 8.3.1: the existing current-lineage read applies
        // object authorization before any reusable content is projected.
        let record = self.load_blueprint_course(session, id).await?;
        let content = record.content.to_domain()?;
        Ok(CanonicalBlueprintCourse::export(
            record.short_name,
            record.long_name,
            record.classification,
            &content,
        ))
    }

    async fn import_blueprint_course(
        &self,
        session: SessionTokenHash,
        request_checksum: RequestChecksum,
        exchange: CanonicalBlueprintCourse,
        bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<CreateBlueprintCourseReceipt, StoreError> {
        let input = exchange
            .into_create_input()
            .map_err(|_| invalid("canonical Blueprint exchange"))?;
        // ASVS 2.3.3: reuse the one atomic create transaction. It owns the
        // actor, Private lifecycle state, Revision 1, fresh local identities,
        // exact pin validation, and Pool forking.
        self.create_blueprint_course(session, request_checksum, input, bloom_receipts)
            .await
    }

    async fn create_blueprint_course(
        &self,
        session: SessionTokenHash,
        request_checksum: RequestChecksum,
        input: CreateBlueprintCourseInput,
        mut bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<CreateBlueprintCourseReceipt, StoreError> {
        input.validate().map_err(invalid_input)?;
        let short_name = input.short_name.clone();
        let long_name = input.long_name.clone();
        let classification = input.classification.clone();
        let pool_choices = input
            .modules
            .iter()
            .map(|module| {
                module
                    .assessments
                    .iter()
                    .map(|assessment| {
                        assessment
                            .entries
                            .iter()
                            .filter_map(|entry| match entry {
                                question_model::BlueprintAssessmentEntryInput::Pool(pool) => {
                                    Some(pool.pool.clone())
                                }
                                _ => None,
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let requested =
            StoredBlueprintCourseContent::requested_question_revisions_from_create(&input);
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let actor = current_actor(&mut transaction).await?;
        if let Some(row) =
            sqlx::query("SELECT * FROM ple_api.blueprint_pool_write_receipt(NULL,$1)")
                .bind(request_checksum.into_bytes().to_vec())
                .fetch_optional(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?
        {
            let receipt = CreateBlueprintCourseReceipt {
                blueprint_revision: BlueprintRevisionReference {
                    blueprint_course_id: reference(
                        row.try_get("blueprint_course_id").map_err(map_sqlx_error)?,
                    )?,
                    revision: revision(row.try_get("revision_number").map_err(map_sqlx_error)?)?,
                },
                blueprint_edit_number: blueprint_edit_number(
                    row.try_get("blueprint_edit_number")
                        .map_err(map_sqlx_error)?,
                ),
                actor,
                request_checksum,
                accepted_at: timestamp(row.try_get("accepted_at_millis").map_err(map_sqlx_error)?)?,
            };
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(receipt);
        }
        validate_question_references(&mut transaction, requested, None).await?;
        let mut content = StoredBlueprintCourseContent::from_create(input, &BTreeMap::new())?;
        super::blueprint_pools::materialize_authoring_pools(
            &mut transaction,
            &mut content,
            pool_choices,
            None,
            None,
            self.pool_id_issuer.as_deref(),
            &mut bloom_receipts,
        )
        .await?;
        let encoded = encode_content(&content)?;
        let row = sqlx::query(
            "SELECT blueprint_course_id, blueprint_revision_number, blueprint_edit_number, \
             (EXTRACT(EPOCH FROM accepted_at) * 1000)::bigint AS accepted_at_millis \
             FROM ple_api.create_blueprint_course($1, $2, $3, $4, $5, $6, $7,$8,$9,$10,$11)",
        )
        .bind(random_uuid()?)
        .bind(request_checksum.into_bytes().to_vec())
        .bind(short_name)
        .bind(long_name)
        .bind(encoded)
        .bind(content.checksum()?.as_bytes().to_vec())
        .bind(classification.discipline_uuid)
        .bind(classification.subject_uuid)
        .bind(classification.topic_uuid)
        .bind(classification.subtopic_uuid)
        .bind(classification_tags(&classification))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let blueprint = reference(row.try_get("blueprint_course_id").map_err(map_sqlx_error)?)?;
        let receipt = CreateBlueprintCourseReceipt {
            blueprint_revision: BlueprintRevisionReference {
                blueprint_course_id: blueprint,
                revision: revision(
                    row.try_get("blueprint_revision_number")
                        .map_err(map_sqlx_error)?,
                )?,
            },
            blueprint_edit_number: blueprint_edit_number(
                row.try_get("blueprint_edit_number")
                    .map_err(map_sqlx_error)?,
            ),
            actor,
            request_checksum,
            accepted_at: timestamp(row.try_get("accepted_at_millis").map_err(map_sqlx_error)?)?,
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn save_blueprint_course(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseId,
        expected_revision: BlueprintRevision,
        request_checksum: RequestChecksum,
        input: ReplaceBlueprintCourseContentInput,
        mut bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<SaveBlueprintCourseReceipt, StoreError> {
        input.validate().map_err(invalid_input)?;
        let pool_choices = input
            .modules
            .iter()
            .map(|module| {
                module
                    .assessments
                    .iter()
                    .map(|assessment| {
                        assessment
                            .content
                            .entries
                            .iter()
                            .filter_map(|entry| match entry {
                                question_model::BlueprintAssessmentEntryInput::Pool(pool) => {
                                    Some(pool.pool.clone())
                                }
                                _ => None,
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let actor = current_actor(&mut transaction).await?;
        if let Some(row) = sqlx::query("SELECT * FROM ple_api.blueprint_pool_write_receipt($1,$2)")
            .bind(reference_value.as_string())
            .bind(request_checksum.into_bytes().to_vec())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?
        {
            let receipt = SaveBlueprintCourseReceipt {
                blueprint_revision: BlueprintRevisionReference {
                    blueprint_course_id: reference_value,
                    revision: revision(row.try_get("revision_number").map_err(map_sqlx_error)?)?,
                },
                changed: row.try_get("changed").map_err(map_sqlx_error)?,
                actor,
                request_checksum,
                accepted_at: timestamp(row.try_get("accepted_at_millis").map_err(map_sqlx_error)?)?,
            };
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(receipt);
        }
        // ASVS 2.3.3: hold the parent lock before resolving and materializing
        // additions; Course adoption takes this same lock and checks the head.
        let daughters =
            sqlx::query("SELECT course_id FROM ple_api.list_blueprint_daughter_course_ids($1)")
                .bind(reference_value.as_string())
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let prior =
            load_revision_content(&mut transaction, reference_value.clone(), expected_revision)
                .await?;
        let requested =
            StoredBlueprintCourseContent::requested_question_revisions_from_replace(&input);
        validate_question_references(&mut transaction, requested, Some(&prior)).await?;
        let mut content =
            StoredBlueprintCourseContent::from_replace(input, &prior, &BTreeMap::new())?;
        super::blueprint_pools::materialize_authoring_pools(
            &mut transaction,
            &mut content,
            pool_choices,
            Some((reference_value.clone(), expected_revision)),
            Some(&prior),
            self.pool_id_issuer.as_deref(),
            &mut bloom_receipts,
        )
        .await?;
        let receipt = self
            .save_trusted_content(
                &mut transaction,
                reference_value,
                expected_revision,
                request_checksum,
                actor,
                &prior,
                &content,
                daughters,
                &mut bloom_receipts,
            )
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }

    async fn rename_blueprint_course(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        input: RenameBlueprintCourseInput,
    ) -> Result<BlueprintMetadataState, StoreError> {
        question_model::validate_blueprint_course_title(&input.short_name)
            .map_err(|_| invalid("Blueprint short name"))?;
        question_model::validate_blueprint_course_title(&input.long_name)
            .map_err(|_| invalid("Blueprint long name"))?;
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let state = self
            .rename_in_transaction(
                &mut transaction,
                reference_value,
                expected_edit_number,
                input,
            )
            .await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(state)
    }

    async fn archive_blueprint(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        confirmation_title: &str,
    ) -> Result<BlueprintMetadataState, StoreError> {
        self.set_availability(
            session,
            reference_value,
            expected_edit_number,
            "archived",
            Some(confirmation_title),
        )
        .await
    }

    async fn publish_blueprint(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
    ) -> Result<BlueprintMetadataState, StoreError> {
        self.set_availability(
            session,
            reference_value,
            expected_edit_number,
            "public",
            None,
        )
        .await
    }

    async fn restore_blueprint(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
    ) -> Result<BlueprintMetadataState, StoreError> {
        self.set_availability(
            session,
            reference_value,
            expected_edit_number,
            "public",
            None,
        )
        .await
    }

    async fn return_blueprint_to_private(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
    ) -> Result<BlueprintMetadataState, StoreError> {
        self.set_availability(
            session,
            reference_value,
            expected_edit_number,
            "private",
            None,
        )
        .await
    }
}

impl PostgresBlueprintCourseStore {
    async fn set_availability(
        &self,
        session: SessionTokenHash,
        reference_value: BlueprintCourseId,
        expected_edit_number: BlueprintEditNumber,
        availability: &'static str,
        archive_confirmation_title: Option<&str>,
    ) -> Result<BlueprintMetadataState, StoreError> {
        // ASVS 2.3.1, 2.3.3: the authenticated database transition is the
        // only lifecycle authority; callers can request only typed methods.
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let row = sqlx::query(
            "SELECT * \
             FROM ple_api.set_blueprint_availability($1, $2, $3, $4)",
        )
        .bind(reference_value.as_string())
        .bind(expected_edit_number.as_i64())
        .bind(availability)
        .bind(archive_confirmation_title)
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let result = decode_metadata_state(&row)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(result)
    }
}

async fn load_revision_content(
    transaction: &mut Transaction<'_, Postgres>,
    reference_value: BlueprintCourseId,
    revision_value: BlueprintRevision,
) -> Result<StoredBlueprintCourseContent, StoreError> {
    let row = sqlx::query("SELECT * FROM ple_api.load_blueprint_revision($1, $2)")
        .bind(reference_value.as_string())
        .bind(revision_number(revision_value)?)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or(StoreError::NotFound)?;
    let Json(content): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
    let content = decode_revision_content(
        content,
        &row.try_get::<Vec<u8>, _>("content_checksum")
            .map_err(map_sqlx_error)?,
    )?;
    Ok(content)
}

pub(super) async fn current_actor(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<AccountId, StoreError> {
    let account_id = sqlx::query_scalar("SELECT ple_api.current_session_account_id()")
        .fetch_one(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    parse_account_id(account_id)
}

/// Validate each submitted immutable Question pin without replacing it with a
/// latest or retained-by-ID Revision. Only exact pins already owned by the
/// expected Blueprint Revision may survive an archived Question lineage.
async fn validate_question_references(
    transaction: &mut Transaction<'_, Postgres>,
    requested: Vec<QuestionRevisionReference>,
    prior: Option<&StoredBlueprintCourseContent>,
) -> Result<(), StoreError> {
    let retained = prior
        .into_iter()
        .flat_map(|content| content.modules.iter())
        .flat_map(|module| module.assessments.iter())
        .flat_map(|assessment| assessment.content.entries.iter())
        .filter_map(|entry| match entry {
            crate::StoredBlueprintAssessmentEntry::Fixed {
                question_revision, ..
            } => Some(question_revision.clone()),
            crate::StoredBlueprintAssessmentEntry::Pool { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    // ASVS 2.2.1 and 2.2.3: validate the exact fixed-Question subset.
    // Pool-only content has an empty subset; Pool materialization has its own
    // exact-reference boundary. PostgreSQL rechecks selection while locked.
    for reference in requested.into_iter().collect::<BTreeSet<_>>() {
        let row = sqlx::query(
            "SELECT question_id, revision_number, availability
             FROM ple_api.load_question_library_revision($1, $2)",
        )
        .bind(reference.question_id.as_str())
        .bind(
            i32::try_from(reference.revision_number.get())
                .map_err(|_| invalid("Published Question Revision"))?,
        )
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?
        .ok_or_else(|| invalid("Published Question exact revision"))?;
        let question_id = row
            .try_get::<String, _>("question_id")
            .map_err(map_sqlx_error)?
            .parse::<QuestionId>()
            .map_err(|_| invalid("Published Question ID"))?;
        let revision_number = row
            .try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?;
        if question_id != reference.question_id
            || u32::try_from(revision_number).ok() != Some(reference.revision_number.get())
        {
            return Err(invalid("Published Question exact revision"));
        }
        let availability = row
            .try_get::<String, _>("availability")
            .map_err(map_sqlx_error)?;
        if availability != "available" && !retained.contains(&reference) {
            return Err(StoreError::InvalidRecord(
                "Blueprint Course requires currently available Published Questions".to_string(),
            ));
        }
    }
    Ok(())
}

fn decode_summary(row: &sqlx::postgres::PgRow) -> Result<StoredBlueprintCourseSummary, StoreError> {
    Ok(StoredBlueprintCourseSummary {
        classification: decode_classification(row)?,
        total_adoptions: u64::try_from(
            row.try_get::<i64, _>("total_adoptions")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Blueprint adoption count"))?,
        total_students_ever_enrolled: u64::try_from(
            row.try_get::<i64, _>("total_students_ever_enrolled")
                .map_err(map_sqlx_error)?,
        )
        .map_err(|_| invalid("Blueprint enrollment count"))?,
        id: reference(row.try_get("blueprint_course_id").map_err(map_sqlx_error)?)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        blueprint_edit_number: blueprint_edit_number(
            row.try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        current_revision: revision(
            row.try_get("current_blueprint_revision_number")
                .map_err(map_sqlx_error)?,
        )?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
    })
}

fn decode_course(row: &sqlx::postgres::PgRow) -> Result<StoredBlueprintCourse, StoreError> {
    let fork_source_blueprint_course_id: Option<String> = row
        .try_get("fork_source_blueprint_course_id")
        .map_err(map_sqlx_error)?;
    let fork_source_revision: Option<i64> = row
        .try_get("fork_source_revision_number")
        .map_err(map_sqlx_error)?;
    let fork_source = match (fork_source_blueprint_course_id, fork_source_revision) {
        (Some(source), Some(number)) => Some(BlueprintRevisionReference {
            blueprint_course_id: reference(source)?,
            revision: revision(number)?,
        }),
        (None, None) => None,
        _ => return Err(invalid("fork origin")),
    };
    let Json(encoded): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
    let content = decode_revision_content(
        encoded,
        &row.try_get::<Vec<u8>, _>("content_checksum")
            .map_err(map_sqlx_error)?,
    )?;
    Ok(StoredBlueprintCourse {
        classification: decode_classification(row)?,
        id: reference(row.try_get("blueprint_course_id").map_err(map_sqlx_error)?)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        blueprint_edit_number: blueprint_edit_number(
            row.try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        current_revision: revision(
            row.try_get("current_blueprint_revision_number")
                .map_err(map_sqlx_error)?,
        )?,
        read_access: read_access(row.try_get("is_owner").map_err(map_sqlx_error)?),
        content,
        fork_source,
    })
}

pub(super) fn encode_content(content: &StoredBlueprintCourseContent) -> Result<Value, StoreError> {
    serde_json::to_value(content).map_err(|_| invalid("Blueprint Content"))
}

fn decode_stored_content(encoded: Value) -> Result<StoredBlueprintCourseContent, StoreError> {
    serde_json::from_value(encoded).map_err(|_| invalid("Blueprint Content"))
}

/// Decode the exact retained content through the ordinary checksum boundary.
pub(super) fn decode_revision_content(
    encoded: Value,
    expected: &[u8],
) -> Result<StoredBlueprintCourseContent, StoreError> {
    let content = decode_stored_content(encoded)?;
    verify_content_checksum(&content, expected)?;
    Ok(content)
}

fn verify_content_checksum(
    content: &StoredBlueprintCourseContent,
    expected: &[u8],
) -> Result<(), StoreError> {
    (expected == content.checksum()?.as_bytes())
        .then_some(())
        .ok_or_else(|| invalid("Blueprint Content Checksum"))
}

fn decode_metadata_state(
    row: &sqlx::postgres::PgRow,
) -> Result<BlueprintMetadataState, StoreError> {
    Ok(BlueprintMetadataState {
        classification: decode_classification(row)?,
        short_name: row.try_get("short_name").map_err(map_sqlx_error)?,
        long_name: row.try_get("long_name").map_err(map_sqlx_error)?,
        availability: availability_value(row.try_get("availability").map_err(map_sqlx_error)?)?,
        blueprint_edit_number: blueprint_edit_number(
            row.try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
    })
}

fn read_access(is_owner: bool) -> BlueprintCourseReadAccess {
    if is_owner {
        BlueprintCourseReadAccess::BlueprintCourseOwner
    } else {
        BlueprintCourseReadAccess::ActiveInstructor
    }
}

fn availability_value(value: String) -> Result<BlueprintAvailability, StoreError> {
    match value.as_str() {
        "private" => Ok(BlueprintAvailability::Private),
        "public" => Ok(BlueprintAvailability::Public),
        "archived" => Ok(BlueprintAvailability::Archived),
        _ => Err(invalid("Blueprint availability")),
    }
}

fn reference(value: String) -> Result<BlueprintCourseId, StoreError> {
    value
        .parse()
        .map_err(|_| invalid("Blueprint Course Reference"))
}
fn revision(value: i64) -> Result<BlueprintRevision, StoreError> {
    u64::try_from(value)
        .ok()
        .and_then(BlueprintRevision::new)
        .ok_or_else(|| invalid("Blueprint Revision"))
}
fn blueprint_edit_number(value: i64) -> BlueprintEditNumber {
    BlueprintEditNumber::from_edit_number(value)
}
fn revision_number(value: BlueprintRevision) -> Result<i64, StoreError> {
    i64::try_from(value.value()).map_err(|_| invalid("Blueprint Revision"))
}
fn timestamp(value: i64) -> Result<Timestamp, StoreError> {
    Ok(Timestamp::from_unix_millis(value))
}
fn invalid_input(error: question_model::BlueprintCourseValidationError) -> StoreError {
    StoreError::InvalidRecord(format!("Blueprint Course request is invalid: {error}"))
}
fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}
pub(super) fn random_uuid() -> Result<uuid::Uuid, StoreError> {
    crate::random_uuid::random_uuid_v4(|_| {
        StoreError::Unavailable("Blueprint Course UUID randomness unavailable".to_string())
    })
}
