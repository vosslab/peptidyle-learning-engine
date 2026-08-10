//! Pagination and line-wrapping for the deterministic PDF writer.

use question_model::AssetId;

use crate::FlowBlock;

use super::render::EmbeddedFont;

pub(super) const IMAGE_LINES: usize = 14;
const LINES_PER_PAGE: usize = 45;
const LEFT_MARGIN: f32 = 72.0;
pub(super) const RIGHT_MARGIN: f32 = 72.0;
pub(super) const PAGE_WIDTH: f32 = 612.0;
pub(super) const TEXT_SIZE: f32 = 11.0;
pub(super) const TEXT_WIDTH: f32 = PAGE_WIDTH - LEFT_MARGIN - RIGHT_MARGIN;

#[derive(Debug, Clone)]
pub(super) enum RenderBlock {
    Text {
        lines: Vec<String>,
        keep_with_next: bool,
    },
    Image(AssetId),
}

pub(super) fn paginate(
    questions: Vec<Vec<FlowBlock>>,
    font: &EmbeddedFont,
) -> Vec<Vec<RenderBlock>> {
    let mut pages = vec![Vec::new()];
    let mut used = 0;
    for question in questions {
        let blocks = flatten(question, font);
        let cost = blocks.iter().map(block_lines).sum::<usize>();
        // Keep an ordinary question intact. An unusually long question starts
        // on a fresh page then splits only at complete visual/text blocks.
        if cost <= LINES_PER_PAGE && used > 0 && used + cost > LINES_PER_PAGE {
            pages.push(Vec::new());
            used = 0;
        }
        for (index, block) in blocks.iter().enumerate() {
            let block_cost = block_lines(block);
            // A heading, choice label, or ordering label belongs with the first
            // following visual block. When the whole pair fits a page, keep it
            // intact; when the following block is longer than a page, keep at
            // least its first rendered line with the label.
            let following_cost = blocks.get(index + 1).map(block_lines);
            let required = match (&block, following_cost) {
                (
                    RenderBlock::Text {
                        keep_with_next: true,
                        ..
                    },
                    Some(next),
                ) => block_cost + next.min(1),
                _ => block_cost,
            };
            if used > 0 && required <= LINES_PER_PAGE && used + required > LINES_PER_PAGE {
                pages.push(Vec::new());
                used = 0;
            }
            if used > 0 && used + block_cost > LINES_PER_PAGE {
                pages.push(Vec::new());
                used = 0;
            }
            if block_cost > LINES_PER_PAGE {
                // long text is already wrapped; emit across pages safely.
                if let RenderBlock::Text { lines, .. } = block {
                    for line in lines {
                        if used == LINES_PER_PAGE {
                            pages.push(Vec::new());
                            used = 0;
                        }
                        pages.last_mut().expect("page").push(RenderBlock::Text {
                            lines: vec![line.clone()],
                            keep_with_next: false,
                        });
                        used += 1;
                    }
                } else {
                    pages.last_mut().expect("page").push(block.clone());
                    used += IMAGE_LINES;
                }
            } else {
                pages.last_mut().expect("page").push(block.clone());
                used += block_cost;
            }
        }
    }
    pages
}

fn flatten(question: Vec<FlowBlock>, font: &EmbeddedFont) -> Vec<RenderBlock> {
    let mut output = Vec::new();
    for block in question {
        match block {
            FlowBlock::Text {
                text,
                keep_with_next,
            } => output.push(RenderBlock::Text {
                lines: wrap(&text, font, TEXT_WIDTH),
                keep_with_next,
            }),
            FlowBlock::Image { asset, .. } => output.push(RenderBlock::Image(asset)),
        }
    }
    output
}

fn block_lines(block: &RenderBlock) -> usize {
    match block {
        RenderBlock::Text { lines, .. } => lines.len(),
        RenderBlock::Image(_) => IMAGE_LINES,
    }
}

/// Wraps whitespace-delimited prose and also splits an over-wide token. This
/// prevents an accession number, URL, or code identifier from running off the
/// printed page.
pub(super) fn wrap(line: &str, font: &EmbeddedFont, width: f32) -> Vec<String> {
    if font.text_width(line, TEXT_SIZE) <= width {
        return vec![line.to_string()];
    }
    let mut output = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        for piece in split_token(word, font, width) {
            let candidate = if current.is_empty() {
                piece.clone()
            } else {
                format!("{current} {piece}")
            };
            if font.text_width(&candidate, TEXT_SIZE) > width && !current.is_empty() {
                output.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&piece);
        }
    }
    if !current.is_empty() {
        output.push(current);
    }
    output
}

fn split_token(token: &str, font: &EmbeddedFont, width: f32) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut piece = String::new();
    for character in token.chars() {
        let mut candidate = piece.clone();
        candidate.push(character);
        if !piece.is_empty() && font.text_width(&candidate, TEXT_SIZE) > width {
            pieces.push(std::mem::take(&mut piece));
        }
        piece.push(character);
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    pieces
}
