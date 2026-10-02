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

#[cfg(test)]
mod tests {
    use learning_data_access::QuestionLibraryTextField;

    use super::QuestionTextQuery;

    /// Human Guidance field examples, plus Question Type and author.
    const FIELD_QUERY: &str = concat!(
        r#"discipline:biology subject:genetics topic:"chromosomal inheritance" "#,
        r#"tags:review subtopic:"x-linked recessive crosses" "#,
        r#"type:"multiple choice" author:Ada"#,
    );

    fn prefix(field: QuestionLibraryTextField) -> &'static str {
        match field {
            QuestionLibraryTextField::Any => "any",
            QuestionLibraryTextField::Discipline => "discipline",
            QuestionLibraryTextField::Subject => "subject",
            QuestionLibraryTextField::Topic => "topic",
            QuestionLibraryTextField::Subtopic => "subtopic",
            QuestionLibraryTextField::Tags => "tags",
            QuestionLibraryTextField::QuestionType => "type",
            QuestionLibraryTextField::Author => "author",
        }
    }

    #[test]
    fn pubmed_style_fields_round_trip_through_the_store_terms() {
        let (question_id, terms) = QuestionTextQuery::parse(Some(FIELD_QUERY)).into_store_terms();
        assert!(question_id.is_none());
        let rendered = terms
            .iter()
            .map(|term| {
                assert!(!term.excluded);
                assert!(!term.value.is_empty());
                assert_ne!(term.field, QuestionLibraryTextField::Any);
                let value = if term.value.contains(char::is_whitespace) {
                    format!("\"{}\"", term.value)
                } else {
                    term.value.clone()
                };
                format!("{}:{value}", prefix(term.field))
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(rendered, FIELD_QUERY);
    }

    #[test]
    fn exact_canonical_question_id_is_the_library_search_identity() {
        let canonical = "ABCD-XEFG";
        let (question_id, terms) = QuestionTextQuery::parse(Some(canonical)).into_store_terms();
        assert_eq!(
            question_id.expect("exact Question ID").to_string(),
            canonical
        );
        assert!(terms.is_empty());

        let (rejected, _) = QuestionTextQuery::parse(Some("ABCD-AEFG")).into_store_terms();
        assert!(rejected.is_none());
        let (lowercase, _) = QuestionTextQuery::parse(Some("abcd-xefg")).into_store_terms();
        assert!(lowercase.is_none());
    }
}
