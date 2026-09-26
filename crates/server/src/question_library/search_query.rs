//! Bounded text-query parsing and matching for Question Library search.

use learning_data_access::{QuestionLibraryTextField, QuestionLibraryTextTerm};
use question_model::PublishedQuestionId;

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

    pub(super) fn into_store_terms(
        self,
    ) -> (Option<PublishedQuestionId>, Vec<QuestionLibraryTextTerm>) {
        (
            self.exact_question_id,
            self.terms
                .into_iter()
                .map(|term| QuestionLibraryTextTerm {
                    field: match term.field {
                        None => QuestionLibraryTextField::Any,
                        Some(SearchField::Discipline) => QuestionLibraryTextField::Discipline,
                        Some(SearchField::Subtopic) => QuestionLibraryTextField::Subtopic,
                        Some(SearchField::Subject) => QuestionLibraryTextField::Subject,
                        Some(SearchField::Topic) => QuestionLibraryTextField::Topic,
                        Some(SearchField::Tags) => QuestionLibraryTextField::Tags,
                        Some(SearchField::QuestionType) => QuestionLibraryTextField::QuestionType,
                        Some(SearchField::Author) => QuestionLibraryTextField::Author,
                    },
                    value: term.value,
                    excluded: term.excluded,
                })
                .collect(),
        )
    }
}
