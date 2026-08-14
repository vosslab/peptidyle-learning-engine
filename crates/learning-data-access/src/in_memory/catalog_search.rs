//! Portable ranked-catalog admission and scoring helpers.

use super::*;

/// Deterministic conformance implementation of the PostgreSQL ranking policy.
/// Scores are fixed-point millionths so cursor ordering is architecture-neutral.
pub(super) fn catalog_search_score(
    record: &PublishedProblemRecord,
    query: &CatalogSearchQuery,
    question_ids: &crate::QuestionIdCodec,
    statistics_available: bool,
) -> Option<(i64, i64)> {
    if matches!(query.statistics, CatalogStatisticsAvailability::Available) && !statistics_available
    {
        return None;
    }
    if matches!(query.statistics, CatalogStatisticsAvailability::Unavailable)
        && statistics_available
    {
        return None;
    }
    if let Some(text) = &query.text {
        if let Some(question_id) = query.exact_question_id() {
            if question_ids.validates(&question_id) {
                if record.question_id != question_id {
                    return None;
                }
            } else if catalog_text_scores(record, text).is_none() {
                return None;
            }
        } else if catalog_text_scores(record, text).is_none() {
            return None;
        }
    }
    if !query.taxonomy.iter().all(|wanted| {
        record
            .question
            .metadata
            .taxonomy
            .iter()
            .any(|term| term.scheme == wanted.scheme && term.code == wanted.code)
    }) {
        return None;
    }
    if !query
        .capabilities
        .iter()
        .all(|capability| record.capabilities.supports(*capability))
    {
        return None;
    }
    if !(query.licenses.is_empty()
        || query
            .licenses
            .iter()
            .any(|license| license.matches(&record.question.metadata.license)))
    {
        return None;
    }
    if query
        .exact_question_id()
        .is_some_and(|question_id| question_ids.validates(&question_id))
    {
        Some((i64::MAX, i64::MAX))
    } else {
        query
            .text
            .as_deref()
            .map_or(Some((0, 0)), |text| catalog_text_scores(record, text))
    }
}

fn catalog_text_scores(record: &PublishedProblemRecord, query: &str) -> Option<(i64, i64)> {
    let searchable = catalog_searchable_text(record);
    let words = searchable.split_whitespace().collect::<Vec<_>>();
    let query_words = query.split_whitespace().collect::<Vec<_>>();
    let lexical = query_words
        .iter()
        .filter(|word| searchable.contains(**word))
        .count() as i64;
    let similarity = query_words.iter().fold(0_i64, |best_total, query_word| {
        let best = words
            .iter()
            .map(|word| trigram_similarity(query_word, word))
            .max()
            .unwrap_or(0);
        best_total.saturating_add(best)
    }) / i64::try_from(query_words.len().max(1)).expect("word count fits i64");
    // A complete lexical phrase is portable: every normalized query word must
    // occur, otherwise the typo path needs intentional trigram similarity.
    (lexical == i64::try_from(query_words.len()).expect("word count fits i64")
        || similarity >= 300_000)
        .then_some((lexical * 1_000_000, similarity))
}

fn catalog_searchable_text(record: &PublishedProblemRecord) -> String {
    std::iter::once(record.question.metadata.title.as_str())
        .chain(record.question.metadata.language.split_whitespace())
        .chain(record.question.metadata.tags.iter().map(|tag| tag.as_str()))
        .chain(record.question.metadata.taxonomy.iter().flat_map(|term| {
            [
                term.scheme.as_str(),
                term.code.as_str(),
                term.label.as_str(),
            ]
        }))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn trigram_similarity(left: &str, right: &str) -> i64 {
    let grams = |value: &str| {
        let padded = format!("  {value} ");
        padded
            .as_bytes()
            .windows(3)
            .map(|part| part.to_vec())
            .collect::<BTreeSet<_>>()
    };
    let left = grams(left);
    let right = grams(right);
    let denominator = left.len() + right.len();
    if denominator == 0 {
        return 0;
    }
    i64::try_from(2 * left.intersection(&right).count() * 1_000_000 / denominator)
        .expect("similarity fits i64")
}
