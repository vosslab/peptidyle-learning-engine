//! Authorized immutable Proposal pins, projected through the canonical exporter.

use async_trait::async_trait;
use question_model::blueprint_course::{
    BlueprintForkApplyAssessmentCopy, BlueprintForkApplyAssessmentDestination,
    BlueprintForkApplyModuleDestination, BlueprintForkApplyModuleLabelCopy,
    BlueprintForkApplyModuleLayout, BlueprintForkApplySelection,
};
use question_model::{
    BlueprintAssessmentId, BlueprintCourseId, BlueprintEditNumber,
    BlueprintModuleReference, BlueprintRevision, BlueprintRevisionReference,
    CanonicalBlueprintCourse, Timestamp,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction, types::Json};
use std::collections::BTreeMap;

use super::{
    blueprint_course::{
        PostgresBlueprintCourseStore, decode_classification, decode_revision_content,
        encode_content,
    },
    connection::{map_sqlx_error, parse_account_id},
};
use crate::{
    AcceptBlueprintChangeProposalInput, AcceptedBlueprintChangeProposal,
    BlueprintChangeProposalAcceptedDecision, BlueprintChangeProposalAcceptedSummary,
    BlueprintChangeProposalDecision, BlueprintChangeProposalListScope,
    BlueprintChangeProposalReview, BlueprintChangeProposalStore, BlueprintChangeProposalSummary,
    CreateBlueprintChangeProposalInput, Cursor, Page, PageRequest, SessionTokenHash, StoreError,
    StoredBlueprintChangeProposal, StoredBlueprintCourseContent, StoredBlueprintModule,
    StoredBlueprintRevision,
};

#[async_trait]
impl BlueprintChangeProposalStore for PostgresBlueprintCourseStore {
    async fn list_blueprint_change_proposals(
        &self,
        session: SessionTokenHash,
        scope: BlueprintChangeProposalListScope,
        page: PageRequest,
    ) -> Result<Page<BlueprintChangeProposalSummary>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let (after_created_at, after_id) = match page.after.as_ref() {
            Some(cursor) => {
                let (timestamp, id) = cursor.as_str().split_once('|').ok_or_else(invalid)?;
                (
                    Some(timestamp.to_owned()),
                    Some(uuid::Uuid::parse_str(id).map_err(|_| invalid())?),
                )
            }
            None => (None, None),
        };
        let (target, mine) = match scope {
            BlueprintChangeProposalListScope::Mine => (None, true),
            BlueprintChangeProposalListScope::Target(reference) => {
                (Some(reference.as_string()), false)
            }
        };
        // ASVS 1.2.4, 8.2.2/3, 8.3.2: bounded participant SQL reauthorizes every page.
        let rows =
            sqlx::query("SELECT * FROM ple_api.list_blueprint_change_proposals($1,$2,$3,$4,$5)")
                .bind(target)
                .bind(mine)
                .bind(after_created_at)
                .bind(after_id)
                .bind(i32::from(page.size.get()) + 1)
                .fetch_all(&mut *transaction)
                .await
                .map_err(map_sqlx_error)?;
        let more = rows.len() > usize::from(page.size.get());
        let retained = rows.iter().take(usize::from(page.size.get()));
        let items = retained
            .clone()
            .map(proposal_summary)
            .collect::<Result<Vec<_>, _>>()?;
        let next_cursor = if more {
            let last = retained.last().ok_or_else(invalid)?;
            let timestamp: String = last.try_get("created_at_key").map_err(map_sqlx_error)?;
            let id: uuid::Uuid = last.try_get("proposal_id").map_err(map_sqlx_error)?;
            Some(Cursor::parse(format!("{timestamp}|{id}")).map_err(|_| invalid())?)
        } else {
            None
        };
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Page { items, next_cursor })
    }

    async fn read_blueprint_change_proposal_review(
        &self,
        session: SessionTokenHash,
        proposal_id: uuid::Uuid,
    ) -> Result<Option<BlueprintChangeProposalReview>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let Some((proposal, [source, target], can_accept)) =
            read_sources_in_transaction(&mut transaction, proposal_id).await?
        else {
            transaction.commit().await.map_err(map_sqlx_error)?;
            return Ok(None);
        };
        // Authorized trees are decoded/checksummed once; exact Pool membership is
        // shared with lineage review, never loaded through ordinary Private history.
        let pool_memberships = super::blueprint_lineage::load_pool_memberships(
            &mut transaction,
            &[source.clone(), target.clone()],
        )
        .await?;
        let accepted = read_accepted_in_transaction(&mut transaction, proposal_id).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(Some(BlueprintChangeProposalReview {
            proposal,
            source,
            target,
            pool_memberships,
            can_accept,
            accepted,
        }))
    }

    async fn create_blueprint_change_proposal(
        &self,
        session: SessionTokenHash,
        input: CreateBlueprintChangeProposalInput,
    ) -> Result<StoredBlueprintChangeProposal, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4, 8.2.2, 2.3.3: bind exact pins; SQL authorizes and locks
        // the reviewed target. The caller cannot supply unchecked proposed JSON.
        let proposal_id: uuid::Uuid = sqlx::query_scalar(
            "SELECT ple_api.create_blueprint_change_proposal($1, $2, $3, $4, $5, $6)",
        )
        .bind(input.source.reference.as_string())
        .bind(revision_number(input.source.revision)?)
        .bind(input.source_blueprint_edit_number.as_i64())
        .bind(input.target.reference.as_string())
        .bind(revision_number(input.target.revision)?)
        .bind(input.target_blueprint_edit_number.as_i64())
        .fetch_one(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let proposal = read_in_transaction(&mut transaction, proposal_id)
            .await?
            .ok_or_else(invalid)?;
        // Projection validates the exact retained content before committing creation.
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(proposal)
    }

    async fn read_blueprint_change_proposal(
        &self,
        session: SessionTokenHash,
        proposal_id: uuid::Uuid,
    ) -> Result<Option<StoredBlueprintChangeProposal>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let proposal = read_in_transaction(&mut transaction, proposal_id).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(proposal)
    }

    async fn accept_blueprint_change_proposal(
        &self,
        session: SessionTokenHash,
        input: AcceptBlueprintChangeProposalInput,
        mut bloom_receipts: crate::PoolBloomPreparationReceipts,
    ) -> Result<AcceptedBlueprintChangeProposal, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        // ASVS 1.2.4, 8.2.2, 2.3.3: bind exact target guards and lock before
        // interpreting selection or allocating any persistent Pool children.
        sqlx::query("SELECT ple_api.lock_blueprint_change_proposal_acceptance($1,$2,$3,$4)")
            .bind(input.proposal_id)
            .bind(input.expected_target.reference.as_string())
            .bind(revision_number(input.expected_target.revision)?)
            .bind(input.expected_target_blueprint_edit_number.as_i64())
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        let rows = sqlx::query("SELECT * FROM ple_api.read_blueprint_change_proposal($1)")
            .bind(input.proposal_id)
            .fetch_all(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;
        if rows.len() != 2 {
            return Err(StoreError::Forbidden);
        }
        let source = stored_content(&rows[0])?;
        let target = stored_content(&rows[1])?;
        let (selection, source_short_name, source_long_name, source_classification) =
            match &input.decision {
                BlueprintChangeProposalDecision::Entire => {
                    (entire_selection(&source), true, true, true)
                }
                BlueprintChangeProposalDecision::Selected {
                    selection,
                    source_short_name,
                    source_long_name,
                    source_classification,
                } => (
                    selection.clone(),
                    *source_short_name,
                    *source_long_name,
                    *source_classification,
                ),
            };
        let new_modules = selection
            .source_module_labels
            .iter()
            .filter(|copy| copy.target_module_reference.is_none())
            .map(|copy| {
                Ok((
                    copy.source_module_reference,
                    BlueprintModuleReference::from_uuid(super::blueprint_course::random_uuid()?),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, StoreError>>()?;
        let new_assessments =
            selection
                .source_assessments
                .iter()
                .filter(|copy| copy.target_assessment_reference.is_none())
                .map(|copy| {
                    Ok((
                        copy.source_assessment_reference,
                        BlueprintAssessmentId::from_uuid(
                            super::blueprint_course::random_uuid()?,
                        ),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, StoreError>>()?;
        let applied = question_model::blueprint_course::apply_blueprint_fork(
            &source.to_domain()?,
            &target.to_domain()?,
            &selection,
            &new_modules,
            &new_assessments,
        )
        .map_err(|error| StoreError::InvalidRecord(error.to_string()))?;
        let short_name: String = rows[if source_short_name { 0 } else { 1 }]
            .try_get("short_name")
            .map_err(map_sqlx_error)?;
        let long_name: String = rows[if source_long_name { 0 } else { 1 }]
            .try_get("long_name")
            .map_err(map_sqlx_error)?;
        let classification =
            decode_classification(&rows[if source_classification { 0 } else { 1 }])?;
        let reviewed =
            CanonicalBlueprintCourse::export(short_name, long_name, classification, &applied);
        let comparison = canonical(&rows[1])?;
        // Local identities are absent from the canonical projection. Check before
        // Pool forks so fresh local allocations cannot manufacture a teaching change.
        if reviewed == comparison {
            return Err(invalid());
        }
        let content_changed = !selection.source_module_labels.is_empty()
            || !selection.source_assessments.is_empty()
            || selection.layout.is_some();
        let copied_assessments = selection
            .source_assessments
            .iter()
            .map(|copy| {
                Ok((
                    copy.target_assessment_reference
                        .or_else(|| {
                            new_assessments
                                .get(&copy.source_assessment_reference)
                                .copied()
                        })
                        .ok_or_else(invalid)?,
                    copy.source_assessment_reference,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, StoreError>>()?;
        let mut content = if content_changed {
            selected_stored_content(&source, &target, &applied, &copied_assessments)?
        } else {
            target.clone()
        };
        if content_changed {
            for module in &mut content.modules {
                for assessment in &mut module.assessments {
                    if copied_assessments.contains_key(&assessment.blueprint_assessment_reference) {
                        let mut copied = StoredBlueprintCourseContent {
                            modules: vec![StoredBlueprintModule {
                                blueprint_module_reference: module.blueprint_module_reference,
                                label: module.label.clone(),
                                assessments: vec![assessment.clone()],
                            }],
                        };
                        super::blueprint_pools::materialize_imported_pools(
                            &mut transaction,
                            &mut copied,
                            self.pool_id_issuer.as_deref(),
                            &mut bloom_receipts,
                        )
                        .await?;
                        *assessment = copied.modules.remove(0).assessments.remove(0);
                    }
                }
            }
        }
        let evidence = BlueprintChangeProposalAcceptedDecision {
            decision: input.decision,
            applied_selection: selection,
            new_modules,
            new_assessments,
        };
        let encoded_evidence = serde_json::to_value(&evidence).map_err(|_| invalid())?;
        let mut digest = Sha256::new();
        digest.update(b"ple:blueprint-proposal-acceptance:");
        digest.update(input.proposal_id.as_bytes());
        digest.update(serde_json::to_vec(&evidence).map_err(|_| invalid())?);
        let checksum: [u8; 32] = digest.finalize().into();
        let encoded_content = if content_changed {
            encode_content(&content)?
        } else {
            // A metadata-only successor retains the exact persisted JSON as well
            // as its domain checksum; do not normalize valid retained encoding.
            let Json(encoded): Json<Value> = rows[1].try_get("content").map_err(map_sqlx_error)?;
            encoded
        };
        sqlx::query(
            "SELECT ple_api.finalize_blueprint_change_proposal_acceptance($1,$2,$3,$4,$5,$6,$7,$8)",
        )
        .bind(input.proposal_id)
        .bind(input.expected_target.reference.as_string())
        .bind(revision_number(input.expected_target.revision)?)
        .bind(input.expected_target_blueprint_edit_number.as_i64())
        .bind(Json(encoded_evidence))
        .bind(checksum.to_vec())
        .bind(Json(encoded_content))
        .bind(content.checksum()?.as_bytes().to_vec())
        .execute(&mut *transaction)
        .await
        .map_err(map_sqlx_error)?;
        let accepted = read_accepted_in_transaction(&mut transaction, input.proposal_id)
            .await?
            .ok_or_else(invalid)?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(accepted)
    }

    async fn read_accepted_blueprint_change_proposal(
        &self,
        session: SessionTokenHash,
        proposal_id: uuid::Uuid,
    ) -> Result<Option<AcceptedBlueprintChangeProposal>, StoreError> {
        let mut transaction = self
            .begin_authenticated_application_transaction(session)
            .await?;
        let accepted = read_accepted_in_transaction(&mut transaction, proposal_id).await?;
        transaction.commit().await.map_err(map_sqlx_error)?;
        Ok(accepted)
    }
}

fn stored_content(row: &sqlx::postgres::PgRow) -> Result<StoredBlueprintCourseContent, StoreError> {
    let Json(content): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
    decode_revision_content(
        content,
        &row.try_get::<Vec<u8>, _>("content_checksum")
            .map_err(map_sqlx_error)?,
    )
}

fn entire_selection(source: &StoredBlueprintCourseContent) -> BlueprintForkApplySelection {
    BlueprintForkApplySelection {
        source_module_labels: source
            .modules
            .iter()
            .map(|module| BlueprintForkApplyModuleLabelCopy {
                source_module_reference: module.blueprint_module_reference,
                target_module_reference: None,
            })
            .collect(),
        source_assessments: source
            .modules
            .iter()
            .flat_map(|module| &module.assessments)
            .map(|assessment| BlueprintForkApplyAssessmentCopy {
                source_assessment_reference: assessment.blueprint_assessment_reference,
                target_assessment_reference: None,
            })
            .collect(),
        layout: Some(
            source
                .modules
                .iter()
                .map(|module| BlueprintForkApplyModuleLayout {
                    module: BlueprintForkApplyModuleDestination::NewFromSource {
                        source_module_reference: module.blueprint_module_reference,
                    },
                    assessments: module
                        .assessments
                        .iter()
                        .map(
                            |assessment| BlueprintForkApplyAssessmentDestination::NewFromSource {
                                source_assessment_reference: assessment
                                    .blueprint_assessment_reference,
                            },
                        )
                        .collect(),
                })
                .collect(),
        ),
    }
}

fn selected_stored_content(
    source: &StoredBlueprintCourseContent,
    target: &StoredBlueprintCourseContent,
    applied: &question_model::BlueprintCourseContent,
    copied: &BTreeMap<BlueprintAssessmentId, BlueprintAssessmentId>,
) -> Result<StoredBlueprintCourseContent, StoreError> {
    let mut modules = Vec::new();
    for module in applied.modules() {
        let mut assessments = Vec::new();
        for assessment in module.assessments() {
            let reference = assessment.blueprint_assessment_reference();
            let (tree, original) = copied
                .get(&reference)
                .map_or((target, reference), |reference| (source, *reference));
            let mut stored = tree
                .modules
                .iter()
                .flat_map(|module| &module.assessments)
                .find(|assessment| assessment.blueprint_assessment_reference == original)
                .ok_or_else(invalid)?
                .clone();
            stored.blueprint_assessment_reference = reference;
            assessments.push(stored);
        }
        modules.push(StoredBlueprintModule {
            blueprint_module_reference: module.blueprint_module_reference(),
            label: module.label().to_owned(),
            assessments,
        });
    }
    let result = StoredBlueprintCourseContent { modules };
    if result.to_domain()? != *applied {
        return Err(invalid());
    }
    Ok(result)
}

async fn read_accepted_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    proposal_id: uuid::Uuid,
) -> Result<Option<AcceptedBlueprintChangeProposal>, StoreError> {
    let row = sqlx::query("SELECT * FROM ple_api.read_accepted_blueprint_change_proposal($1)")
        .bind(proposal_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let Json(decision): Json<BlueprintChangeProposalAcceptedDecision> =
        row.try_get("decision").map_err(map_sqlx_error)?;
    Ok(Some(AcceptedBlueprintChangeProposal {
        proposal_id: row.try_get("proposal_id").map_err(map_sqlx_error)?,
        actor: parse_account_id(row.try_get("actor_account_id").map_err(map_sqlx_error)?)?,
        accepted_at: Timestamp::from_unix_millis(
            row.try_get("accepted_at_ms").map_err(map_sqlx_error)?,
        ),
        target: reference(&row)?,
        target_blueprint_edit_number: BlueprintEditNumber::from_edit_number(
            row.try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        decision,
        resulting_json: canonical(&row)?,
    }))
}

async fn read_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    proposal_id: uuid::Uuid,
) -> Result<Option<StoredBlueprintChangeProposal>, StoreError> {
    Ok(read_sources_in_transaction(transaction, proposal_id)
        .await?
        .map(|(proposal, _, _)| proposal))
}

async fn read_sources_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    proposal_id: uuid::Uuid,
) -> Result<
    Option<(
        StoredBlueprintChangeProposal,
        [StoredBlueprintRevision; 2],
        bool,
    )>,
    StoreError,
> {
    let rows = sqlx::query("SELECT * FROM ple_api.read_blueprint_change_proposal($1)")
        .bind(proposal_id)
        .fetch_all(&mut **transaction)
        .await
        .map_err(map_sqlx_error)?;
    if rows.is_empty() {
        return Ok(None);
    }
    if rows.len() != 2
        || rows[0]
            .try_get::<i32, _>("source_position")
            .map_err(map_sqlx_error)?
            != 0
        || rows[1]
            .try_get::<i32, _>("source_position")
            .map_err(map_sqlx_error)?
            != 1
    {
        return Err(invalid());
    }
    let source = &rows[0];
    let target = &rows[1];
    let source_content = content_row(source)?;
    let target_content = content_row(target)?;
    let proposal = StoredBlueprintChangeProposal {
        proposal_id: source.try_get("proposal_id").map_err(map_sqlx_error)?,
        proposer: source
            .try_get("proposer_account_id")
            .map_err(map_sqlx_error)
            .and_then(parse_account_id)?,
        created_at: Timestamp::from_unix_millis(
            source.try_get("created_at_ms").map_err(map_sqlx_error)?,
        ),
        source: reference(source)?,
        source_blueprint_edit_number: BlueprintEditNumber::from_edit_number(
            source
                .try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        target: reference(target)?,
        target_blueprint_edit_number: BlueprintEditNumber::from_edit_number(
            target
                .try_get("blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        proposed_json: canonical_content(source, &source_content)?,
        target_comparison_json: canonical_content(target, &target_content)?,
        target_is_stale: source.try_get("target_is_stale").map_err(map_sqlx_error)?,
    };
    let revisions = [
        StoredBlueprintRevision {
            reference: proposal.source.clone(),
            content: source_content,
        },
        StoredBlueprintRevision {
            reference: proposal.target.clone(),
            content: target_content,
        },
    ];
    Ok(Some((
        proposal,
        revisions,
        source.try_get("can_accept").map_err(map_sqlx_error)?,
    )))
}

fn canonical(row: &sqlx::postgres::PgRow) -> Result<CanonicalBlueprintCourse, StoreError> {
    canonical_content(row, &content_row(row)?)
}

fn content_row(row: &sqlx::postgres::PgRow) -> Result<StoredBlueprintCourseContent, StoreError> {
    let Json(content): Json<Value> = row.try_get("content").map_err(map_sqlx_error)?;
    let checksum: Vec<u8> = row.try_get("content_checksum").map_err(map_sqlx_error)?;
    decode_revision_content(content, &checksum)
}

fn canonical_content(
    row: &sqlx::postgres::PgRow,
    content: &StoredBlueprintCourseContent,
) -> Result<CanonicalBlueprintCourse, StoreError> {
    // Existing immutable metadata events are the authority, never today's names.
    Ok(CanonicalBlueprintCourse::export(
        row.try_get::<String, _>("short_name")
            .map_err(map_sqlx_error)?,
        row.try_get::<String, _>("long_name")
            .map_err(map_sqlx_error)?,
        decode_classification(row)?,
        &content.to_domain()?,
    ))
}

fn reference(row: &sqlx::postgres::PgRow) -> Result<BlueprintRevisionReference, StoreError> {
    let reference: String = row.try_get("public_reference").map_err(map_sqlx_error)?;
    let revision: i64 = row.try_get("revision_number").map_err(map_sqlx_error)?;
    Ok(BlueprintRevisionReference {
        reference: reference
            .parse::<BlueprintCourseId>()
            .map_err(|_| invalid())?,
        revision: BlueprintRevision::new(u64::try_from(revision).map_err(|_| invalid())?)
            .ok_or_else(invalid)?,
    })
}

fn proposal_summary(
    row: &sqlx::postgres::PgRow,
) -> Result<BlueprintChangeProposalSummary, StoreError> {
    let target = summary_reference(row, "target_public_reference", "target_revision_number")?;
    let accepted_at: Option<i64> = row.try_get("accepted_at_ms").map_err(map_sqlx_error)?;
    let accepted_revision: Option<i64> = row
        .try_get("accepted_target_revision_number")
        .map_err(map_sqlx_error)?;
    let accepted_etag: Option<i64> = row
        .try_get("accepted_target_blueprint_edit_number")
        .map_err(map_sqlx_error)?;
    let accepted = match (accepted_at, accepted_revision, accepted_etag) {
        (None, None, None) => None,
        (Some(at), Some(revision), Some(etag)) => Some(BlueprintChangeProposalAcceptedSummary {
            accepted_at: Timestamp::from_unix_millis(at),
            target: BlueprintRevisionReference {
                reference: target.reference.clone(),
                revision: BlueprintRevision::new(u64::try_from(revision).map_err(|_| invalid())?)
                    .ok_or_else(invalid)?,
            },
            target_blueprint_edit_number: BlueprintEditNumber::from_edit_number(etag),
        }),
        _ => return Err(invalid()),
    };
    Ok(BlueprintChangeProposalSummary {
        proposal_id: row.try_get("proposal_id").map_err(map_sqlx_error)?,
        created_at: Timestamp::from_unix_millis(
            row.try_get("created_at_ms").map_err(map_sqlx_error)?,
        ),
        source: summary_reference(row, "source_public_reference", "source_revision_number")?,
        source_blueprint_edit_number: BlueprintEditNumber::from_edit_number(
            row.try_get("source_blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        source_short_name: row.try_get("source_short_name").map_err(map_sqlx_error)?,
        source_long_name: row.try_get("source_long_name").map_err(map_sqlx_error)?,
        target,
        target_blueprint_edit_number: BlueprintEditNumber::from_edit_number(
            row.try_get("target_blueprint_edit_number")
                .map_err(map_sqlx_error)?,
        ),
        target_short_name: row.try_get("target_short_name").map_err(map_sqlx_error)?,
        target_long_name: row.try_get("target_long_name").map_err(map_sqlx_error)?,
        target_is_stale: row.try_get("target_is_stale").map_err(map_sqlx_error)?,
        accepted,
    })
}

fn summary_reference(
    row: &sqlx::postgres::PgRow,
    name: &str,
    number: &str,
) -> Result<BlueprintRevisionReference, StoreError> {
    let value: String = row.try_get(name).map_err(map_sqlx_error)?;
    let revision: i64 = row.try_get(number).map_err(map_sqlx_error)?;
    Ok(BlueprintRevisionReference {
        reference: value.parse().map_err(|_| invalid())?,
        revision: BlueprintRevision::new(u64::try_from(revision).map_err(|_| invalid())?)
            .ok_or_else(invalid)?,
    })
}

fn revision_number(revision: BlueprintRevision) -> Result<i64, StoreError> {
    i64::try_from(revision.value()).map_err(|_| invalid())
}

fn invalid() -> StoreError {
    StoreError::InvalidRecord("Blueprint Change Proposal record is invalid".into())
}
