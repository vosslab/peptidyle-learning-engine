use anyhow::{Context, Result};
use question_model::{QuestionBackend, QuestionRevisionTuple, QuestionType};

use super::{Manifest, SourceRevisions};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReceiptBlueprintRevisionTuple {
    blueprint_course_id: String,
    revision_number: u64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Receipt {
    pub(super) blueprint_revision_tuple: ReceiptBlueprintRevisionTuple,
    pub(super) source_repository: String,
    pub(super) source_revision: String,
    pub(super) topics: Vec<ReceiptTopic>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReceiptTopic {
    pub(super) source_key: String,
    pub(super) title: String,
    pub(super) questions: Vec<ReceiptQuestion>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReceiptQuestion {
    pub(super) source_id: String,
    pub(super) title: String,
    pub(super) source_path: String,
    pub(super) source_checksum: String,
    pub(super) backend: QuestionBackend,
    pub(super) question_type: QuestionType,
    pub(super) webwork_pg_path: String,
    pub(super) question_revision_tuple: QuestionRevisionTuple,
}

impl Receipt {
    pub(crate) fn blueprint_course_id(&self) -> &str {
        &self.blueprint_revision_tuple.blueprint_course_id
    }

    pub(crate) fn installation_summary(&self) -> String {
        let question_count = self
            .topics
            .iter()
            .map(|topic| topic.questions.len())
            .sum::<usize>();
        format!(
            "Genetics Blueprint {} revision {}: {} topics, {} canonical Questions",
            self.blueprint_revision_tuple.blueprint_course_id,
            self.blueprint_revision_tuple.revision_number,
            self.topics.len(),
            question_count
        )
    }

    pub(super) fn new(
        blueprint_course_id: String,
        revision_number: u64,
        manifest: &Manifest,
        revisions: &SourceRevisions,
    ) -> Result<Self> {
        let sources = manifest
            .parameterized_sources
            .iter()
            .map(|source| (source.source_id.as_str(), source))
            .collect::<std::collections::BTreeMap<_, _>>();
        let topics = manifest
            .topics
            .iter()
            .map(|topic| -> Result<_> {
                let questions = topic
                    .source_ids
                    .iter()
                    .map(|source_id| -> Result<_> {
                        let source = sources.get(source_id.as_str()).with_context(|| {
                            format!("curriculum receipt is missing source {source_id}")
                        })?;
                        let question_revision_tuple =
                            revisions.get(source_id).cloned().with_context(|| {
                                format!("curriculum receipt is missing revision for {source_id}")
                            })?;
                        Ok(ReceiptQuestion {
                            source_id: source_id.clone(),
                            title: source.question_title.clone(),
                            source_path: source.pg_source.display().to_string(),
                            source_checksum: source.pg_sha256.clone(),
                            backend: QuestionBackend::Webwork,
                            question_type: super::super::parameterized_publication::question_type(
                                source.question_type,
                            ),
                            webwork_pg_path: source.webwork_pg_path.clone(),
                            question_revision_tuple,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(ReceiptTopic {
                    source_key: topic.slug.clone(),
                    title: topic.title.clone(),
                    questions,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            blueprint_revision_tuple: ReceiptBlueprintRevisionTuple {
                blueprint_course_id,
                revision_number,
            },
            source_repository: manifest.course.source_repository.clone(),
            source_revision: manifest.course.source_revision.clone(),
            topics,
        })
    }
}

impl std::fmt::Display for Receipt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string(self)
            .map_err(|_| std::fmt::Error)?
            .fmt(formatter)
    }
}
