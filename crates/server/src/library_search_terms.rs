//! Shared bounded Library text grammar; matching remains object-specific.

#[derive(Clone, Copy)]
pub(crate) enum SearchField {
    Discipline,
    Subtopic,
    Subject,
    Topic,
    Tags,
    QuestionType,
    Author,
}

pub(crate) struct SearchTerm {
    pub(crate) field: Option<SearchField>,
    pub(crate) value: String,
    pub(crate) excluded: bool,
}

pub(crate) fn parse_terms(text: &str) -> Vec<SearchTerm> {
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
