//! Deterministic CommonMark rendering for the already-validated markup tree.

use super::{Element, MarkupLimits, Node, QtiMarkupError};

pub(super) fn render_document(
    nodes: &[Node],
    limits: MarkupLimits,
) -> Result<String, QtiMarkupError> {
    let mut writer = MarkdownWriter::new(limits);
    let mut previous_was_block = false;
    let mut started = false;
    for (index, node) in nodes.iter().enumerate() {
        if is_whitespace_text(node) {
            let next_is_block = nodes[index.saturating_add(1)..]
                .iter()
                .find(|candidate| !is_whitespace_text(candidate))
                .is_some_and(is_block);
            if !started || previous_was_block || next_is_block {
                continue;
            }
        }
        let is_block = is_block(node);
        if started && (is_block || previous_was_block) {
            writer.finish_block();
            writer.literal("\n\n")?;
        }
        if is_block {
            render_block(node, &mut writer)?;
            writer.finish_block();
        } else {
            render_inline_node(node, &mut writer)?;
        }
        started = writer.has_content();
        previous_was_block = is_block;
    }
    writer.finish_block();
    Ok(writer.into_string())
}

fn is_whitespace_text(node: &Node) -> bool {
    matches!(node, Node::Text(text) if text.trim().is_empty())
}

struct MarkdownWriter {
    output: String,
    limits: MarkupLimits,
    output_chars: usize,
    pending_space: bool,
}

impl MarkdownWriter {
    fn new(limits: MarkupLimits) -> Self {
        Self {
            output: String::new(),
            limits,
            output_chars: 0,
            pending_space: false,
        }
    }

    fn has_content(&self) -> bool {
        self.output_chars > 0
    }

    fn finish_block(&mut self) {
        self.pending_space = false;
    }

    fn text(&mut self, value: &str) -> Result<bool, QtiMarkupError> {
        let mut visible = false;
        for character in value.chars() {
            if character.is_whitespace() {
                self.pending_space = true;
            } else {
                self.flush_space()?;
                self.escaped_character(character)?;
                visible = true;
            }
        }
        Ok(visible)
    }

    fn literal(&mut self, value: &str) -> Result<(), QtiMarkupError> {
        self.flush_space()?;
        for character in value.chars() {
            self.push(character)?;
        }
        Ok(())
    }

    fn raw_character(&mut self, character: char) -> Result<(), QtiMarkupError> {
        self.flush_space()?;
        self.push(character)
    }

    fn flush_space(&mut self) -> Result<(), QtiMarkupError> {
        if self.pending_space {
            if self.output_chars > 0 {
                self.push(' ')?;
            }
            self.pending_space = false;
        }
        Ok(())
    }

    fn escaped_character(&mut self, character: char) -> Result<(), QtiMarkupError> {
        match character {
            '&' => self.literal("&amp;"),
            '<' => self.literal("&lt;"),
            '>' => self.literal("&gt;"),
            '\\' | '*' | '_' | '`' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '!' | '.' => {
                self.raw_character('\\')?;
                self.raw_character(character)
            }
            _ => self.raw_character(character),
        }
    }

    fn push(&mut self, character: char) -> Result<(), QtiMarkupError> {
        let next = self.output_chars.saturating_add(1);
        if next > self.limits.max_output_chars {
            return Err(QtiMarkupError::OutputLimit);
        }
        self.output.push(character);
        self.output_chars = next;
        Ok(())
    }

    fn into_string(self) -> String {
        self.output
    }
}

fn is_block(node: &Node) -> bool {
    matches!(
        node,
        Node::Element(
            Element::Paragraph | Element::Division | Element::UnorderedList | Element::OrderedList,
            _
        )
    )
}

fn render_block(node: &Node, writer: &mut MarkdownWriter) -> Result<(), QtiMarkupError> {
    let Node::Element(element, children) = node else {
        return Err(QtiMarkupError::InvalidStructure);
    };
    match element {
        Element::Paragraph | Element::Division => render_inline(children, writer).map(|_| ()),
        Element::UnorderedList | Element::OrderedList => render_list(*element, children, writer),
        _ => Err(QtiMarkupError::InvalidStructure),
    }
}

fn render_list(
    element: Element,
    children: &[Node],
    writer: &mut MarkdownWriter,
) -> Result<(), QtiMarkupError> {
    let mut item_index = 0_usize;
    for child in children {
        let Node::Element(Element::ListItem, item_children) = child else {
            return Err(QtiMarkupError::InvalidStructure);
        };
        if item_index > 0 {
            writer.finish_block();
            writer.literal("\n")?;
        }
        item_index = item_index.saturating_add(1);
        if element == Element::UnorderedList {
            writer.literal("- ")?;
        } else {
            writer.literal(&item_index.to_string())?;
            writer.literal(". ")?;
        }
        if !render_inline(item_children, writer)? {
            return Err(QtiMarkupError::InvalidStructure);
        }
        writer.finish_block();
    }
    if item_index == 0 {
        return Err(QtiMarkupError::InvalidStructure);
    }
    Ok(())
}

fn render_inline(nodes: &[Node], writer: &mut MarkdownWriter) -> Result<bool, QtiMarkupError> {
    let mut visible = false;
    for node in nodes {
        visible |= render_inline_node(node, writer)?;
    }
    Ok(visible)
}

fn render_inline_node(node: &Node, writer: &mut MarkdownWriter) -> Result<bool, QtiMarkupError> {
    match node {
        Node::Text(text) => writer.text(text),
        Node::Break => {
            writer.finish_block();
            writer.literal("  \n")?;
            Ok(true)
        }
        Node::Element(Element::Strong, children) => {
            writer.literal("**")?;
            let visible = render_inline(children, writer)?;
            writer.literal("**")?;
            Ok(visible)
        }
        Node::Element(Element::Emphasis, children) => {
            writer.literal("*")?;
            let visible = render_inline(children, writer)?;
            writer.literal("*")?;
            Ok(visible)
        }
        Node::Element(Element::Code, children) => render_code(children, writer),
        Node::Element(_, _) => Err(QtiMarkupError::InvalidStructure),
    }
}

fn render_code(children: &[Node], writer: &mut MarkdownWriter) -> Result<bool, QtiMarkupError> {
    let longest = longest_backtick_run(children)?;
    for _ in 0..=longest {
        writer.literal("`")?;
    }
    let mut visible = false;
    for child in children {
        let Node::Text(text) = child else {
            return Err(QtiMarkupError::InvalidStructure);
        };
        for character in text.chars() {
            if character.is_whitespace() {
                writer.pending_space = true;
            } else {
                writer.flush_space()?;
                writer.raw_character(character)?;
                visible = true;
            }
        }
    }
    writer.flush_space()?;
    for _ in 0..=longest {
        writer.literal("`")?;
    }
    Ok(visible)
}

fn longest_backtick_run(children: &[Node]) -> Result<usize, QtiMarkupError> {
    let mut longest = 0_usize;
    let mut current = 0_usize;
    for child in children {
        let Node::Text(text) = child else {
            return Err(QtiMarkupError::InvalidStructure);
        };
        for character in text.chars() {
            if character == '`' {
                current = current.saturating_add(1);
                longest = longest.max(current);
            } else {
                current = 0;
            }
        }
    }
    Ok(longest)
}
