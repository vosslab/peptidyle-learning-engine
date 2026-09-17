//! Exact immutable Blueprint Assessment-owned Pool materialization.
use super::connection::map_sqlx_error;
use crate::blueprint_course::StoredBlueprintAssessmentEntry;
use crate::{CourseInstancePoolIdIssuer, StoreError, StoredBlueprintCourseContent};
use question_model::{
    BlueprintAssessmentReference, BlueprintCourseReference, BlueprintPoolInputChoice,
    BlueprintRevision, QuestionPoolRevisionNumber, QuestionPoolRevisionReference,
    QuestionRevisionReference,
};
use sqlx::{Postgres, Row, Transaction};

pub(super) async fn materialize_imported_pools(
    transaction: &mut Transaction<'_, Postgres>,
    content: &mut StoredBlueprintCourseContent,
    issuer: Option<&dyn CourseInstancePoolIdIssuer>,
    bloom_receipts: &mut crate::PoolBloomPreparationReceipts,
) -> Result<(), StoreError> {
    for module in &mut content.modules {
        for assessment in &mut module.assessments {
            for entry in &mut assessment.content.entries {
                if let StoredBlueprintAssessmentEntry::Pool {
                    question_pool_revision,
                    ..
                } = entry
                {
                    *question_pool_revision = import(
                        transaction,
                        question_pool_revision,
                        issuer,
                        bloom_receipts.take_next()?,
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}

async fn import(
    transaction: &mut Transaction<'_, Postgres>,
    source: &QuestionPoolRevisionReference,
    issuer: Option<&dyn CourseInstancePoolIdIssuer>,
    bloom_receipt: crate::BloomPreparationReceiptId,
) -> Result<QuestionPoolRevisionReference, StoreError> {
    let child = issuer
        .ok_or_else(|| {
            StoreError::InvalidRecord("Question Pool fork identity issuer is unavailable".into())
        })?
        .issue_question_pool_id()?;
    // ASVS 1.2.4 / 2.3.3: exact source pins and fresh children share the Blueprint transaction.
    sqlx::query("SELECT ple_api.fork_blueprint_question_pool($1,$2,$3,$4,$5)")
        .bind(source.question_pool_id.as_str())
        .bind(source.revision_number.get() as i64)
        .bind(super::blueprint_course::random_uuid()?)
        .bind(child.as_str())
        .bind(bloom_receipt.as_uuid())
        .execute(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    Ok(QuestionPoolRevisionReference {
        question_pool_id: child,
        revision_number: QuestionPoolRevisionNumber::new(1)
            .map_err(|_| StoreError::InvalidRecord("Question Pool Revision".into()))?,
    })
}

pub(super) async fn materialize_authoring_pools(
    transaction: &mut Transaction<'_, Postgres>,
    content: &mut StoredBlueprintCourseContent,
    choices: Vec<Vec<Vec<BlueprintPoolInputChoice>>>,
    context: Option<(BlueprintCourseReference, BlueprintRevision)>,
    prior: Option<&StoredBlueprintCourseContent>,
    issuer: Option<&dyn CourseInstancePoolIdIssuer>,
    bloom_receipts: &mut crate::PoolBloomPreparationReceipts,
) -> Result<(), StoreError> {
    // ASVS 2.2.2 / 8.2.2: validate every retained choice before any Pool writes.
    let mut seen = std::collections::BTreeSet::new();
    for (module, choices) in content.modules.iter().zip(&choices) {
        for (assessment, choices) in module.assessments.iter().zip(choices) {
            for choice in choices {
                if let BlueprintPoolInputChoice::Retained {
                    question_pool_revision,
                    ..
                } = choice
                {
                    let old = prior
                        .and_then(|prior| {
                            prior.modules.iter().flat_map(|m| &m.assessments).find(|a| {
                                a.blueprint_assessment_reference
                                    == assessment.blueprint_assessment_reference
                            })
                        })
                        .ok_or(StoreError::Forbidden)?;
                    if !old.content.entries.iter().any(|entry| matches!(entry,
                        StoredBlueprintAssessmentEntry::Pool { question_pool_revision: pin, .. } if pin == question_pool_revision))
                        || !seen.insert(question_pool_revision.question_pool_id.clone()) { return Err(StoreError::Forbidden); }
                    let (reference, revision) = context.as_ref().ok_or(StoreError::Forbidden)?;
                    members(
                        transaction,
                        reference,
                        assessment.blueprint_assessment_reference,
                        question_pool_revision,
                        Some(*revision),
                    )
                    .await?;
                }
            }
        }
    }
    for (module, choices) in content.modules.iter_mut().zip(choices) {
        for (assessment, choices) in module.assessments.iter_mut().zip(choices) {
            let mut choices = choices.into_iter();
            for entry in &mut assessment.content.entries {
                let StoredBlueprintAssessmentEntry::Pool {
                    question_pool_revision: pin,
                    ..
                } = entry
                else {
                    continue;
                };
                match choices.next().ok_or(StoreError::Forbidden)? {
                    BlueprintPoolInputChoice::Import {
                        question_pool_revision,
                    } => {
                        *pin = import(
                            transaction,
                            &question_pool_revision,
                            issuer,
                            bloom_receipts.take_next()?,
                        )
                        .await?
                    }
                    BlueprintPoolInputChoice::Retained {
                        question_pool_revision,
                        members: replacement,
                        interchangeability_attested,
                    } => {
                        if let Some(replacement) = replacement {
                            let (reference, revision) =
                                context.as_ref().ok_or(StoreError::Forbidden)?;
                            let old = members(
                                transaction,
                                reference,
                                assessment.blueprint_assessment_reference,
                                &question_pool_revision,
                                Some(*revision),
                            )
                            .await?;
                            if replacement != old {
                                let ids: Vec<_> = replacement
                                    .iter()
                                    .map(|q| q.question_id.as_str().to_owned())
                                    .collect();
                                let revisions: Vec<_> = replacement
                                    .iter()
                                    .map(|q| q.revision_number.get() as i32)
                                    .collect();
                                let bloom_receipt = bloom_receipts.take_next()?;
                                let row = sqlx::query("SELECT ple_api.append_blueprint_pool_revision($1,$2,$3,$4,$5,$6,$7,$8,$9) AS revision")
                                    .bind(reference.as_string()).bind(assessment.blueprint_assessment_reference.as_uuid())
                                    .bind(revision.value() as i64).bind(pin.question_pool_id.as_str())
                                    .bind(pin.revision_number.get() as i64).bind(ids).bind(revisions).bind(interchangeability_attested)
                                    .bind(bloom_receipt.as_uuid())
                                    .fetch_one(&mut **transaction).await.map_err(map_sqlx_error)?;
                                pin.revision_number = QuestionPoolRevisionNumber::new(
                                    row.try_get::<i64, _>("revision").map_err(map_sqlx_error)?
                                        as u64,
                                )
                                .map_err(|_| {
                                    StoreError::InvalidRecord("Question Pool Revision".into())
                                })?;
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub(super) async fn members(
    transaction: &mut Transaction<'_, Postgres>,
    reference: &BlueprintCourseReference,
    assessment: BlueprintAssessmentReference,
    pin: &QuestionPoolRevisionReference,
    write: Option<BlueprintRevision>,
) -> Result<Vec<QuestionRevisionReference>, StoreError> {
    let rows = sqlx::query("SELECT * FROM ple_api.blueprint_pool_members($1,$2,$3,$4,$5,$6)")
        .bind(reference.as_string())
        .bind(assessment.as_uuid())
        .bind(pin.question_pool_id.as_str())
        .bind(write.is_some())
        .bind(write.map(|r| r.value() as i64))
        .bind(Some(pin.revision_number.get() as i64))
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    rows.into_iter()
        .map(|row| {
            Ok(QuestionRevisionReference {
                question_id: row
                    .try_get::<String, _>("question_id")
                    .map_err(map_sqlx_error)?
                    .parse()
                    .map_err(|_| StoreError::InvalidRecord("Question ID".into()))?,
                revision_number: question_model::QuestionRevisionNumber::new(
                    row.try_get::<i32, _>("question_revision_number")
                        .map_err(map_sqlx_error)? as u32,
                )
                .map_err(|_| StoreError::InvalidRecord("Question Revision".into()))?,
            })
        })
        .collect()
}
