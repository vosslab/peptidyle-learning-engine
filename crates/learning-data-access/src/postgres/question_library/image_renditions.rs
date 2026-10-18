//! Immutable Native source image bindings for the authorized Library result set.

use std::collections::BTreeMap;

use question_model::{
    PublishedQuestionRevisionTuple, QuestionBackend, QuestionHtmlImageBinding,
    QuestionImageAssetId, QuestionImageAssetTuple, QuestionRevisionNumber, SourceObjectChecksum,
};
use sqlx::{Postgres, Row, Transaction};

use super::{invalid, map_sqlx_error};
use crate::{PublishedQuestionLibraryEntry, StoreError};

pub(super) async fn load_image_bindings<'a>(
    transaction: &mut Transaction<'_, Postgres>,
    entries: impl Iterator<Item = &'a mut PublishedQuestionLibraryEntry>,
) -> Result<(), StoreError> {
    let entries = entries
        .filter(|entry| entry.backend == QuestionBackend::Ple)
        .collect::<Vec<_>>();
    let mut by_revision = entries
        .iter()
        .map(|entry| {
            (
                entry.published_question_revision_tuple.clone(),
                Vec::<QuestionHtmlImageBinding>::new(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if by_revision.is_empty() {
        return Ok(());
    }
    let (question_ids, revision_numbers): (Vec<_>, Vec<_>) = by_revision
        .keys()
        .map(|publication| {
            Ok((
                publication.published_question_id.as_str().to_owned(),
                i32::try_from(publication.revision_number.get())
                    .map_err(|_| invalid("Question Revision Number"))?,
            ))
        })
        .collect::<Result<Vec<_>, StoreError>>()?
        .into_iter()
        .unzip();
    // ASVS 1.2.4, 8.3.1: bind paired exact tuples from the preceding authorized
    // result set, in its existing session transaction, in one bounded query.
    let rows = sqlx::query(
        "SELECT publication.published_question_id, publication.revision_number, \
                binding.source_relative_name, binding.question_image_asset_id, \
                binding.question_image_checksum \
         FROM unnest($1::text[], $2::integer[]) \
              AS publication(published_question_id, revision_number) \
         CROSS JOIN LATERAL ple_api.load_question_library_image_bindings(\
             publication.published_question_id, publication.revision_number) AS binding \
         ORDER BY publication.published_question_id, publication.revision_number, \
                  binding.question_image_asset_id",
    )
    .bind(&question_ids)
    .bind(&revision_numbers)
    .fetch_all(&mut **transaction)
    .await
    .map_err(map_sqlx_error)?;
    for row in &rows {
        let revision_number = row
            .try_get::<i32, _>("revision_number")
            .map_err(map_sqlx_error)?;
        let publication = PublishedQuestionRevisionTuple {
            published_question_id: row
                .try_get::<String, _>("published_question_id")
                .map_err(map_sqlx_error)?
                .parse()
                .map_err(|_| invalid("Published Question ID"))?,
            revision_number: u32::try_from(revision_number)
                .ok()
                .and_then(|value| QuestionRevisionNumber::new(value).ok())
                .ok_or_else(|| invalid("Question Revision Number"))?,
        };
        by_revision
            .get_mut(&publication)
            .ok_or_else(|| invalid("Question Library image Revision"))?
            .push(QuestionHtmlImageBinding {
                source_relative_name: row
                    .try_get("source_relative_name")
                    .map_err(map_sqlx_error)?,
                question_image_asset_tuple: QuestionImageAssetTuple {
                    question_image_asset_id: QuestionImageAssetId::from_uuid(
                        row.try_get("question_image_asset_id")
                            .map_err(map_sqlx_error)?,
                    ),
                    checksum: SourceObjectChecksum::parse(
                        row.try_get::<String, _>("question_image_checksum")
                            .map_err(map_sqlx_error)?,
                    )
                    .map_err(|_| invalid("Question Image Source Checksum"))?
                    .to_string(),
                },
            });
    }
    for entry in entries {
        entry.question_image_bindings = by_revision
            .get(&entry.published_question_revision_tuple)
            .cloned()
            .ok_or_else(|| invalid("Question Library image Revision"))?;
    }
    Ok(())
}
