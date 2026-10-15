//! HTML display inspection and immutable inline-image binding.

use std::collections::{BTreeMap, BTreeSet};

use question_model::{QuestionContentBlock, QuestionHtmlImageBinding, QuestionResponseFormat};
use scraper::{Html, Selector};
use url::Url;

use super::{PleQuestionJsonError, invalid};

/// Inspects parsed image elements; comments, scripts and inventory are not usage.
pub(super) fn relative_image_names(html: &str) -> Result<Vec<String>, PleQuestionJsonError> {
    let document = Html::parse_fragment(html);
    let selector = Selector::parse("img[src]").expect("fixed image selector is valid");
    let mut names = BTreeSet::new();
    for image in document.select(&selector) {
        let name = image.value().attr("src").expect("selector requires src");
        if let Ok(url) = Url::parse(name) {
            // ASVS 1.3.6: remote image declarations are inspected, never fetched.
            if url.scheme() != "https"
                || url.host().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return invalid("external display images require an absolute HTTPS URL");
            }
            continue;
        }
        // ASVS 2.2.1: local images match source-relative registry names exactly.
        if name.trim().is_empty()
            || name.len() > 4096
            || name.starts_with('/')
            || name.contains(['\\', '?', '#', ':'])
            || name.chars().any(|character| character.is_control())
            || name
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return invalid("display image source must be a safe relative name or HTTPS URL");
        }
        names.insert(name.to_owned());
    }
    Ok(names.into_iter().collect())
}

pub(super) fn bind_blocks(
    blocks: &mut [QuestionContentBlock],
    bindings: &[QuestionHtmlImageBinding],
) -> Result<(), PleQuestionJsonError> {
    for block in blocks {
        if let QuestionContentBlock::Html { html, images } = block {
            *images = bindings_for(html, bindings)?;
        }
    }
    Ok(())
}

pub(super) fn bindings_for(
    html: &str,
    bindings: &[QuestionHtmlImageBinding],
) -> Result<Vec<QuestionHtmlImageBinding>, PleQuestionJsonError> {
    let mut by_name = BTreeMap::new();
    for binding in bindings {
        if by_name
            .insert(binding.source_relative_name.as_str(), binding)
            .is_some()
            || binding.question_image_asset_tuple.checksum.len() != 64
            || !binding
                .question_image_asset_tuple
                .checksum
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return invalid("HTML image bindings require unique names and SHA-256 checksums");
        }
    }
    relative_image_names(html)?
        .into_iter()
        .map(|name| {
            by_name
                .get(name.as_str())
                .map(|binding| (*binding).clone())
                .ok_or_else(|| {
                    PleQuestionJsonError::InvalidDocument(format!(
                        "display image has no supplied asset: {name}"
                    ))
                })
        })
        .collect()
}

pub(super) fn html_blocks(
    html: &str,
    bindings: Option<&[QuestionHtmlImageBinding]>,
) -> Result<Vec<QuestionContentBlock>, PleQuestionJsonError> {
    Ok(vec![QuestionContentBlock::Html {
        html: html.to_owned(),
        images: bindings
            .map(|images| bindings_for(html, images))
            .transpose()?
            .unwrap_or_default(),
    }])
}

pub(super) fn bind_response(
    response: &mut QuestionResponseFormat,
    bindings: &[QuestionHtmlImageBinding],
) -> Result<(), PleQuestionJsonError> {
    match response {
        QuestionResponseFormat::MultipleChoice { choices, .. } => {
            for choice in choices {
                bind_blocks(&mut choice.body, bindings)?;
            }
        }
        QuestionResponseFormat::MultiBlank { blanks } => {
            for blank in blanks {
                bind_blocks(&mut blank.label, bindings)?;
            }
        }
        QuestionResponseFormat::Matching { prompts, choices } => {
            for prompt in prompts {
                bind_blocks(&mut prompt.body, bindings)?;
            }
            for choice in choices {
                bind_blocks(&mut choice.body, bindings)?;
            }
        }
        QuestionResponseFormat::Ordering { items } => {
            for item in items {
                bind_blocks(&mut item.body, bindings)?;
            }
        }
        QuestionResponseFormat::Hotspot { regions, .. } => {
            for region in regions {
                bind_blocks(&mut region.label, bindings)?;
            }
        }
        _ => {}
    }
    Ok(())
}
