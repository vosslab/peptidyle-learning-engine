//! Reproducible Question Variation recipes and their answer-free presentation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const AUTHOR_CONTENT_DIGEST_DOMAIN: &[u8] = b"ple:author-content:v1\0";

use crate::generation::{QuestionReproduction, QuestionSeed};
use crate::question_content::QuestionContentBlock;
use crate::{QuestionResponseFormat, QuestionRevisionTuple};

/// Closed reviewed runtime libraries available to an isolated author-content
/// document. This deliberately is not a URL or package reference.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthorContentLibraryId {
    Rdkit,
}

/// Immutable answer-free author content retained below the generic browser
/// presentation boundary.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorContentPresentation {
    source: String,
    library_ids: Vec<AuthorContentLibraryId>,
}

impl AuthorContentPresentation {
    pub const MAX_SOURCE_CHARS: usize = 65_536;
    pub const MAX_LIBRARY_IDS: usize = 16;

    pub fn new(
        source: String,
        library_ids: Vec<AuthorContentLibraryId>,
    ) -> Result<Self, &'static str> {
        if source.trim().is_empty() || source.chars().count() > Self::MAX_SOURCE_CHARS {
            return Err("author content source is invalid");
        }
        if library_ids.len() > Self::MAX_LIBRARY_IDS {
            return Err("author content library IDs are invalid");
        }
        if library_ids
            .iter()
            .filter(|id| matches!(id, AuthorContentLibraryId::Rdkit))
            .count()
            > 1
        {
            return Err("author content library IDs are invalid");
        }
        let mut library_ids = library_ids;
        library_ids.sort_by_key(|library_id| match library_id {
            AuthorContentLibraryId::Rdkit => 0_u8,
        });
        Ok(Self {
            source,
            library_ids,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn library_ids(&self) -> &[AuthorContentLibraryId] {
        &self.library_ids
    }

    /// Canonical public binding for this otherwise private descriptor. The
    /// digest carries no source bytes and lets the generic presentation token
    /// remain browser-reproducible.
    pub fn digest(&self) -> String {
        let mut bytes = AUTHOR_CONTENT_DIGEST_DOMAIN.to_vec();
        let source = self.source.as_bytes();
        bytes.extend_from_slice(&(source.len() as u32).to_be_bytes());
        bytes.extend_from_slice(source);
        bytes.extend_from_slice(&(self.library_ids.len() as u32).to_be_bytes());
        for library_id in &self.library_ids {
            bytes.push(match library_id {
                AuthorContentLibraryId::Rdkit => 0,
            });
        }
        let mut digest = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            use std::fmt::Write as _;
            write!(&mut digest, "{byte:02x}").expect("writing to String cannot fail");
        }
        digest
    }
}

/// Internal presentation policy for native choice Questions.
///
/// This is deliberately skipped from the serialized Question Variation and
/// public response contracts. PLE Question JSON is currently the only source
/// that selects nonce-randomized choice order; other Question Backends retain
/// their own presentation behavior.
#[doc(hidden)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum NativeChoiceOrder {
    /// Preserve authored choice order.
    #[default]
    Fixed,
    /// Derive the issued order from the durable presentation nonce and choice IDs.
    NonceRandomized,
}

/// The reproducible state for one exact Question Revision.
///
/// Static sources retain no invented seed. Seeded backends retain their seed
/// and generated-parameter integrity evidence together.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVariation {
    /// Exact immutable Question Revision that produced this presentation.
    pub question_revision_tuple: QuestionRevisionTuple,
    /// Explicit static or seeded reproduction facts for this variation.
    pub reproduction: QuestionReproduction,
}

impl QuestionVariation {
    /// Records the exact facts that reproduce an issued Question Variation.
    pub fn from_question_revision_and_reproduction(
        question_revision_tuple: QuestionRevisionTuple,
        reproduction: QuestionReproduction,
    ) -> Self {
        Self {
            question_revision_tuple,
            reproduction,
        }
    }

    /// Records a seeded generated variation.
    pub fn from_question_revision_and_question_seed(
        question_revision_tuple: QuestionRevisionTuple,
        question_seed: QuestionSeed,
        generated_parameter_sha256: String,
    ) -> Self {
        Self::from_question_revision_and_reproduction(
            question_revision_tuple,
            QuestionReproduction::Seeded {
                question_seed,
                generated_parameter_sha256,
            },
        )
    }
}

/// Server-held answer-free Question Presentation derived from a Question Variation.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionVariationPresentation {
    /// The exact reproducible variation this presentation renders.
    pub variation: QuestionVariation,
    /// A bounded student-facing Question Title from published metadata or a safe imported
    /// Question Backend label. This deliberately excludes Question Source, Answer Key,
    /// and Question Grading Input while letting the student identify the issued Question.
    pub question_title: String,
    /// The prompt, in render order.
    pub prompt: Vec<QuestionContentBlock>,
    /// The shape of response this variant expects.
    pub response: QuestionResponseFormat,
    /// Server-only native choice-order policy; never emitted in public contracts.
    #[serde(skip)]
    pub native_choice_order: NativeChoiceOrder,
    /// Isolated author-content evidence. This never enters `QuestionPresentation`.
    #[serde(skip)]
    pub author_content: Option<AuthorContentPresentation>,
}
