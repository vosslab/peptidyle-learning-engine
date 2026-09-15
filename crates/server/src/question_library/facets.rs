//! Bounded full-snapshot Question Library facet aggregation.

use std::collections::BTreeMap;

use question_model::{
    Capability, MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS, MAX_QUESTION_SEARCH_TAG_FACETS,
    QuestionBackend, QuestionSearchAuthorFacet, QuestionSearchBackendFacet,
    QuestionSearchCapabilityFacet, QuestionSearchCourseUseFacet, QuestionSearchFacets,
    QuestionSearchQuestionLicenseFacet, QuestionSearchSubjectFacet, QuestionSearchTagFacet,
    QuestionSearchTopicFacet, QuestionTypeFacet, normalized_question_search_group_value,
};

use super::ResolvedQuestionLibraryEntry;

/// ASVS 8.2.2 and 8.3.1: aggregates only the session-authorized, query-matched
/// entries supplied by the route; it never performs or widens a Store lookup.
pub(super) fn facets(entries: &[&ResolvedQuestionLibraryEntry]) -> QuestionSearchFacets {
    let mut authors = BTreeMap::<String, (String, u64)>::new();
    let mut backends = BTreeMap::<QuestionBackend, u64>::new();
    let mut tags = BTreeMap::<String, (String, u64)>::new();
    let mut subjects = BTreeMap::<String, (String, u64)>::new();
    let mut topics = BTreeMap::<String, (String, u64)>::new();
    let mut question_types = BTreeMap::<question_model::QuestionType, u64>::new();
    let mut capabilities = BTreeMap::<Capability, u64>::new();
    let mut licenses = BTreeMap::<question_model::QuestionLicense, u64>::new();
    for entry in entries {
        let summary = &entry.summary;
        count_text_facets(
            &mut authors,
            summary
                .authorship
                .authors
                .iter()
                .map(|author| author.display_name.as_str()),
        );
        *backends.entry(summary.backend).or_default() += 1;
        count_text_facets(
            &mut tags,
            summary.metadata.tags.iter().map(|tag| tag.as_str()),
        );
        count_text_facets(&mut subjects, entry.subject.iter().map(String::as_str));
        count_text_facets(&mut topics, entry.topic.iter().map(String::as_str));
        *question_types.entry(summary.question_type).or_default() += 1;
        for capability in summary.capabilities.declared() {
            *capabilities.entry(capability).or_default() += 1;
        }
        if let Some(license) = &summary.metadata.question_license {
            *licenses.entry(license.clone()).or_default() += 1;
        }
    }
    QuestionSearchFacets {
        author_names_truncated: authors.len() > MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS,
        author_names: bounded_text_facets(authors, MAX_QUESTION_SEARCH_AUTHOR_NAME_FACETS)
            .into_iter()
            .map(|(author_name, count)| QuestionSearchAuthorFacet { author_name, count })
            .collect(),
        backends: backends
            .into_iter()
            .map(|(backend, count)| QuestionSearchBackendFacet { backend, count })
            .collect(),
        tags_truncated: tags.len() > MAX_QUESTION_SEARCH_TAG_FACETS,
        tags: bounded_text_facets(tags, MAX_QUESTION_SEARCH_TAG_FACETS)
            .into_iter()
            .map(|(tag, count)| QuestionSearchTagFacet { tag, count })
            .collect(),
        subjects_truncated: subjects.len() > MAX_QUESTION_SEARCH_TAG_FACETS,
        subjects: bounded_text_facets(subjects, MAX_QUESTION_SEARCH_TAG_FACETS)
            .into_iter()
            .map(|(subject, count)| QuestionSearchSubjectFacet { subject, count })
            .collect(),
        topics_truncated: topics.len() > MAX_QUESTION_SEARCH_TAG_FACETS,
        topics: bounded_text_facets(topics, MAX_QUESTION_SEARCH_TAG_FACETS)
            .into_iter()
            .map(|(topic, count)| QuestionSearchTopicFacet { topic, count })
            .collect(),
        question_types: question_types
            .into_iter()
            .map(|(question_type, count)| QuestionTypeFacet {
                question_type,
                count,
            })
            .collect(),
        capabilities: capabilities
            .into_iter()
            .map(|(capability, count)| QuestionSearchCapabilityFacet { capability, count })
            .collect(),
        question_licenses: licenses
            .into_iter()
            .map(
                |(question_license, count)| QuestionSearchQuestionLicenseFacet {
                    question_license,
                    count,
                },
            )
            .collect(),
        used_in_my_courses: QuestionSearchCourseUseFacet {
            used: entries
                .iter()
                .filter(|entry| entry.used_in_current_account_courses)
                .count() as u64,
        },
    }
}

fn bounded_text_facets(
    facets: BTreeMap<String, (String, u64)>,
    maximum: usize,
) -> Vec<(String, u64)> {
    let mut facets = facets.into_values().collect::<Vec<_>>();
    facets.sort_by(|(left_value, left_count), (right_value, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_value.cmp(right_value))
    });
    facets.truncate(maximum);
    facets
}

fn count_text_facets<'a>(
    counts: &mut BTreeMap<String, (String, u64)>,
    values: impl Iterator<Item = &'a str>,
) {
    let mut values_for_question = BTreeMap::<String, String>::new();
    for value in values {
        let display = value.split_whitespace().collect::<Vec<_>>().join(" ");
        let key = normalized_question_search_group_value(&display);
        if key.is_empty() {
            continue;
        }
        values_for_question
            .entry(key)
            .and_modify(|current_display| {
                if display < *current_display {
                    *current_display = display.clone();
                }
            })
            .or_insert(display);
    }
    for (key, display) in values_for_question {
        counts
            .entry(key)
            .and_modify(|(current_display, count)| {
                if display < *current_display {
                    *current_display = display.clone();
                }
                *count += 1;
            })
            .or_insert((display, 1));
    }
}
