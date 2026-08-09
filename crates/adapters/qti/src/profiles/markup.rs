//! Strict, bounded QTI text projection into the small Markdown subset PLE owns.
//!
//! Canvas stores HTML in `mattext`; Blackboard stores markup directly in the XML
//! tree.  They deliberately enter through different functions so XML text is
//! never accidentally reparsed as HTML.  `html5ever` is used only as a tokenizer
//! for Canvas fragments: its forgiving DOM/tree builder would hide malformed
//! source that this import boundary must refuse.

use std::cell::{Cell, RefCell};
use std::fmt;

use html5ever::tendril::StrTendril;
use html5ever::tokenizer::{
    BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};

use super::{
    BLACKBOARD_ITEM_NAMESPACE, QtiProfileDiagnosticCode, QtiSafeDiagnostic,
    QtiSafeDiagnosticLocation, QtiSafeDiagnosticTemplate,
};
use crate::xml::{XmlContentRef, XmlNode};

mod renderer;
use renderer::render_document;

const XHTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
const XMLNS_NAMESPACE: &str = "http://www.w3.org/2000/xmlns/";
const MAX_MARKUP_INPUT_BYTES: usize = 65_536;
const MAX_MARKUP_TOKENS: usize = 8_192;
const MAX_MARKUP_NESTING: usize = 32;

/// Limits for one independently projected text field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MarkupLimits {
    max_input_bytes: usize,
    max_tokens: usize,
    max_nesting: usize,
    max_output_chars: usize,
}

impl MarkupLimits {
    pub(crate) const PROMPT: Self = Self {
        max_input_bytes: MAX_MARKUP_INPUT_BYTES,
        max_tokens: MAX_MARKUP_TOKENS,
        max_nesting: MAX_MARKUP_NESTING,
        max_output_chars: 65_536,
    };

    pub(crate) const CHOICE: Self = Self {
        max_input_bytes: MAX_MARKUP_INPUT_BYTES,
        max_tokens: MAX_MARKUP_TOKENS,
        max_nesting: MAX_MARKUP_NESTING,
        max_output_chars: 16_384,
    };
}

/// A source-independent refusal reason suitable for a bounded item report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QtiMarkupError {
    InputLimit,
    TokenLimit,
    NestingLimit,
    OutputLimit,
    UnsupportedMarkup,
    InvalidStructure,
    InvalidNamespace,
}

impl QtiMarkupError {
    pub(crate) fn safe_diagnostic(self, location: QtiSafeDiagnosticLocation) -> QtiSafeDiagnostic {
        QtiSafeDiagnostic::new(
            QtiProfileDiagnosticCode::Markup,
            location,
            QtiSafeDiagnosticTemplate::UnsupportedMarkup,
        )
        .expect("fixed markup diagnostic is safe")
    }
}

impl fmt::Display for QtiMarkupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("QTI markup cannot be projected by the supported text profile")
    }
}

impl std::error::Error for QtiMarkupError {}

/// Projects Canvas `mattext` ordered content by parsing exactly one HTML layer.
pub(crate) fn project_canvas_mattext(
    mattext: &XmlNode,
    limits: MarkupLimits,
) -> Result<String, QtiMarkupError> {
    let mut html = String::new();
    for content in mattext.content() {
        match content {
            // XML text is decoded once by `parse_xml`; html5ever owns the one
            // subsequent HTML entity decode.  Do not unescape it again here.
            XmlContentRef::Text(text) | XmlContentRef::Cdata(text) => {
                let next_len = html.len().saturating_add(text.len());
                if next_len > limits.max_input_bytes {
                    return Err(QtiMarkupError::InputLimit);
                }
                html.push_str(text);
            }
            XmlContentRef::Child(_)
            | XmlContentRef::Comment(_)
            | XmlContentRef::ProcessingInstruction { .. } => {
                return Err(QtiMarkupError::UnsupportedMarkup);
            }
        }
    }
    let nodes = tokenize_canvas_html(&html, limits)?;
    render_document(&nodes, limits)
}

/// Projects Blackboard's direct XML/XHTML child stream without HTML reparsing.
pub(crate) fn project_blackboard_xhtml(
    parent: &XmlNode,
    limits: MarkupLimits,
) -> Result<String, QtiMarkupError> {
    validate_blackboard_container(parent)?;
    let mut input_bytes = 0_usize;
    let mut tokens = 0_usize;
    let nodes = direct_children(parent, limits, &mut input_bytes, &mut tokens, 0)?;
    validate_tree(&nodes, limits)?;
    render_document(&nodes, limits)
}

/// Projects an `itemBody` prompt while excluding its validated choice control.
pub(crate) fn project_blackboard_item_body(
    body: &XmlNode,
    limits: MarkupLimits,
) -> Result<String, QtiMarkupError> {
    validate_blackboard_container(body)?;
    let mut input_bytes = 0_usize;
    let mut tokens = 0_usize;
    let mut nodes = Vec::new();
    for content in body.content() {
        if let XmlContentRef::Child(index) = content
            && body.children().get(index).is_some_and(|child| {
                child.name() == "choiceInteraction"
                    && child.namespace_uri() == Some(BLACKBOARD_ITEM_NAMESPACE)
            })
        {
            continue;
        }
        tokens = tokens.saturating_add(1);
        if tokens > limits.max_tokens {
            return Err(QtiMarkupError::TokenLimit);
        }
        match content {
            XmlContentRef::Text(text) | XmlContentRef::Cdata(text) => {
                input_bytes = input_bytes.saturating_add(text.len());
                if input_bytes > limits.max_input_bytes {
                    return Err(QtiMarkupError::InputLimit);
                }
                nodes.push(Node::Text(text.to_string()));
            }
            XmlContentRef::Child(index) => {
                let child = body
                    .children()
                    .get(index)
                    .ok_or(QtiMarkupError::InvalidStructure)?;
                nodes.push(direct_element(
                    child,
                    limits,
                    &mut input_bytes,
                    &mut tokens,
                    1,
                )?);
            }
            XmlContentRef::Comment(_) | XmlContentRef::ProcessingInstruction { .. } => {
                return Err(QtiMarkupError::UnsupportedMarkup);
            }
        }
    }
    validate_tree(&nodes, limits)?;
    render_document(&nodes, limits)
}

/// Projects a validated `simpleChoice`; its profile-owned identifier and fixed
/// attributes are not markup attributes.
pub(crate) fn project_blackboard_choice(
    choice: &XmlNode,
    limits: MarkupLimits,
) -> Result<String, QtiMarkupError> {
    if choice.namespace_uri() != Some(BLACKBOARD_ITEM_NAMESPACE) {
        return Err(QtiMarkupError::InvalidNamespace);
    }
    let mut input_bytes = 0_usize;
    let mut tokens = 0_usize;
    let nodes = direct_children(choice, limits, &mut input_bytes, &mut tokens, 0)?;
    validate_tree(&nodes, limits)?;
    render_document(&nodes, limits)
}

fn tokenize_canvas_html(source: &str, limits: MarkupLimits) -> Result<Vec<Node>, QtiMarkupError> {
    let sink = HtmlTokenSink::new(limits.max_tokens);
    let tokenizer = Tokenizer::new(
        sink,
        TokenizerOpts {
            exact_errors: true,
            ..TokenizerOpts::default()
        },
    );
    let input = BufferQueue::default();
    input.push_back(StrTendril::from_slice(source));
    while !input.is_empty() {
        let _ = tokenizer.feed(&input);
    }
    tokenizer.end();
    if tokenizer.sink.overflow.get() {
        return Err(QtiMarkupError::TokenLimit);
    }
    let tokens = tokenizer.sink.tokens.into_inner();
    html_tokens_to_nodes(tokens, limits)
}

struct HtmlTokenSink {
    tokens: RefCell<Vec<Token>>,
    max_tokens: usize,
    overflow: Cell<bool>,
}

impl HtmlTokenSink {
    fn new(max_tokens: usize) -> Self {
        Self {
            tokens: RefCell::new(Vec::new()),
            max_tokens,
            overflow: Cell::new(false),
        }
    }
}

impl TokenSink for HtmlTokenSink {
    type Handle = ();

    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<Self::Handle> {
        let mut tokens = self.tokens.borrow_mut();
        if tokens.len() < self.max_tokens {
            tokens.push(token);
        } else {
            self.overflow.set(true);
        }
        TokenSinkResult::Continue
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Element {
    Root,
    Paragraph,
    Division,
    Strong,
    Emphasis,
    Code,
    UnorderedList,
    OrderedList,
    ListItem,
}

impl Element {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "p" => Some(Self::Paragraph),
            "div" => Some(Self::Division),
            "strong" | "b" => Some(Self::Strong),
            "em" | "i" => Some(Self::Emphasis),
            "code" => Some(Self::Code),
            "ul" => Some(Self::UnorderedList),
            "ol" => Some(Self::OrderedList),
            "li" => Some(Self::ListItem),
            _ => None,
        }
    }

    fn accepts_text(self) -> bool {
        !matches!(self, Self::UnorderedList | Self::OrderedList)
    }

    fn accepts_child(self, child: Self) -> bool {
        let inline = matches!(child, Self::Strong | Self::Emphasis | Self::Code);
        match self {
            Self::Root => {
                inline
                    || matches!(
                        child,
                        Self::Paragraph | Self::Division | Self::UnorderedList | Self::OrderedList
                    )
            }
            Self::Paragraph | Self::Division | Self::ListItem | Self::Strong | Self::Emphasis => {
                inline
            }
            Self::Code => false,
            Self::UnorderedList | Self::OrderedList => child == Self::ListItem,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Text(String),
    Break,
    Element(Element, Vec<Node>),
}

struct Frame {
    element: Element,
    children: Vec<Node>,
}

fn html_tokens_to_nodes(
    tokens: Vec<Token>,
    limits: MarkupLimits,
) -> Result<Vec<Node>, QtiMarkupError> {
    let mut stack = vec![Frame {
        element: Element::Root,
        children: Vec::new(),
    }];
    for token in tokens {
        match token {
            Token::CharacterTokens(text) => append_text(&mut stack, text.as_ref())?,
            Token::NullCharacterToken
            | Token::CommentToken(_)
            | Token::DoctypeToken(_)
            | Token::ParseError(_) => return Err(QtiMarkupError::UnsupportedMarkup),
            Token::EOFToken => {}
            Token::TagToken(tag) => {
                if tag.had_duplicate_attributes || !tag.attrs.is_empty() {
                    return Err(QtiMarkupError::UnsupportedMarkup);
                }
                let name = tag.name.as_ref();
                if name == "br" {
                    if tag.kind != TagKind::StartTag
                        || !stack_last(&stack)?.accepts_text()
                        || stack_last(&stack)? == Element::Code
                    {
                        return Err(QtiMarkupError::InvalidStructure);
                    }
                    stack_last_mut(&mut stack)?.children.push(Node::Break);
                    continue;
                }
                let element = Element::from_name(name).ok_or(QtiMarkupError::UnsupportedMarkup)?;
                match tag.kind {
                    TagKind::StartTag => {
                        if tag.self_closing || !stack_last(&stack)?.accepts_child(element) {
                            return Err(QtiMarkupError::InvalidStructure);
                        }
                        if stack.len() >= limits.max_nesting {
                            return Err(QtiMarkupError::NestingLimit);
                        }
                        stack.push(Frame {
                            element,
                            children: Vec::new(),
                        });
                    }
                    TagKind::EndTag => close_frame(&mut stack, element)?,
                }
            }
        }
    }
    if stack.len() != 1 {
        return Err(QtiMarkupError::InvalidStructure);
    }
    Ok(stack.pop().expect("root frame exists").children)
}

fn direct_children(
    parent: &XmlNode,
    limits: MarkupLimits,
    input_bytes: &mut usize,
    tokens: &mut usize,
    depth: usize,
) -> Result<Vec<Node>, QtiMarkupError> {
    let mut nodes = Vec::new();
    for content in parent.content() {
        *tokens = tokens.saturating_add(1);
        if *tokens > limits.max_tokens {
            return Err(QtiMarkupError::TokenLimit);
        }
        match content {
            XmlContentRef::Text(text) | XmlContentRef::Cdata(text) => {
                *input_bytes = input_bytes.saturating_add(text.len());
                if *input_bytes > limits.max_input_bytes {
                    return Err(QtiMarkupError::InputLimit);
                }
                if matches!(parent.name(), "ul" | "ol") && text.trim().is_empty() {
                    continue;
                }
                nodes.push(Node::Text(text.to_string()));
            }
            XmlContentRef::Child(index) => {
                let child = parent
                    .children()
                    .get(index)
                    .ok_or(QtiMarkupError::InvalidStructure)?;
                nodes.push(direct_element(
                    child,
                    limits,
                    input_bytes,
                    tokens,
                    depth + 1,
                )?);
            }
            XmlContentRef::Comment(_) | XmlContentRef::ProcessingInstruction { .. } => {
                return Err(QtiMarkupError::UnsupportedMarkup);
            }
        }
    }
    Ok(nodes)
}

fn direct_element(
    node: &XmlNode,
    limits: MarkupLimits,
    input_bytes: &mut usize,
    tokens: &mut usize,
    depth: usize,
) -> Result<Node, QtiMarkupError> {
    if depth > limits.max_nesting {
        return Err(QtiMarkupError::NestingLimit);
    }
    validate_blackboard_node(node)?;
    *input_bytes = input_bytes.saturating_add(node.name().len().saturating_add(5));
    if *input_bytes > limits.max_input_bytes {
        return Err(QtiMarkupError::InputLimit);
    }
    let element = Element::from_name(node.name()).ok_or(QtiMarkupError::UnsupportedMarkup)?;
    let children = direct_children(node, limits, input_bytes, tokens, depth)?;
    Ok(Node::Element(element, children))
}

fn validate_blackboard_node(node: &XmlNode) -> Result<(), QtiMarkupError> {
    if !matches!(node.namespace_uri(), Some(uri) if uri == BLACKBOARD_ITEM_NAMESPACE || uri == XHTML_NAMESPACE)
    {
        return Err(QtiMarkupError::InvalidNamespace);
    }
    if node.attributes().next().is_some() {
        return Err(QtiMarkupError::UnsupportedMarkup);
    }
    Ok(())
}

fn validate_blackboard_container(node: &XmlNode) -> Result<(), QtiMarkupError> {
    if !matches!(node.namespace_uri(), Some(uri) if uri == BLACKBOARD_ITEM_NAMESPACE || uri == XHTML_NAMESPACE)
    {
        return Err(QtiMarkupError::InvalidNamespace);
    }
    for attribute in node.attributes() {
        if attribute.prefix().is_some()
            || attribute.local_name() != "xmlns"
            || attribute.namespace_uri() != Some(XMLNS_NAMESPACE)
            || attribute.value() != node.namespace_uri().unwrap_or_default()
        {
            return Err(QtiMarkupError::UnsupportedMarkup);
        }
    }
    Ok(())
}

fn validate_tree(nodes: &[Node], limits: MarkupLimits) -> Result<Vec<Node>, QtiMarkupError> {
    let mut stack = vec![Element::Root];
    for node in nodes {
        validate_node(node, &mut stack, limits)?;
    }
    Ok(nodes.to_vec())
}

fn validate_node(
    node: &Node,
    stack: &mut Vec<Element>,
    limits: MarkupLimits,
) -> Result<(), QtiMarkupError> {
    match node {
        Node::Text(text) => {
            if !element_stack_last(stack)?.accepts_text() && !text.trim().is_empty() {
                return Err(QtiMarkupError::InvalidStructure);
            }
        }
        Node::Break => {
            if !element_stack_last(stack)?.accepts_text()
                || matches!(element_stack_last(stack)?, Element::Code)
            {
                return Err(QtiMarkupError::InvalidStructure);
            }
        }
        Node::Element(element, children) => {
            if !element_stack_last(stack)?.accepts_child(*element) {
                return Err(QtiMarkupError::InvalidStructure);
            }
            if stack.len() >= limits.max_nesting {
                return Err(QtiMarkupError::NestingLimit);
            }
            stack.push(*element);
            for child in children {
                validate_node(child, stack, limits)?;
            }
            stack.pop();
        }
    }
    Ok(())
}

fn append_text(stack: &mut [Frame], text: &str) -> Result<(), QtiMarkupError> {
    let parent = stack_last_mut(stack)?;
    if !parent.element.accepts_text() {
        if text.trim().is_empty() {
            return Ok(());
        }
        return Err(QtiMarkupError::InvalidStructure);
    }
    parent.children.push(Node::Text(text.to_string()));
    Ok(())
}

fn close_frame(stack: &mut Vec<Frame>, expected: Element) -> Result<(), QtiMarkupError> {
    let frame = stack.pop().ok_or(QtiMarkupError::InvalidStructure)?;
    if frame.element != expected || frame.element == Element::Root {
        return Err(QtiMarkupError::InvalidStructure);
    }
    stack_last_mut(stack)?
        .children
        .push(Node::Element(frame.element, frame.children));
    Ok(())
}

fn stack_last(stack: &[Frame]) -> Result<Element, QtiMarkupError> {
    stack
        .last()
        .map(|frame| frame.element)
        .ok_or(QtiMarkupError::InvalidStructure)
}

fn element_stack_last(stack: &[Element]) -> Result<Element, QtiMarkupError> {
    stack
        .last()
        .copied()
        .ok_or(QtiMarkupError::InvalidStructure)
}

fn stack_last_mut(stack: &mut [Frame]) -> Result<&mut Frame, QtiMarkupError> {
    stack.last_mut().ok_or(QtiMarkupError::InvalidStructure)
}

#[cfg(test)]
#[path = "markup/tests.rs"]
mod tests;
