//! The question envelope: the payload a client receives (WP-C1).
//!
//! The envelope is what MOD-UI-RENDER maps to components, so every block kind
//! it can contain is enumerated here. A closed set means the renderer's match
//! is exhaustive, and adding a block kind makes the compiler point at the
//! renderer.
//!
//! The envelope carries prompt content and response shape. Answer keys and
//! grading material stay in `crates/grading`; an M3 gate inspects a browser
//! network trace to confirm it.

use serde::{Deserialize, Serialize};

use crate::QuestionVersionReference;
use crate::generation::Seed;
use crate::identity::AssetId;
use crate::response::ResponseDefinition;

/// A reference to a stored asset.
///
/// The checksum travels with the reference so a client can verify that the
/// bytes it received are the bytes the question was authored against, which is
/// what makes a cached render trustworthy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    /// Identifier of the stored object.
    pub asset: AssetId,
    /// Hex-encoded checksum computed when the asset was written.
    pub checksum: String,
}

/// One renderable piece of a question prompt.
///
/// Each variant that carries visual content also carries text describing it.
/// That text is required rather than optional: a question whose figure has no
/// description is unusable with a screen reader, and MOD-UI-RENDER surfaces a
/// missing description as an authoring error rather than rendering a gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ContentBlock {
    /// Prose, in a restricted Markdown subset that the renderer sanitizes.
    Text {
        /// Markdown source.
        markdown: String,
    },
    /// A mathematical expression.
    Math {
        /// LaTeX source.
        latex: String,
        /// Spoken-form description for assistive technology.
        description: String,
    },
    /// An image or figure.
    Image {
        /// The stored asset.
        asset: AssetRef,
        /// Description of what the image conveys.
        description: String,
    },
    /// A code listing.
    Code {
        /// Language name for highlighting, for example `python`.
        language: String,
        /// The listing itself.
        source: String,
    },
    /// A data table.
    Table {
        /// Column headings, left to right.
        headers: Vec<String>,
        /// Rows, each holding one cell per heading.
        rows: Vec<Vec<String>>,
        /// Description of what the table shows.
        description: String,
    },
}

/// One generated variant of a question, ready to render.
///
/// This is the unit the render cache stores and the reproducibility record
/// describes. It is keyed by `(Question Version Reference, seed)`: the same pair produces the same
/// envelope on every machine, which is what lets a repeat request be served
/// from cache and lets a grade be re-derived years later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionEnvelope {
    /// Exact immutable Question Version that produced this envelope.
    pub question_version: QuestionVersionReference,
    /// The seed that produced this variant.
    pub seed: Seed,
    /// A bounded student-facing title from published metadata or a safe imported
    /// provider label. This deliberately excludes authored source and grading
    /// material while letting the student identify the issued question.
    pub title: String,
    /// The prompt, in render order.
    pub prompt: Vec<ContentBlock>,
    /// The shape of response this variant expects.
    pub response: ResponseDefinition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_blocks_carry_their_description() {
        let block = ContentBlock::Math {
            latex: r"\frac{1}{2}".to_string(),
            description: "one half".to_string(),
        };
        let json = serde_json::to_string(&block).expect("serialization should succeed");
        assert!(json.contains("one half"));
    }

    #[test]
    fn blocks_serialize_with_a_discriminant() {
        let block = ContentBlock::Text {
            markdown: "Balance the equation.".to_string(),
        };
        let json = serde_json::to_string(&block).expect("serialization should succeed");
        assert!(json.starts_with(r#"{"kind":"text""#));
    }
}
