//! PostgreSQL Course Instance to Blueprint Course publication.

use std::sync::Arc;

use async_trait::async_trait;
use question_model::{
    AccountId, BlueprintAssessmentReference, BlueprintCourseReference, BlueprintMetadataEtag,
    BlueprintModuleReference, BlueprintRevision, BlueprintRevisionReference,
    CourseInstanceReference, CreateBlueprintCourseReceipt, CreateBlueprintFromCourseInstanceInput,
    RequestChecksum, Timestamp,
};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Row, postgres::PgRow, types::Json};

use super::blueprint_course::{
    PostgresBlueprintCourseStore, classification_tags, encode_content, random_uuid,
};
use super::connection::map_sqlx_error;
use super::{Pool, blueprint_pools};
use crate::{
    CourseBlueprintPublicationStore, CourseInstancePoolIdIssuer, SessionTokenHash, StoreError,
    StoredBlueprintAssessment, StoredBlueprintAssessmentContent, StoredBlueprintCourseContent,
    StoredBlueprintModule,
};

/// PostgreSQL implementation of atomic Course reusable-structure publication.
#[derive(Clone)]
pub struct PostgresCourseBlueprintPublicationStore {
    blueprints: PostgresBlueprintCourseStore,
}

impl PostgresCourseBlueprintPublicationStore {
    pub fn new(pool: Pool) -> Self {
        Self {
            blueprints: PostgresBlueprintCourseStore::new(pool),
        }
    }

    pub fn with_question_pool_id_issuer(
        mut self,
        issuer: Arc<dyn CourseInstancePoolIdIssuer>,
    ) -> Self {
        self.blueprints = self.blueprints.with_question_pool_id_issuer(issuer);
        self
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAssessment {
    content: StoredBlueprintAssessmentContent,
}

#[async_trait]
impl CourseBlueprintPublicationStore for PostgresCourseBlueprintPublicationStore {
    async fn create_blueprint_from_course_instance(
        &self,
        session: SessionTokenHash,
        source_course: CourseInstanceReference,
        request_checksum: RequestChecksum,
        input: CreateBlueprintFromCourseInstanceInput,
        mut bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<CreateBlueprintCourseReceipt, StoreError> {
        input.validate().map_err(|error| {
            StoreError::InvalidRecord(format!("Course Blueprint request is invalid: {error}"))
        })?;
        let mut transaction = self
            .blueprints
            .begin_authenticated_application_transaction(session)
            .await?;
        let actor = super::blueprint_course::current_actor(&mut transaction).await?;

        // ASVS 2.3.3: a retry resolves before any fresh child or Pool identity
        // is issued, preventing committed orphan Pool forks.
        if let Some(row) = sqlx::query(
            "SELECT public_reference, blueprint_revision_number, metadata_etag, \
             (EXTRACT(EPOCH FROM accepted_at) * 1000)::bigint AS accepted_at_millis \
             FROM ple_api.course_blueprint_publication_receipt($1, $2)",
        )
        .bind(source_course.as_string())
        .bind(request_checksum.into_bytes().to_vec())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?
        {
            let receipt = decode_receipt(&row, actor, request_checksum)?;
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(receipt);
        }

        // ASVS 8.2.2/8.3.1: the database checks current Course membership and
        // locks Course metadata plus all current Assessment rows before this
        // server-owned projection is decoded.
        let row = sqlx::query(
            "SELECT course_metadata_etag, source_snapshot, source_assessments \
             FROM ple_api.load_course_blueprint_publication_source($1)",
        )
        .bind(source_course.as_string())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let expected_course_metadata_etag: uuid::Uuid = row
            .try_get("course_metadata_etag")
            .map_err(map_sqlx_error)?;
        let Json(source_snapshot): Json<Value> =
            row.try_get("source_snapshot").map_err(map_sqlx_error)?;
        let Json(source_assessments): Json<Vec<SourceAssessment>> =
            row.try_get("source_assessments").map_err(map_sqlx_error)?;

        let mut content = publication_content(source_assessments)?;
        blueprint_pools::materialize_imported_pools(
            &mut transaction,
            &mut content,
            self.blueprints.pool_id_issuer.as_deref(),
            &mut bloom_receipts,
        )
        .await?;
        let encoded = encode_content(&content)?;
        let checksum = content.checksum()?;
        let classification = input.classification;
        let row = sqlx::query(
            "SELECT public_reference, blueprint_revision_number, metadata_etag, \
             (EXTRACT(EPOCH FROM accepted_at) * 1000)::bigint AS accepted_at_millis \
             FROM ple_api.create_blueprint_from_course_instance( \
                 $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        )
        .bind(source_course.as_string())
        .bind(expected_course_metadata_etag)
        .bind(source_snapshot)
        .bind(random_uuid()?)
        .bind(request_checksum.into_bytes().to_vec())
        .bind(input.short_name)
        .bind(input.long_name)
        .bind(encoded)
        .bind(checksum.as_bytes().to_vec())
        .bind(classification.discipline_uuid)
        .bind(classification.subject_uuid)
        .bind(classification.topic_uuid)
        .bind(classification.subtopic_uuid)
        .bind(classification_tags(&classification))
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let receipt = decode_receipt(&row, actor, request_checksum)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(receipt)
    }
}

fn publication_content(
    source: Vec<SourceAssessment>,
) -> Result<StoredBlueprintCourseContent, StoreError> {
    if source.is_empty() {
        return Ok(StoredBlueprintCourseContent {
            modules: Vec::new(),
        });
    }
    let assessments = source
        .into_iter()
        .map(|assessment| {
            Ok(StoredBlueprintAssessment {
                blueprint_assessment_reference: BlueprintAssessmentReference::from_uuid(
                    random_uuid()?,
                ),
                content: assessment.content,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(StoredBlueprintCourseContent {
        modules: vec![StoredBlueprintModule {
            blueprint_module_reference: BlueprintModuleReference::from_uuid(random_uuid()?),
            label: "Assessments".to_string(),
            assessments,
        }],
    })
}

fn decode_receipt(
    row: &PgRow,
    actor: AccountId,
    request_checksum: RequestChecksum,
) -> Result<CreateBlueprintCourseReceipt, StoreError> {
    let reference = row
        .try_get::<String, _>("public_reference")
        .map_err(map_sqlx_error)?
        .parse::<BlueprintCourseReference>()
        .map_err(|_| invalid("Blueprint Course Reference"))?;
    let revision = u64::try_from(
        row.try_get::<i64, _>("blueprint_revision_number")
            .map_err(map_sqlx_error)?,
    )
    .ok()
    .and_then(BlueprintRevision::new)
    .ok_or_else(|| invalid("Blueprint Revision"))?;
    Ok(CreateBlueprintCourseReceipt {
        blueprint_revision: BlueprintRevisionReference {
            reference,
            revision,
        },
        metadata_etag: BlueprintMetadataEtag::from_uuid(
            row.try_get("metadata_etag").map_err(map_sqlx_error)?,
        ),
        actor,
        request_checksum,
        accepted_at: Timestamp::from_unix_millis(
            row.try_get("accepted_at_millis").map_err(map_sqlx_error)?,
        ),
    })
}

fn invalid(label: &str) -> StoreError {
    StoreError::InvalidRecord(format!("database returned an invalid {label}"))
}

#[cfg(test)]
mod tests {
    use super::{SourceAssessment, publication_content};
    use crate::StoredBlueprintAssessmentContent;
    use question_model::{
        AssessmentActivityRules, AssessmentInstructions, AssessmentType,
        BlueprintAssessmentDefaults, LateWorkRule, StudentFeedbackReleaseRule,
    };

    #[test]
    fn empty_course_has_no_synthetic_module() {
        let content = publication_content(Vec::new()).expect("empty Course projection");
        assert!(content.modules.is_empty());
        content
            .checksum()
            .expect("empty Blueprint content checksum");
    }

    #[test]
    fn empty_assessment_uses_the_deterministic_wrapper_module() {
        let content = publication_content(vec![SourceAssessment {
            content: StoredBlueprintAssessmentContent {
                assessment_type: AssessmentType::RegularAssignment,
                title: "Empty Assessment".to_string(),
                instructions: AssessmentInstructions::default(),
                entries: Vec::new(),
                defaults: BlueprintAssessmentDefaults {
                    assessment_attempt_time_limit_seconds: None,
                    attempt_limit: None,
                    late_work_rule: LateWorkRule::Accept,
                    activity_rules: AssessmentActivityRules::default(),
                    student_feedback_release_rule: StudentFeedbackReleaseRule::default(),
                },
            },
        }])
        .expect("empty Assessment projection");

        assert_eq!(content.modules.len(), 1);
        assert_eq!(content.modules[0].label, "Assessments");
        assert!(content.modules[0].assessments[0].content.entries.is_empty());
        content
            .checksum()
            .expect("empty Assessment content checksum");
    }
}
