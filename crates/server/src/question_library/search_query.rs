//! Bounded text-query parsing and matching for Question Library search.

use question_model::{PublishedQuestionId, QuestionType};

use super::ResolvedQuestionLibraryEntry;

use crate::library_search_terms::{SearchField, SearchTerm, parse_terms};

pub(super) struct QuestionTextQuery {
    exact_question_id: Option<PublishedQuestionId>,
    terms: Vec<SearchTerm>,
}

impl QuestionTextQuery {
    /// Parses the normalized, length-bounded text once at the trusted service layer.
    ///
    /// ASVS 2.2.1 and 2.2.2: the server applies the documented allow-listed field
    /// grammar instead of relying on browser parsing or passing input to an interpreter.
    pub(super) fn parse(text: Option<&str>) -> Self {
        let exact_question_id = text.and_then(|value| value.parse::<PublishedQuestionId>().ok());
        let terms = if exact_question_id.is_some() {
            Vec::new()
        } else {
            text.map(parse_terms).unwrap_or_default()
        };
        Self {
            exact_question_id,
            terms,
        }
    }

    pub(super) fn matches(&self, entry: &ResolvedQuestionLibraryEntry) -> bool {
        if let Some(question_id) = &self.exact_question_id {
            return &entry.summary.question_id == question_id;
        }
        self.terms.iter().all(|term| term.matches(entry))
    }
}

impl SearchTerm {
    fn matches(&self, entry: &ResolvedQuestionLibraryEntry) -> bool {
        if self.value.is_empty() {
            return false;
        }
        let found = match self.field {
            Some(SearchField::Discipline) => {
                optional_value_contains(&entry.discipline, &self.value)
            }
            Some(SearchField::Subtopic) => optional_value_contains(&entry.subtopic, &self.value),
            Some(SearchField::Subject) => optional_value_contains(&entry.subject, &self.value),
            Some(SearchField::Topic) => optional_value_contains(&entry.topic, &self.value),
            Some(SearchField::Tags) => entry
                .summary
                .metadata
                .tags
                .iter()
                .any(|tag| value_contains(tag.as_str(), &self.value)),
            Some(SearchField::QuestionType) => {
                question_type_label(entry.summary.question_type).contains(&self.value)
            }
            Some(SearchField::Author) => entry
                .summary
                .authorship
                .authors
                .iter()
                .any(|author| value_contains(author.display_name.as_str(), &self.value)),
            None => unfielded_value_contains(entry, &self.value),
        };
        found != self.excluded
    }
}

fn unfielded_value_contains(entry: &ResolvedQuestionLibraryEntry, needle: &str) -> bool {
    value_contains(&entry.summary.metadata.question_title, needle)
        || value_contains(&entry.summary.metadata.question_description, needle)
        || entry
            .summary
            .metadata
            .tags
            .iter()
            .any(|tag| value_contains(tag.as_str(), needle))
        || optional_value_contains(&entry.subject, needle)
        || optional_value_contains(&entry.topic, needle)
        || entry
            .summary
            .authorship
            .authors
            .iter()
            .any(|author| value_contains(author.display_name.as_str(), needle))
        || question_type_label(entry.summary.question_type).contains(needle)
}

fn optional_value_contains(value: &Option<String>, needle: &str) -> bool {
    value
        .as_deref()
        .is_some_and(|value| value_contains(value, needle))
}

fn value_contains(value: &str, needle: &str) -> bool {
    value.to_lowercase().contains(needle)
}

fn question_type_label(question_type: QuestionType) -> &'static str {
    match question_type {
        QuestionType::MultipleChoice => "multiple choice",
        QuestionType::MultipleAnswer => "multiple answer",
        QuestionType::FillInBlank => "fill in the blank",
        QuestionType::MultipleFillInBlank => "multiple fill in the blank",
        QuestionType::Numeric => "numeric",
        QuestionType::Matching => "matching",
        QuestionType::Ordering => "ordering",
        QuestionType::Hotspot => "hotspot",
    }
}
