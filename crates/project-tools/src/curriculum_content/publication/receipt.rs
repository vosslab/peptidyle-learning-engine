use std::collections::BTreeMap;

use anyhow::{Context, Result};
use question_model::{QuestionBackend, QuestionRevisionReference, QuestionType};

use super::{Manifest, ReplacementRevisions, question_type, replacement_source, source_key};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Receipt {
    pub(super) blueprint_reference: String,
    pub(super) blueprint_revision: u64,
    pub(super) source_repository: String,
    pub(super) source_revision: String,
    pub(super) topics: Vec<ReceiptTopic>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReceiptTopic {
    pub(super) source_key: String,
    pub(super) title: String,
    pub(super) banks: Vec<ReceiptBank>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReceiptBank {
    pub(super) source_key: String,
    pub(super) title: String,
    pub(super) source_path: String,
    pub(super) source_checksum: String,
    pub(super) selection_count: u32,
    pub(super) backend: QuestionBackend,
    pub(super) canonical_source_id: Option<String>,
    pub(super) canonical_question_revision: Option<QuestionRevisionReference>,
    pub(super) rows: Vec<ReceiptRow>,
    pub(super) pool_question_revisions: Vec<QuestionRevisionReference>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReceiptRow {
    pub(super) source_key: String,
    pub(super) row_id: String,
    pub(super) title: String,
    pub(super) source_path: String,
    pub(super) source_checksum: String,
    pub(super) backend: QuestionBackend,
    pub(super) question_type: QuestionType,
    pub(super) webwork_pg_path: String,
    pub(super) pool_position: usize,
    pub(super) question_revision: QuestionRevisionReference,
}

impl Receipt {
    pub(crate) fn installation_summary(&self) -> String {
        let bank_count = self
            .topics
            .iter()
            .map(|topic| topic.banks.len())
            .sum::<usize>();
        let row_count = self
            .topics
            .iter()
            .flat_map(|topic| &topic.banks)
            .map(|bank| bank.rows.len())
            .sum::<usize>();
        format!(
            "Genetics Blueprint {} revision {}: {} topics, {} banks, {} Questions",
            self.blueprint_reference,
            self.blueprint_revision,
            self.topics.len(),
            bank_count,
            row_count
        )
    }

    pub(super) fn new(
        reference: String,
        revision: u64,
        manifest: &Manifest,
        published: &BTreeMap<String, QuestionRevisionReference>,
        replacements: &ReplacementRevisions,
    ) -> Result<Self> {
        let topics = manifest
            .topics
            .iter()
            .map(|topic| -> Result<ReceiptTopic> {
                Ok(ReceiptTopic {
                    source_key: topic.slug.clone(),
                    title: topic.title.clone(),
                    banks: topic
                        .banks
                        .iter()
                        .map(|bank| -> Result<ReceiptBank> {
                            let replacement = replacement_source(manifest, topic, bank);
                            let canonical_question_revision = replacement
                                .map(|_source| {
                                    replacements
                                        .get(&(topic.slug.clone(), bank.slug.clone()))
                                        .cloned()
                                        .with_context(|| {
                                            format!(
                                                "curriculum receipt is missing canonical replacement for {}/{}",
                                                topic.slug, bank.slug
                                            )
                                        })
                                })
                                .transpose()?;
                            let rows = bank
                                .rows
                                .iter()
                                .filter(|_| replacement.is_none())
                                .enumerate()
                                .map(|(pool_position, row)| -> Result<ReceiptRow> {
                                    let source_key = source_key(topic, bank, row);
                                    let question_revision = published
                                        .get(&source_key)
                                        .cloned()
                                        .with_context(|| {
                                            format!(
                                                "curriculum receipt is missing a published revision for {}",
                                                row.row_id
                                            )
                                        })?;
                                    Ok(ReceiptRow {
                                        source_key,
                                        row_id: row.row_id.clone(),
                                        title: row.question_title.clone(),
                                        source_path: row.pg_source.display().to_string(),
                                        source_checksum: row.pg_sha256.clone(),
                                        backend: QuestionBackend::Webwork,
                                        question_type: question_type(row.question_type),
                                        webwork_pg_path: row.webwork_pg_path.clone(),
                                        pool_position,
                                        question_revision,
                                    })
                                })
                                .collect::<Result<Vec<_>>>()?;
                            Ok(ReceiptBank {
                                source_key: format!("{}/{}", topic.slug, bank.slug),
                                title: bank.title.clone(),
                                source_path: bank.source.display().to_string(),
                                source_checksum: bank.source_sha256.clone(),
                                selection_count: bank.selection_count,
                                backend: QuestionBackend::Webwork,
                                canonical_source_id: replacement
                                    .map(|source| source.source_id.clone()),
                                canonical_question_revision,
                                pool_question_revisions: rows
                                    .iter()
                                    .map(|row| row.question_revision.clone())
                                    .collect(),
                                rows,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            blueprint_reference: reference,
            blueprint_revision: revision,
            source_repository: manifest.course.source_repository.clone(),
            source_revision: manifest.course.source_revision.clone(),
            topics,
        })
    }
}

impl std::fmt::Display for Receipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(f)
    }
}
