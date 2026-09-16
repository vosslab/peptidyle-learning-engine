//! Trusted creation of the reusable Pools referenced by curriculum Blueprints.

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use learning_data_access::{
    CreateQuestionPoolError, CreateQuestionPoolInput, PublishedQuestionPoolRevision,
    QuestionPoolCreationStore, SessionTokenHash,
};
use question_model::{
    QuestionPoolRevisionNumber, QuestionPoolRevisionReference, QuestionRevisionReference,
};
use server_core::question_publication::QuestionIdIssuer;
use uuid::Uuid;

use super::super::Manifest;
use super::source_key;

const QUESTION_POOL_IDENTITY_ATTEMPTS: usize = 8;

pub(super) type PublishedPools = BTreeMap<(String, String), PublishedQuestionPoolRevision>;

pub(super) async fn publish_static_pools(
    session: SessionTokenHash,
    manifest: &Manifest,
    published: &BTreeMap<String, QuestionRevisionReference>,
    replacements: &BTreeMap<(String, String), QuestionRevisionReference>,
    store: &dyn QuestionPoolCreationStore,
    issuer: &dyn QuestionIdIssuer,
) -> Result<PublishedPools> {
    let mut pools = PublishedPools::new();
    for topic in &manifest.topics {
        for bank in &topic.banks {
            let bank_key = (topic.slug.clone(), bank.slug.clone());
            if replacements.contains_key(&bank_key) {
                continue;
            }
            let members = bank
                .rows
                .iter()
                .map(|row| {
                    published
                        .get(&source_key(topic, bank, row))
                        .cloned()
                        .with_context(|| {
                            format!("published curriculum source is missing for {}", row.row_id)
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            let pool = create_published_pool(session.clone(), members, store, issuer).await?;
            if pools.insert(bank_key.clone(), pool).is_some() {
                bail!(
                    "curriculum Pool identity duplicated: {}/{}",
                    bank_key.0,
                    bank_key.1
                );
            }
        }
    }
    Ok(pools)
}

async fn create_published_pool(
    session: SessionTokenHash,
    members: Vec<QuestionRevisionReference>,
    store: &dyn QuestionPoolCreationStore,
    issuer: &dyn QuestionIdIssuer,
) -> Result<PublishedQuestionPoolRevision> {
    for _ in 0..QUESTION_POOL_IDENTITY_ATTEMPTS {
        let public_question_pool_id = issuer
            .issue_question_id()
            .context("issuing a curriculum Question Pool public ID")?;
        let input = CreateQuestionPoolInput {
            question_pool_id: Uuid::now_v7(),
            public_question_pool_id,
            members: members.clone(),
            interchangeability_attested: true,
        };
        match store.create_question_pool(session.clone(), input).await {
            Ok(created) => {
                let revision_number = QuestionPoolRevisionNumber::new(created.revision_number)
                    .map_err(|_| anyhow::anyhow!("created curriculum Pool Revision is invalid"))?;
                return Ok(PublishedQuestionPoolRevision {
                    question_pool_revision: QuestionPoolRevisionReference {
                        question_pool_id: created.public_question_pool_id,
                        revision_number,
                    },
                    members,
                });
            }
            Err(CreateQuestionPoolError::IdentityCollision) => continue,
            Err(CreateQuestionPoolError::Store(error)) => {
                return Err(error).context("creating a reusable curriculum Question Pool");
            }
        }
    }
    bail!("curriculum Question Pool public ID collisions exhausted the retry bound")
}
