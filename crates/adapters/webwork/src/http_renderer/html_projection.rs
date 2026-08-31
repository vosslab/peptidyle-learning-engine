//! Strict structural projection of the bounded upstream RadioButtons fragment.

use html5ever::tendril::StrTendril;
use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};
use sha2::{Digest, Sha256};

use super::*;

pub(super) fn body_html(
    object: &Map<String, Value>,
    service_base: &Url,
) -> Result<String, RendererFailure> {
    let rendered = object
        .get("renderedHTML")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| bad("renderer omitted question body"))?;
    extract_problem_body(rendered, service_base)
}

fn extract_problem_body(html: &str, service_base: &Url) -> Result<String, RendererFailure> {
    if html.len() > DEFAULT_MAX_RESPONSE_BYTES {
        return Err(bad("renderer HTML exceeds the supported bound"));
    }
    let tokens = tokenize_html(html)?;
    let mut stack = Vec::<String>::new();
    let mut body_depth = None;
    let mut body_seen = false;
    let mut base_seen = false;
    let mut form_seen = false;
    let mut output = String::new();

    for token in tokens {
        match token {
            Token::CharacterTokens(text) => {
                if body_depth.is_some() {
                    append_escaped_html(&mut output, text.as_ref());
                }
            }
            Token::TagToken(tag) => match tag.kind {
                TagKind::StartTag => {
                    validate_tag(&tag)?;
                    let name = tag.name.to_string();
                    if name == "base" {
                        if base_seen {
                            return Err(bad("renderer repeated its base URL"));
                        }
                        let href = required_bounded_attribute(&tag, "href", MAX_PG_PATH_BYTES)?;
                        verify_site_url(&href, service_base)?;
                        base_seen = true;
                    }
                    if name == "form" && attribute(&tag, "id").as_deref() == Some("problemMainForm")
                    {
                        if form_seen {
                            return Err(bad("renderer repeated its problem form"));
                        }
                        let action = required_bounded_attribute(&tag, "action", MAX_PG_PATH_BYTES)?;
                        verify_form_action_url(&action, service_base)?;
                        form_seen = true;
                    }
                    let starts_body = name == "div"
                        && attribute(&tag, "id").as_deref() == Some("problem_body")
                        && has_class(&tag, "problem-content");
                    if starts_body {
                        if body_seen || body_depth.is_some() {
                            return Err(bad("renderer repeated its problem body"));
                        }
                        body_seen = true;
                        body_depth = Some(stack.len() + 1);
                    } else if body_depth.is_some() {
                        if matches!(name.as_str(), "script" | "style") {
                            return Err(bad("renderer problem body contains executable markup"));
                        }
                        append_start_tag(&mut output, &tag);
                    }
                    if !is_void_element(&name) {
                        if stack.len() >= MAX_HTML_NESTING {
                            return Err(bad("renderer markup exceeds nesting bound"));
                        }
                        stack.push(name);
                    }
                }
                TagKind::EndTag => {
                    validate_tag(&tag)?;
                    let name = tag.name.to_string();
                    let open = stack
                        .pop()
                        .ok_or_else(|| bad("renderer returned unbalanced markup"))?;
                    if open != name {
                        return Err(bad("renderer returned unbalanced markup"));
                    }
                    if body_depth == Some(stack.len() + 1) {
                        body_depth = None;
                    } else if body_depth.is_some() {
                        append_end_tag(&mut output, &name);
                    }
                }
            },
            Token::DoctypeToken(_) | Token::CommentToken(_) if body_depth.is_none() => {}
            Token::EOFToken => {}
            Token::NullCharacterToken
            | Token::CommentToken(_)
            | Token::DoctypeToken(_)
            | Token::ParseError(_) => return Err(bad("renderer returned malformed HTML")),
        }
    }
    if !stack.is_empty() || body_depth.is_some() || !body_seen || !base_seen || !form_seen {
        return Err(bad(
            "renderer HTML is missing required standalone structure",
        ));
    }
    if output.trim().is_empty() {
        return Err(bad("renderer problem body is empty"));
    }
    Ok(output)
}
#[derive(Debug)]
pub(super) struct Radio {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) label: String,
}
#[derive(Debug)]
pub(super) struct ParsedRadioHtml {
    pub(super) controls: Vec<Radio>,
    pub(super) prompt_text: String,
    pub(super) prompt_html: String,
}

/// Tokenize the renderer fragment with html5ever and accept only the exact PG
/// RadioButtons shape we ship: one container, direct wrapping labels, the
/// empty direct `div` separators (optionally carrying PG's exact spacing
/// style) emitted between PGML choices, and attrless `strong`/`sub` elements
/// within label text.
/// A tokenizer is intentionally used instead of an HTML DOM because browser
/// error recovery would turn malformed hostile markup into a different tree.
pub(super) fn parse_single_radio_group(
    html: &str,
    protected_html_values: &BTreeSet<String>,
) -> Result<ParsedRadioHtml, RendererFailure> {
    if html.len() > DEFAULT_MAX_RESPONSE_BYTES {
        return Err(bad("renderer question body exceeds the supported bound"));
    }
    let tokens = tokenize_html(html)?;
    let mut stack = Vec::<OpenElement>::new();
    let mut controls = Vec::new();
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut values = BTreeSet::new();
    let mut radio_container_seen = false;
    let mut radio_container_depth = None;
    let mut radio_container_allows_prompt = false;
    let mut active_label = None::<ActiveLabel>;
    let mut prompt_text = String::new();
    let mut prompt_html = String::new();

    for token in tokens {
        match token {
            Token::CharacterTokens(text) => {
                reject_protected_text(text.as_ref(), protected_html_values)?;
                if radio_container_depth.is_some() {
                    if let Some(label) = active_label.as_mut() {
                        label.text.push_str(text.as_ref());
                    } else if radio_container_allows_prompt && controls.is_empty() {
                        push_bounded(&mut prompt_text, text.as_ref(), MAX_PROMPT_CHARS)?;
                        append_escaped_html(&mut prompt_html, text.as_ref());
                    } else if !text.trim().is_empty() {
                        return Err(bad("radio group contains unlabeled content"));
                    }
                } else {
                    push_bounded(&mut prompt_text, text.as_ref(), MAX_PROMPT_CHARS)?;
                    append_escaped_html(&mut prompt_html, text.as_ref());
                }
            }
            Token::TagToken(tag) => match tag.kind {
                TagKind::StartTag => {
                    validate_tag(&tag)?;
                    for attribute in &tag.attrs {
                        reject_protected_text(attribute.value.as_ref(), protected_html_values)?;
                    }
                    let name = tag.name.to_string();
                    if name == "script" || name == "style" {
                        return Err(bad("renderer question body contains executable markup"));
                    }
                    let is_container = name == "div"
                        && (has_class(&tag, "radio-buttons-container") || has_class(&tag, "PGML"));
                    let prompt_markup = radio_container_allows_prompt
                        && controls.is_empty()
                        && active_label.is_none()
                        && is_safe_prompt_element(&name);
                    if is_container {
                        if radio_container_seen || radio_container_depth.is_some() {
                            return Err(bad("renderer returned more than one radio group"));
                        }
                        radio_container_seen = true;
                        radio_container_depth = Some(stack.len() + 1);
                        radio_container_allows_prompt = has_class(&tag, "PGML");
                    } else if name == "input" {
                        let depth = radio_container_depth.ok_or_else(|| {
                            bad("renderer question body contains an unsupported input")
                        })?;
                        if stack.len() != depth + 1 || active_label.is_none() {
                            return Err(bad("radio input must be directly wrapped by its label"));
                        }
                        let label = active_label.as_mut().expect("checked active label");
                        if label.radio.is_some() {
                            return Err(bad("radio label contains multiple controls"));
                        }
                        let radio = radio_from_tag(&tag, &mut names, &mut ids, &mut values)?;
                        label.radio = Some(radio);
                    } else if name == "label" {
                        let depth = radio_container_depth
                            .ok_or_else(|| bad("renderer returned a non-radio label"))?;
                        if stack.len() + 1 != depth + 1 || active_label.is_some() {
                            return Err(bad("radio labels must directly wrap one input"));
                        }
                        active_label = Some(ActiveLabel::default());
                    } else if name == "div"
                        && radio_container_depth.is_some_and(|depth| stack.len() == depth)
                        && is_pg_radio_separator(&tag)
                    {
                        // PGML emits an empty direct div between RadioButtons
                        // labels. Any content or nested control still refuses.
                    } else if name == "br"
                        && radio_container_depth.is_some_and(|depth| stack.len() == depth)
                        && tag.attrs.is_empty()
                    {
                        // PG-2.17 emits direct BR separators between PGML labels.
                    } else if active_label.is_some() && is_safe_plain_text_label_element(&tag) {
                        // PGML preserves emphasis, molecular formulae, and its
                        // reviewed color emphasis inside choice labels. The
                        // browser-facing choice is plain text, so only text
                        // content is retained; no renderer markup crosses.
                    } else if prompt_markup {
                        append_start_tag(&mut prompt_html, &tag);
                    } else if radio_container_depth.is_some() {
                        return Err(bad("radio group has unsupported nesting"));
                    } else {
                        append_start_tag(&mut prompt_html, &tag);
                    }
                    if name != "input" && !is_void_element(&name) {
                        if stack.len() >= MAX_HTML_NESTING {
                            return Err(bad("renderer markup exceeds nesting bound"));
                        }
                        stack.push(OpenElement {
                            name,
                            is_container,
                            prompt_markup,
                        });
                    } else if tag.self_closing && name != "input" {
                        // A self-closing non-void tag is not part of the PG fragment contract.
                        return Err(bad("renderer returned malformed self-closing markup"));
                    }
                }
                TagKind::EndTag => {
                    validate_tag(&tag)?;
                    let name = tag.name.to_string();
                    if is_void_element(&name) || name == "input" {
                        return Err(bad("renderer returned malformed void-element close"));
                    }
                    let open = stack
                        .pop()
                        .ok_or_else(|| bad("renderer returned unbalanced markup"))?;
                    if open.name != name {
                        return Err(bad("renderer returned unbalanced markup"));
                    }
                    if name == "label" {
                        let label = active_label
                            .take()
                            .ok_or_else(|| bad("renderer returned malformed radio label"))?;
                        let mut radio = label
                            .radio
                            .ok_or_else(|| bad("radio label lacks a control"))?;
                        let text = label.text.trim();
                        if text.is_empty() || text.chars().count() > MAX_RADIO_LABEL_CHARS {
                            return Err(bad("radio label is outside the supported bound"));
                        }
                        radio.label = text.to_owned();
                        controls.push(radio);
                    }
                    if open.is_container {
                        radio_container_depth = None;
                        radio_container_allows_prompt = false;
                    } else if open.prompt_markup || radio_container_depth.is_none() {
                        append_end_tag(&mut prompt_html, &name);
                    }
                }
            },
            Token::EOFToken => {}
            Token::NullCharacterToken
            | Token::CommentToken(_)
            | Token::DoctypeToken(_)
            | Token::ParseError(_) => return Err(bad("renderer returned malformed HTML")),
        }
    }
    if !stack.is_empty() || radio_container_depth.is_some() || active_label.is_some() {
        return Err(bad("renderer returned unbalanced markup"));
    }
    if !radio_container_seen
        || controls.len() < 2
        || controls.len() > MAX_RADIO_CHOICES
        || names.len() != 1
    {
        return Err(bad("renderer did not return one supported radio group"));
    }
    if prompt_text.trim().is_empty() {
        return Err(bad("renderer prompt is empty"));
    }
    Ok(ParsedRadioHtml {
        controls,
        prompt_text: prompt_text.trim().to_owned(),
        prompt_html,
    })
}

pub(super) fn reject_protected_text(
    value: &str,
    protected_html_values: &BTreeSet<String>,
) -> Result<(), RendererFailure> {
    if protected_html_values
        .iter()
        .any(|protected| value.contains(protected))
    {
        return Err(bad("renderer HTML contained protected material"));
    }
    Ok(())
}

#[derive(Debug)]
struct OpenElement {
    name: String,
    is_container: bool,
    prompt_markup: bool,
}

#[derive(Debug, Default)]
struct ActiveLabel {
    radio: Option<Radio>,
    text: String,
}

pub(super) fn tokenize_html(html: &str) -> Result<Vec<Token>, RendererFailure> {
    use std::cell::{Cell, RefCell};

    struct Sink {
        tokens: RefCell<Vec<Token>>,
        overflow: Cell<bool>,
    }
    impl TokenSink for Sink {
        type Handle = ();
        fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<Self::Handle> {
            let mut tokens = self.tokens.borrow_mut();
            if tokens.len() >= MAX_HTML_TOKENS {
                self.overflow.set(true);
            } else {
                tokens.push(token);
            }
            TokenSinkResult::Continue
        }
    }
    let sink = Sink {
        tokens: RefCell::new(Vec::new()),
        overflow: Cell::new(false),
    };
    let tokenizer = Tokenizer::new(
        sink,
        TokenizerOpts {
            exact_errors: true,
            ..TokenizerOpts::default()
        },
    );
    let input = BufferQueue::default();
    input.push_back(StrTendril::from_slice(html));
    while !input.is_empty() {
        let _ = tokenizer.feed(&input);
    }
    tokenizer.end();
    if tokenizer.sink.overflow.get() {
        return Err(bad("renderer markup exceeds token bound"));
    }
    Ok(tokenizer.sink.tokens.into_inner())
}

pub(super) fn validate_tag(tag: &Tag) -> Result<(), RendererFailure> {
    if tag.had_duplicate_attributes {
        return Err(bad("renderer markup contains duplicate attributes"));
    }
    if tag.kind == TagKind::EndTag && (!tag.attrs.is_empty() || tag.self_closing) {
        return Err(bad("renderer markup has malformed closing tag"));
    }
    Ok(())
}

fn radio_from_tag(
    tag: &Tag,
    names: &mut BTreeSet<String>,
    ids: &mut BTreeSet<String>,
    values: &mut BTreeSet<String>,
) -> Result<Radio, RendererFailure> {
    if tag.self_closing
        || !attribute(tag, "type").is_some_and(|value| value.eq_ignore_ascii_case("radio"))
    {
        return Err(bad("renderer question body contains an unsupported input"));
    }
    let name = required_bounded_attribute(tag, "name", MAX_RADIO_FIELD_BYTES)?;
    if !name.starts_with("AnSwEr")
        || !name[6..].bytes().all(|byte| byte.is_ascii_digit())
        || matches!(
            name.as_str(),
            "courseID" | "user" | "passwd" | "problemSource" | "WWsubmit"
        )
    {
        return Err(bad(
            "renderer radio name is outside the supported upstream contract",
        ));
    }
    let value = required_bounded_attribute(tag, "value", MAX_RADIO_VALUE_BYTES)?;
    let id = required_bounded_attribute(tag, "id", MAX_RADIO_FIELD_BYTES)?;
    if !ids.insert(id) || !values.insert(value.clone()) {
        return Err(bad("renderer repeated radio identifier"));
    }
    names.insert(name.clone());
    Ok(Radio {
        name,
        value,
        label: String::new(),
    })
}

pub(super) fn attribute(tag: &Tag, name: &str) -> Option<String> {
    tag.attrs
        .iter()
        .find(|attribute| attribute.name.local.as_ref() == name)
        .map(|attribute| attribute.value.to_string())
}

pub(super) fn required_bounded_attribute(
    tag: &Tag,
    name: &str,
    maximum: usize,
) -> Result<String, RendererFailure> {
    let value =
        attribute(tag, name).ok_or_else(|| bad("radio control lacks required attribute"))?;
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(bad(
            "radio control attribute is outside the supported bound",
        ));
    }
    Ok(value)
}

pub(super) fn has_class(tag: &Tag, wanted: &str) -> bool {
    attribute(tag, "class").is_some_and(|classes| {
        classes
            .split_ascii_whitespace()
            .any(|class| class == wanted)
    })
}

fn is_pg_radio_separator(tag: &Tag) -> bool {
    tag.attrs.is_empty()
        || (tag.attrs.len() == 1
            && matches!(
                attribute(tag, "style").as_deref(),
                Some("margin-bottom: 0.7em;") | Some("margin-top:1em")
            ))
}

fn is_safe_plain_text_label_element(tag: &Tag) -> bool {
    let name = tag.name.to_string();
    if matches!(name.as_str(), "strong" | "sub" | "sup") {
        return tag.attrs.is_empty();
    }
    name == "span"
        && tag.attrs.len() == 1
        && attribute(tag, "style").is_some_and(|style| {
            let Some(color) = style
                .strip_prefix("color: #")
                .and_then(|value| value.strip_suffix("; font-weight:700;"))
            else {
                return false;
            };
            color.len() == 6 && color.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

fn is_safe_prompt_element(name: &str) -> bool {
    matches!(
        name,
        "p" | "span" | "br" | "strong" | "b" | "em" | "i" | "sub" | "sup"
    )
}

fn is_void_element(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

pub(super) fn push_bounded(
    target: &mut String,
    value: &str,
    maximum: usize,
) -> Result<(), RendererFailure> {
    if target.chars().count().saturating_add(value.chars().count()) > maximum {
        return Err(bad("renderer prompt exceeds the supported bound"));
    }
    target.push_str(value);
    Ok(())
}

fn append_escaped_html(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }
}

fn append_start_tag(output: &mut String, tag: &Tag) {
    use std::fmt::Write as _;
    let _ = write!(output, "<{}", tag.name);
    for attribute in &tag.attrs {
        let _ = write!(output, " {}=\"", attribute.name.local);
        append_escaped_html(output, attribute.value.as_ref());
        output.push('\"');
    }
    output.push('>');
}

fn append_end_tag(output: &mut String, name: &str) {
    output.push_str("</");
    output.push_str(name);
    output.push('>');
}

pub(super) fn opaque_choice_id(
    request: RenderRequest<'_>,
    ordinal: usize,
) -> Result<ResponseItemReference, RendererFailure> {
    opaque_item_id(request, 0, ordinal)
}

pub(super) fn opaque_item_id(
    request: RenderRequest<'_>,
    role: u32,
    ordinal: usize,
) -> Result<ResponseItemReference, RendererFailure> {
    let mut hash = Sha256::new();
    hash.update(b"ple:webwork:choice:v1\0");
    hash.update(request.question_revision.question_id.to_string().as_bytes());
    hash.update(
        request
            .question_revision
            .revision_number
            .get()
            .to_be_bytes(),
    );
    hash.update(request.seed.to_be_bytes());
    hash.update(role.to_be_bytes());
    hash.update((ordinal as u32).to_be_bytes());
    let digest = hash.finalize();
    let mut encoded = String::with_capacity(32);
    for byte in &digest[..16] {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    Ok(ResponseItemReference::new(format!("ww-{encoded}")))
}
