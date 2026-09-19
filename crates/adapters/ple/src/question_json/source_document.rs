//! Strict source shapes for all supported PLE Question JSON Question Types.

mod author_script;
#[path = "source_compile.rs"]
mod source_compile;

use source_compile::{
    compile_response, validate_answers, validate_blanks, validate_choice_question,
    validate_hotspot, validate_matching, validate_numeric, validate_ordering,
};

use std::collections::HashSet;

use question_model::answer::{NumericResponseTolerance, TextResponseMatchRule};
use question_model::question_citation::QuestionCitation;
use question_model::question_license::QuestionLicense;
use question_model::response::QuestionType;
use question_model::{QuestionAssetTuple, QuestionHint, QuestionMetadata};
use serde::{Deserialize, Serialize};
use url::Url;

use author_script::{PleQuestionJsonAuthorScript, compile_author_content, validate_author_script};

use super::{
    CompiledPleQuestionJson, MAX_PROMPT_CHARS, MAX_TAG_CHARS, PLE_QUESTION_JSON_FORMAT_NAME,
    PleQuestionJsonChoice, PleQuestionJsonError, PleQuestionJsonOutcomeFeedback,
    PleQuestionJsonPresentation, PleQuestionJsonPrivateGrading, invalid, markdown_blocks,
    validate_bounded_text, validate_markdown, validate_metadata_text, validate_optional_feedback,
    validate_optional_hint,
};

const MAX_BLANKS: usize = 50;
const MAX_TEXT_RESPONSE_CHARS: u32 = 16_384;
const MAX_EXTERNAL_RESOURCES: usize = 100;
const MAX_EXTERNAL_RESOURCE_URL_CHARS: usize = 4_096;

/// Common metadata outside a closed, type-specific response object.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PleQuestionJsonDocumentBody {
    format: String,
    question_title: String,
    question_description: String,
    prompt: String,
    response: PleQuestionJsonResponse,
    #[serde(default)]
    feedback: PleQuestionJsonOutcomeFeedback,
    #[serde(default)]
    question_hint: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    question_license: Option<QuestionLicense>,
    #[serde(default)]
    question_citation: Option<QuestionCitation>,
    /// Complete, author-declared inventory of every remote URL used by this
    /// Question. It is source metadata only: this document neither fetches
    /// nor grants a browser permission for a recorded resource.
    #[serde(default)]
    external_resources: Vec<PleQuestionJsonExternalResource>,
    /// Untrusted browser-only behavior declared by an author.  This is source
    /// metadata, not a PLE execution capability: the later browser boundary
    /// receives it only through an isolated author-content runtime.
    #[serde(default)]
    author_script: Option<PleQuestionJsonAuthorScript>,
    language: String,
}

/// One remote resource declared by a native Question author.
///
/// The closed kind set makes links, images, scripts, stylesheets, and
/// miscellaneous resources reviewable without making a new execution or
/// upload surface. Later browser-policy steps decide which recorded entries
/// may load and how they are served.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonExternalResource {
    url: String,
    kind: PleQuestionJsonExternalResourceKind,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum PleQuestionJsonExternalResourceKind {
    Link,
    Image,
    Script,
    Stylesheet,
    Other,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PleQuestionJsonResponse {
    SingleChoice {
        choices: Vec<PleQuestionJsonChoice>,
        correct_choice: String,
        #[serde(default)]
        randomize_choices: bool,
    },
    MultipleAnswer {
        choices: Vec<PleQuestionJsonChoice>,
        correct_choices: Vec<String>,
        #[serde(default)]
        randomize_choices: bool,
    },
    FillIn {
        answers: Vec<String>,
        match_mode: PleQuestionJsonTextResponseMatchRule,
        max_length: u32,
    },
    MultiFillIn {
        blanks: Vec<PleQuestionJsonBlank>,
    },
    Numeric {
        answer: f64,
        tolerance: PleQuestionJsonNumericResponseTolerance,
        #[serde(default)]
        unit: Option<String>,
    },
    Matching {
        prompts: Vec<PleQuestionJsonMatchingPrompt>,
        choices: Vec<PleQuestionJsonMatchingChoice>,
        matches: Vec<PleQuestionJsonMatch>,
    },
    Ordering {
        items: Vec<PleQuestionJsonOrderingItem>,
        correct_order: Vec<String>,
    },
    Hotspot {
        surface: PleQuestionJsonHotspotSurface,
        regions: Vec<PleQuestionJsonHotspotRegion>,
        correct_regions: Vec<String>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum PleQuestionJsonTextResponseMatchRule {
    Exact,
    CaseInsensitive,
    Normalized,
}

impl From<PleQuestionJsonTextResponseMatchRule> for TextResponseMatchRule {
    fn from(value: PleQuestionJsonTextResponseMatchRule) -> Self {
        match value {
            PleQuestionJsonTextResponseMatchRule::Exact => Self::Exact,
            PleQuestionJsonTextResponseMatchRule::CaseInsensitive => Self::CaseInsensitive,
            PleQuestionJsonTextResponseMatchRule::Normalized => Self::Normalized,
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum PleQuestionJsonNumericResponseTolerance {
    Exact,
    Absolute { epsilon: f64 },
    Relative { fraction: f64 },
    SignificantFigures { digits: u8 },
}

impl From<&PleQuestionJsonNumericResponseTolerance> for NumericResponseTolerance {
    fn from(value: &PleQuestionJsonNumericResponseTolerance) -> Self {
        match value {
            PleQuestionJsonNumericResponseTolerance::Exact => Self::Exact,
            PleQuestionJsonNumericResponseTolerance::Absolute { epsilon } => {
                Self::Absolute { epsilon: *epsilon }
            }
            PleQuestionJsonNumericResponseTolerance::Relative { fraction } => Self::Relative {
                fraction: *fraction,
            },
            PleQuestionJsonNumericResponseTolerance::SignificantFigures { digits } => {
                Self::SignificantFigures { digits: *digits }
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonMatchingPrompt {
    id: String,
    text: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonMatchingChoice {
    id: String,
    text: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonOrderingItem {
    id: String,
    text: String,
}

trait PleQuestionJsonResponseMember {
    fn id(&self) -> &str;
    fn text(&self) -> &str;
}

macro_rules! response_member {
    ($type:ident) => {
        impl PleQuestionJsonResponseMember for $type {
            fn id(&self) -> &str {
                &self.id
            }

            fn text(&self) -> &str {
                &self.text
            }
        }
    };
}

response_member!(PleQuestionJsonMatchingPrompt);
response_member!(PleQuestionJsonMatchingChoice);
response_member!(PleQuestionJsonOrderingItem);

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonBlank {
    id: String,
    label: String,
    answers: Vec<String>,
    match_mode: PleQuestionJsonTextResponseMatchRule,
    max_length: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonMatch {
    prompt: String,
    choice: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonHotspotSurface {
    question_asset: String,
    checksum: String,
    description: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PleQuestionJsonHotspotRegion {
    id: String,
    label: String,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
}

impl PleQuestionJsonDocumentBody {
    pub(super) fn has_external_image_resource(&self) -> bool {
        self.external_resources
            .iter()
            .any(|resource| resource.kind == PleQuestionJsonExternalResourceKind::Image)
    }

    pub(super) fn with_hotspot_surface_asset(
        &self,
        question_asset: QuestionAssetTuple,
    ) -> Result<Self, PleQuestionJsonError> {
        let mut published = self.clone();
        let PleQuestionJsonResponse::Hotspot { surface, .. } = &mut published.response else {
            return invalid("PLE Question JSON source has no hotspot surface to retarget");
        };
        surface.question_asset = question_asset.question_asset.to_string();
        surface.checksum = question_asset.checksum;
        published.validate()?;
        Ok(published)
    }

    pub(super) fn imported_single_choice(
        question_title: String,
        question_description: String,
        prompt: String,
        choices: Vec<PleQuestionJsonChoice>,
        correct_choice: String,
    ) -> Self {
        Self {
            format: PLE_QUESTION_JSON_FORMAT_NAME.to_string(),
            question_title,
            question_description,
            prompt,
            response: PleQuestionJsonResponse::SingleChoice {
                choices,
                correct_choice,
                randomize_choices: false,
            },
            feedback: PleQuestionJsonOutcomeFeedback::default(),
            question_hint: None,
            tags: Vec::new(),
            question_license: None,
            question_citation: None,
            external_resources: Vec::new(),
            author_script: None,
            language: "en-US".to_string(),
        }
    }

    pub(super) fn validate(&self) -> Result<(), PleQuestionJsonError> {
        if self.format != PLE_QUESTION_JSON_FORMAT_NAME {
            return Err(PleQuestionJsonError::UnsupportedFormat);
        }
        question_model::validate_question_title(&self.question_title)
            .map_err(PleQuestionJsonError::InvalidQuestionTitle)?;
        if let Err(error) =
            question_model::validate_question_description(&self.question_description)
        {
            return invalid(&error.to_string());
        }
        validate_markdown("prompt", &self.prompt, MAX_PROMPT_CHARS)?;
        validate_optional_feedback(self.feedback.correct.as_deref())?;
        validate_optional_feedback(self.feedback.incorrect.as_deref())?;
        validate_optional_hint(self.question_hint.as_deref())?;
        validate_metadata_text("language", &self.language)?;
        for tag in &self.tags {
            validate_bounded_text("tag", tag, MAX_TAG_CHARS)?;
        }
        validate_external_resources(&self.external_resources)?;
        validate_author_script(self.author_script.as_ref())?;
        self.validate_response()
    }

    fn validate_response(&self) -> Result<(), PleQuestionJsonError> {
        match &self.response {
            PleQuestionJsonResponse::SingleChoice {
                choices,
                correct_choice,
                ..
            } => validate_choice_question(choices, std::slice::from_ref(correct_choice), true),
            PleQuestionJsonResponse::MultipleAnswer {
                choices,
                correct_choices,
                ..
            } => validate_choice_question(choices, correct_choices, false),
            PleQuestionJsonResponse::FillIn {
                answers,
                max_length,
                ..
            } => validate_answers(answers, *max_length),
            PleQuestionJsonResponse::MultiFillIn { blanks } => validate_blanks(blanks),
            PleQuestionJsonResponse::Numeric {
                answer,
                tolerance,
                unit,
            } => validate_numeric(*answer, tolerance, unit.as_deref()),
            PleQuestionJsonResponse::Matching {
                prompts,
                choices,
                matches,
            } => validate_matching(prompts, choices, matches),
            PleQuestionJsonResponse::Ordering {
                items,
                correct_order,
            } => validate_ordering(items, correct_order),
            PleQuestionJsonResponse::Hotspot {
                surface,
                regions,
                correct_regions,
            } => validate_hotspot(surface, regions, correct_regions),
        }
    }

    pub(super) fn compile(&self) -> Result<CompiledPleQuestionJson, PleQuestionJsonError> {
        self.validate()?;
        let question_type = question_type_for(&self.response);
        let (response, native_choice_order, answer_key, choice_feedback, prompt_suffix) =
            compile_response(&self.response)?;
        let mut prompt = markdown_blocks(&self.prompt);
        prompt.extend(prompt_suffix);
        let source_checksum = super::sha256_hex(
            &serde_json::to_vec(self)
                .map_err(|error| PleQuestionJsonError::Encoding(error.to_string()))?,
        );
        let private = PleQuestionJsonPrivateGrading::new_with_key(
            source_checksum,
            question_type,
            &response,
            answer_key,
            choice_feedback,
            self.feedback.correct.clone(),
            self.feedback.incorrect.clone(),
        )?;
        let question_hint = self
            .question_hint
            .as_deref()
            .map(markdown_blocks)
            .and_then(QuestionHint::new);
        Ok(CompiledPleQuestionJson {
            presentation: PleQuestionJsonPresentation {
                metadata: QuestionMetadata {
                    question_title: self.question_title.clone(),
                    question_description: self.question_description.clone(),
                    tags: self
                        .tags
                        .iter()
                        .cloned()
                        .map(question_model::Tag::new)
                        .collect(),
                    question_license: self.question_license.clone(),
                    question_citation: self.question_citation.clone(),
                    language: self.language.clone(),
                },
                prompt,
                response,
                question_type,
                native_choice_order,
            },
            private,
            question_hint,
            author_content: compile_author_content(self.author_script.as_ref())?,
        })
    }
}

fn validate_external_resources(
    resources: &[PleQuestionJsonExternalResource],
) -> Result<(), PleQuestionJsonError> {
    if resources.len() > MAX_EXTERNAL_RESOURCES {
        return invalid("external resource count is outside the supported range");
    }
    let mut unique = HashSet::new();
    for resource in resources {
        validate_external_resource_url(&resource.url)?;
        if !unique.insert(resource.url.as_str()) {
            return invalid("external resource URLs must be unique");
        }
    }
    Ok(())
}

fn validate_external_resource_url(value: &str) -> Result<(), PleQuestionJsonError> {
    if value.chars().count() > MAX_EXTERNAL_RESOURCE_URL_CHARS
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
        || !has_valid_percent_escapes(value)
    {
        return invalid("external resource URL must be bounded printable text");
    }

    let parsed = Url::parse(value).map_err(|_| {
        PleQuestionJsonError::InvalidDocument("external resource URL is malformed".to_string())
    })?;
    if parsed.scheme() != "https" || parsed.host().is_none() {
        return invalid("external resource URL must be an absolute HTTPS URL");
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return invalid("external resource URL must not contain user information");
    }
    Ok(())
}

fn has_valid_percent_escapes(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

fn question_type_for(response: &PleQuestionJsonResponse) -> QuestionType {
    match response {
        PleQuestionJsonResponse::SingleChoice { .. } => QuestionType::MultipleChoice,
        PleQuestionJsonResponse::MultipleAnswer { .. } => QuestionType::MultipleAnswer,
        PleQuestionJsonResponse::FillIn { .. } => QuestionType::FillInBlank,
        PleQuestionJsonResponse::MultiFillIn { .. } => QuestionType::MultipleFillInBlank,
        PleQuestionJsonResponse::Numeric { .. } => QuestionType::Numeric,
        PleQuestionJsonResponse::Matching { .. } => QuestionType::Matching,
        PleQuestionJsonResponse::Ordering { .. } => QuestionType::Ordering,
        PleQuestionJsonResponse::Hotspot { .. } => QuestionType::Hotspot,
    }
}
