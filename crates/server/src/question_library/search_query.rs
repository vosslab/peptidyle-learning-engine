//! Bounded text-query parsing and matching for Question Library search.

use question_model::{QuestionId, QuestionType};

use super::ResolvedQuestionLibraryEntry;

#[derive(Clone, Copy)]
enum SearchField {
    Discipline,
    Subtopic,
    Subject,
    Topic,
    Tags,
    QuestionType,
    Author,
}

struct SearchTerm {
    field: Option<SearchField>,
    value: String,
    excluded: bool,
}

pub(super) struct QuestionTextQuery {
    exact_question_id: Option<QuestionId>,
    terms: Vec<SearchTerm>,
}

impl QuestionTextQuery {
    /// Parses the normalized, length-bounded text once at the trusted service layer.
    ///
    /// ASVS 2.2.1 and 2.2.2: the server applies the documented allow-listed field
    /// grammar instead of relying on browser parsing or passing input to an interpreter.
    pub(super) fn parse(text: Option<&str>) -> Self {
        let exact_question_id = text.and_then(|value| value.parse::<QuestionId>().ok());
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

fn parse_terms(text: &str) -> Vec<SearchTerm> {
    let mut terms = Vec::new();
    let mut cursor = 0;
    while cursor < text.len() {
        cursor = skip_whitespace(text, cursor);
        if cursor == text.len() {
            break;
        }

        let (excluded, term_start) = exclusion_prefix(text, cursor);
        cursor = term_start;
        let (field, value_start) = field_prefix(text, cursor);
        cursor = value_start;
        let (value, next_cursor) = term_value(text, cursor);
        terms.push(SearchTerm {
            field,
            value: value.to_string(),
            excluded,
        });
        cursor = next_cursor;
    }
    terms
}

fn skip_whitespace(text: &str, mut cursor: usize) -> usize {
    while let Some(character) = text[cursor..].chars().next() {
        if !character.is_whitespace() {
            break;
        }
        cursor += character.len_utf8();
    }
    cursor
}

fn exclusion_prefix(text: &str, cursor: usize) -> (bool, usize) {
    let remainder = &text[cursor..];
    if let Some(after_minus) = remainder.strip_prefix('-')
        && after_minus
            .chars()
            .next()
            .is_some_and(|character| !character.is_whitespace())
    {
        return (true, cursor + 1);
    }
    (false, cursor)
}

fn field_prefix(text: &str, cursor: usize) -> (Option<SearchField>, usize) {
    let remainder = &text[cursor..];
    let prefix_end = remainder
        .find(|character: char| character == ':' || character.is_whitespace() || character == '"')
        .unwrap_or(remainder.len());
    if remainder.as_bytes().get(prefix_end) != Some(&b':') {
        return (None, cursor);
    }
    let field = match &remainder[..prefix_end] {
        "discipline" => SearchField::Discipline,
        "subtopic" => SearchField::Subtopic,
        "subject" => SearchField::Subject,
        "topic" => SearchField::Topic,
        "tags" => SearchField::Tags,
        "type" => SearchField::QuestionType,
        "author" => SearchField::Author,
        _ => return (None, cursor),
    };
    (Some(field), cursor + prefix_end + 1)
}

fn term_value(text: &str, cursor: usize) -> (&str, usize) {
    if cursor == text.len() {
        return ("", cursor);
    }
    if text[cursor..].starts_with('"') {
        let value_start = cursor + 1;
        if let Some(relative_end) = text[value_start..].find('"') {
            let value_end = value_start + relative_end;
            return (&text[value_start..value_end], value_end + 1);
        }
        return (&text[value_start..], text.len());
    }
    let value_end = text[cursor..]
        .find(char::is_whitespace)
        .map_or(text.len(), |relative_end| cursor + relative_end);
    (&text[cursor..value_end], value_end)
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
