//! Strict projection of the shipped PGML two-column matching interaction.

use html5ever::tokenizer::{TagKind, Token};

use super::html_projection::{reject_protected_text, tokenize_html, validate_tag};
use super::*;

const MAX_MATCH_ITEMS: usize = 26;

pub(super) struct MatchingPrompt {
    pub(super) field: String,
    pub(super) label: String,
}

pub(super) struct MatchingChoice {
    pub(super) value: String,
    pub(super) label: String,
}

pub(super) struct ParsedMatchingHtml {
    pub(super) prompts: Vec<MatchingPrompt>,
    pub(super) choices: Vec<MatchingChoice>,
    pub(super) prompt_text: String,
    pub(super) prompt_html: String,
}

#[derive(Clone)]
enum Node {
    Text(String),
    Element(Element),
}

#[derive(Clone)]
struct Element {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<Node>,
}

pub(super) fn parse_matching_group(
    html: &str,
    protected_html_values: &BTreeSet<String>,
) -> Result<ParsedMatchingHtml, RendererFailure> {
    if html.len() > DEFAULT_MAX_RESPONSE_BYTES {
        return Err(bad("renderer question body exceeds the supported bound"));
    }
    let root = strict_tree(html, protected_html_values)?;
    let root_children = significant(&root.children);
    let [Node::Element(pgml)] = root_children.as_slice() else {
        return Err(bad("matching render must contain one PGML root"));
    };
    require_attrs(pgml, &[("class", "PGML")])?;

    let mut prompt_text = String::new();
    let mut columns = None;
    for node in &pgml.children {
        match node {
            Node::Text(text) if columns.is_none() => {
                push_matching_text(&mut prompt_text, text)?;
            }
            Node::Text(text) if text.trim().is_empty() => {}
            Node::Element(element) if is_margin(element) && columns.is_none() => {}
            Node::Element(element)
                if element.name == "div"
                    && element.attrs.get("class").map(String::as_str) == Some("two-column")
                    && columns.is_none() =>
            {
                columns = Some(element);
            }
            _ => return Err(bad("matching PGML root has unsupported content")),
        }
    }
    let prompt_text = prompt_text.trim().to_string();
    if prompt_text.is_empty() {
        return Err(bad("renderer prompt is empty"));
    }
    let columns = columns.ok_or_else(|| bad("renderer omitted matching columns"))?;
    require_attrs(columns, &[("class", "two-column")])?;
    let column_children = significant(&columns.children);
    let [Node::Element(left), Node::Element(right)] = column_children.as_slice() else {
        return Err(bad("matching render must have exactly two columns"));
    };
    require_attrs(left, &[])?;
    require_attrs(right, &[("class", "right-col")])?;

    let (prompts, option_values) = parse_left(left)?;
    let choices = parse_right(right)?;
    if prompts.len() != choices.len()
        || option_values.len() != choices.len()
        || choices
            .iter()
            .zip(&option_values)
            .any(|(choice, option)| choice.value != *option)
    {
        return Err(bad("matching prompt and choice sets do not agree"));
    }
    let prompt_html = format!("<p>{}</p>", escape_html(&prompt_text));
    Ok(ParsedMatchingHtml {
        prompts,
        choices,
        prompt_text,
        prompt_html,
    })
}

fn strict_tree(
    html: &str,
    protected_html_values: &BTreeSet<String>,
) -> Result<Element, RendererFailure> {
    let mut stack = vec![Element {
        name: "#root".into(),
        attrs: BTreeMap::new(),
        children: Vec::new(),
    }];
    for token in tokenize_html(html)? {
        match token {
            Token::CharacterTokens(text) => {
                reject_protected_text(text.as_ref(), protected_html_values)?;
                let current = stack
                    .last_mut()
                    .ok_or_else(|| bad("renderer returned unbalanced matching markup"))?;
                if let Some(Node::Text(previous)) = current.children.last_mut() {
                    previous.push_str(text.as_ref());
                } else {
                    current.children.push(Node::Text(text.to_string()));
                }
            }
            Token::TagToken(tag) => {
                validate_tag(&tag)?;
                for attr in &tag.attrs {
                    reject_protected_text(attr.value.as_ref(), protected_html_values)?;
                }
                let name = tag.name.to_string();
                if !matches!(
                    name.as_str(),
                    "div" | "select" | "option" | "strong" | "sub" | "sup" | "span"
                ) || tag.self_closing
                {
                    return Err(bad("matching render contains unsupported markup"));
                }
                match tag.kind {
                    TagKind::StartTag => {
                        if stack.len() >= MAX_HTML_NESTING {
                            return Err(bad("renderer markup exceeds nesting bound"));
                        }
                        let attrs = tag
                            .attrs
                            .into_iter()
                            .map(|attr| (attr.name.local.to_string(), attr.value.to_string()))
                            .collect();
                        stack.push(Element {
                            name,
                            attrs,
                            children: Vec::new(),
                        });
                    }
                    TagKind::EndTag => {
                        let completed = stack
                            .pop()
                            .filter(|element| element.name == name)
                            .ok_or_else(|| bad("renderer returned unbalanced matching markup"))?;
                        stack
                            .last_mut()
                            .ok_or_else(|| bad("renderer returned unbalanced matching markup"))?
                            .children
                            .push(Node::Element(completed));
                    }
                }
            }
            Token::EOFToken => {}
            Token::NullCharacterToken
            | Token::CommentToken(_)
            | Token::DoctypeToken(_)
            | Token::ParseError(_) => return Err(bad("renderer returned malformed HTML")),
        }
    }
    if stack.len() != 1 {
        return Err(bad("renderer returned unbalanced matching markup"));
    }
    stack
        .pop()
        .ok_or_else(|| bad("renderer returned empty matching markup"))
}

fn parse_left(left: &Element) -> Result<(Vec<MatchingPrompt>, Vec<String>), RendererFailure> {
    let nodes = significant(&left.children);
    let mut prompts = Vec::new();
    let mut expected_options = None::<Vec<String>>;
    let mut index = 0;
    let mut fields = BTreeSet::new();
    while index < nodes.len() {
        let Node::Element(wrapper) = nodes[index] else {
            return Err(bad("matching prompt omitted its select wrapper"));
        };
        let (field, options) = parse_select_wrapper(wrapper, prompts.len() + 1)?;
        if !fields.insert(field.clone()) {
            return Err(bad("matching render repeated an upstream field"));
        }
        if expected_options
            .as_ref()
            .is_some_and(|value| value != &options)
        {
            return Err(bad("matching selects expose different choice sets"));
        }
        expected_options.get_or_insert(options);
        index += 1;
        let Node::Element(number) = nodes
            .get(index)
            .copied()
            .ok_or_else(|| bad("matching prompt omitted its visible ordinal"))?
        else {
            return Err(bad("matching prompt omitted its visible ordinal"));
        };
        require_attrs(number, &[])?;
        if number.name != "strong"
            || plain_text(number)?.trim() != format!("{}.", prompts.len() + 1)
        {
            return Err(bad("matching prompt ordinal is inconsistent"));
        }
        index += 1;
        let mut label = String::new();
        while index < nodes.len() {
            match nodes[index] {
                Node::Element(element) if is_margin(element) => {
                    index += 1;
                    break;
                }
                Node::Element(element)
                    if element.attrs.get("class").is_some_and(|classes| {
                        classes
                            .split_ascii_whitespace()
                            .any(|class| class == "d-inline")
                    }) =>
                {
                    break;
                }
                node => {
                    append_plain_node(&mut label, node)?;
                    index += 1;
                }
            }
        }
        let label = label.trim().to_string();
        if label.is_empty() || label.chars().count() > MAX_RADIO_LABEL_CHARS {
            return Err(bad("matching prompt is outside the supported bound"));
        }
        prompts.push(MatchingPrompt { field, label });
        if prompts.len() > MAX_MATCH_ITEMS {
            return Err(bad("matching render exceeds the item bound"));
        }
    }
    let options = expected_options.ok_or_else(|| bad("matching render has no prompts"))?;
    if prompts.len() < 2 {
        return Err(bad("matching render requires at least two prompts"));
    }
    Ok((prompts, options))
}

fn parse_select_wrapper(
    wrapper: &Element,
    ordinal: usize,
) -> Result<(String, Vec<String>), RendererFailure> {
    if wrapper.name != "div" {
        return Err(bad("matching select wrapper is not a div"));
    }
    let field = wrapper
        .attrs
        .get("data-feedback-insert-element")
        .cloned()
        .ok_or_else(|| bad("matching select wrapper lacks its field"))?;
    require_attrs(
        wrapper,
        &[
            ("class", "d-inline text-nowrap"),
            ("data-feedback-insert-element", &field),
            ("data-feedback-insert-method", "append_content"),
        ],
    )?;
    validate_answer_field(&field)?;
    let children = significant(&wrapper.children);
    let [Node::Element(select)] = children.as_slice() else {
        return Err(bad("matching wrapper must contain one select"));
    };
    if select.name != "select" {
        return Err(bad("matching wrapper must contain one select"));
    }
    require_attrs(
        select,
        &[
            ("aria-label", &format!("answer {ordinal} ")),
            ("class", "pg-select"),
            ("id", &field),
            ("name", &field),
            ("size", "1"),
        ],
    )?;
    let options = significant(&select.children);
    if options.len() < 4 || options.len() > MAX_MATCH_ITEMS + 2 {
        return Err(bad(
            "matching select option count is outside the supported bound",
        ));
    }
    let first = option(options[0])?;
    require_option_attrs(first, true, true, "")?;
    if plain_text(first)?.trim() != "?" {
        return Err(bad("matching select lacks the disabled placeholder"));
    }
    let blank = option(options[1])?;
    require_option_attrs(blank, false, true, "")?;
    if !plain_text(blank)?.trim().is_empty() {
        return Err(bad("matching select blank option is malformed"));
    }
    let mut values = Vec::new();
    for (offset, node) in options[2..].iter().enumerate() {
        let option = option(node)?;
        let expected = char::from(
            b'A' + u8::try_from(offset)
                .map_err(|_| bad("matching select option count is outside the supported bound"))?,
        )
        .to_string();
        require_option_attrs(option, false, false, &expected)?;
        if plain_text(option)?.trim() != expected {
            return Err(bad("matching select value and label disagree"));
        }
        values.push(expected);
    }
    Ok((field, values))
}

fn parse_right(right: &Element) -> Result<Vec<MatchingChoice>, RendererFailure> {
    let nodes = significant(&right.children);
    let mut choices = Vec::new();
    let mut index = 0;
    while index < nodes.len() {
        let Node::Text(letter) = nodes[index] else {
            return Err(bad("matching choice omitted its visible letter"));
        };
        let expected = char::from(
            b'A' + u8::try_from(choices.len())
                .map_err(|_| bad("matching choice count is outside the supported bound"))?,
        )
        .to_string();
        if letter.trim() != format!("{expected}.") {
            return Err(bad("matching choice letter is inconsistent"));
        }
        index += 1;
        let Node::Element(label) = nodes
            .get(index)
            .copied()
            .ok_or_else(|| bad("matching choice omitted its label"))?
        else {
            return Err(bad("matching choice omitted its label"));
        };
        if label.name != "span" || !valid_choice_style(label) {
            return Err(bad("matching choice label markup is unsupported"));
        }
        let text = plain_text(label)?.trim().to_string();
        if text.is_empty() || text.chars().count() > MAX_RADIO_LABEL_CHARS {
            return Err(bad("matching choice is outside the supported bound"));
        }
        choices.push(MatchingChoice {
            value: expected,
            label: text,
        });
        index += 1;
        if index < nodes.len() {
            let Node::Element(separator) = nodes[index] else {
                return Err(bad("matching choices require PG separators"));
            };
            if !is_margin(separator) {
                return Err(bad("matching choices require PG separators"));
            }
            index += 1;
        }
        if choices.len() > MAX_MATCH_ITEMS {
            return Err(bad("matching choice count is outside the supported bound"));
        }
    }
    if choices.len() < 2 {
        return Err(bad("matching render requires at least two choices"));
    }
    Ok(choices)
}

fn option(node: &Node) -> Result<&Element, RendererFailure> {
    let Node::Element(element) = node else {
        return Err(bad("matching select contains non-option content"));
    };
    if element.name != "option" {
        return Err(bad("matching select contains non-option content"));
    }
    Ok(element)
}

fn require_option_attrs(
    option: &Element,
    disabled: bool,
    selected: bool,
    value: &str,
) -> Result<(), RendererFailure> {
    let mut expected = vec![("class", "tex2jax_ignore"), ("value", value)];
    if disabled {
        expected.push(("disabled", ""));
    }
    if selected {
        expected.push(("selected", ""));
    }
    require_attrs(option, &expected)
}

fn validate_answer_field(field: &str) -> Result<(), RendererFailure> {
    if field.len() > MAX_RADIO_FIELD_BYTES
        || !field.starts_with("AnSwEr")
        || field[6..].is_empty()
        || !field[6..].bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(bad("matching field is outside the upstream contract"));
    }
    Ok(())
}

fn valid_choice_style(element: &Element) -> bool {
    if element.attrs.len() != 1 {
        return false;
    }
    let Some(style) = element.attrs.get("style") else {
        return false;
    };
    let Some(color) = style
        .strip_prefix("color: #")
        .and_then(|value| value.strip_suffix("; font-weight:700;"))
    else {
        return false;
    };
    color.len() == 6 && color.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_margin(element: &Element) -> bool {
    element.name == "div"
        && element
            .children
            .iter()
            .all(|child| matches!(child, Node::Text(text) if text.trim().is_empty()))
        && element.attrs.len() == 1
        && element.attrs.get("style").map(String::as_str) == Some("margin-top:1em")
}

fn require_attrs(element: &Element, expected: &[(&str, &str)]) -> Result<(), RendererFailure> {
    if element.attrs.len() != expected.len()
        || expected
            .iter()
            .any(|(name, value)| element.attrs.get(*name).map(String::as_str) != Some(*value))
    {
        return Err(bad(
            "matching markup attributes are outside the supported contract",
        ));
    }
    Ok(())
}

fn significant(children: &[Node]) -> Vec<&Node> {
    children
        .iter()
        .filter(|node| !matches!(node, Node::Text(text) if text.trim().is_empty()))
        .collect()
}

fn plain_text(element: &Element) -> Result<String, RendererFailure> {
    let mut text = String::new();
    for child in &element.children {
        append_plain_node(&mut text, child)?;
    }
    Ok(text)
}

fn append_plain_node(target: &mut String, node: &Node) -> Result<(), RendererFailure> {
    match node {
        Node::Text(text) => push_matching_text(target, text),
        Node::Element(element)
            if element.attrs.is_empty()
                && matches!(element.name.as_str(), "sub" | "sup" | "strong") =>
        {
            for child in &element.children {
                append_plain_node(target, child)?;
            }
            Ok(())
        }
        _ => Err(bad("matching label contains unsupported markup")),
    }
}

fn push_matching_text(target: &mut String, value: &str) -> Result<(), RendererFailure> {
    if target.chars().count().saturating_add(value.chars().count()) > MAX_PROMPT_CHARS {
        return Err(bad("matching text exceeds the supported bound"));
    }
    target.push_str(value);
    Ok(())
}

fn escape_html(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }
    output
}
