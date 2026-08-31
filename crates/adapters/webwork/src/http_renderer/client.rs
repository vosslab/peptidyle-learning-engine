//! Private HTTP client for the standalone PG renderer.
//!
//! The upstream endpoint is deliberately treated as an untrusted private
//! service: it receives only trusted immutable source, seed, display policy,
//! and a server-resolved submitted answer. PLE translates its form/JSON
//! dialect into an answer-free question envelope.

#[path = "html_projection.rs"]
mod html_projection;
use html_projection::*;
#[path = "matching_projection.rs"]
mod matching_projection;
use matching_projection::*;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
#[cfg(test)]
use grading::QuestionGradingOutcome;
#[cfg(test)]
use question_model::GradingResult;
use question_model::answer::ResponseSelectionRule;
use question_model::envelope::ContentBlock;
use question_model::response::{ResponseItemReference, ChoiceOption, QuestionResponseFormat};
use question_model::{QuestionPresentation, StudentResponse};
use reqwest::header::{ACCEPT, CONTENT_TYPE, LOCATION};
use reqwest::{Client, StatusCode, Url};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use super::response_shape::RESPONSE_KEYS;

use crate::renderer_contract::{
    GradeRequest, RenderRequest, RenderedWebworkQuestion, RendererFailure, RendererIdentity,
    UpstreamControlV1, WebworkRenderer, WebworkReplayMappingV1,
};

const JSON_MEDIA_TYPE: &str = "application/json";
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1_048_576;
const MAX_PG_SOURCE_BYTES: usize = 262_144;
const MAX_PG_PATH_BYTES: usize = 1_024;
// Reviewed PG questions legitimately produce answer-state JWTs above 64 KiB.
// No token can exceed the client's already bounded default response body.
const MAX_PRIVATE_JWT_BYTES: usize = DEFAULT_MAX_RESPONSE_BYTES;
const MAX_RADIO_CHOICES: usize = 32;
const MAX_RADIO_FIELD_BYTES: usize = 128;
const MAX_RADIO_VALUE_BYTES: usize = 512;
const MAX_RADIO_LABEL_CHARS: usize = 4_096;
const MAX_PROMPT_CHARS: usize = 16_384;
const MAX_HTML_TOKENS: usize = 8_192;
const MAX_HTML_NESTING: usize = 64;

/// Configuration rejected before a private renderer request is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererConfigError {
    InvalidBaseUri,
    InvalidLimits,
    MissingRendererIdentity,
}

impl std::fmt::Display for RendererConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidBaseUri => {
                "renderer base URI must be absolute http(s), query-free, and fragment-free"
            }
            Self::InvalidLimits => "renderer deadlines and response limit must be positive",
            Self::MissingRendererIdentity => "renderer identity must be configured",
        })
    }
}
impl std::error::Error for RendererConfigError {}

/// Server-owned standalone renderer endpoint, identity, and resource limits.
#[derive(Clone)]
pub struct HttpWebworkRendererConfig {
    base_uri: Url,
    deadline: Duration,
    max_response_bytes: usize,
    expected_renderer: RendererIdentity,
}

impl std::fmt::Debug for HttpWebworkRendererConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpWebworkRendererConfig")
            .field("base_uri", &self.base_uri.as_str())
            .field("deadline", &self.deadline)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("expected_renderer", &self.expected_renderer)
            .finish()
    }
}

impl HttpWebworkRendererConfig {
    /// Builds a private standalone-renderer configuration. The base is the
    /// service origin root and may never carry a credential or token.
    pub fn new(
        base_uri: &str,
        deadline: Duration,
        max_response_bytes: usize,
        expected_renderer: RendererIdentity,
    ) -> Result<Self, RendererConfigError> {
        let base_uri = Url::parse(base_uri).map_err(|_| RendererConfigError::InvalidBaseUri)?;
        if !matches!(base_uri.scheme(), "http" | "https")
            || base_uri.host_str().is_none()
            || !base_uri.username().is_empty()
            || base_uri.password().is_some()
            || base_uri.query().is_some()
            || base_uri.fragment().is_some()
            || base_uri.path() != "/"
        {
            return Err(RendererConfigError::InvalidBaseUri);
        }
        if deadline.is_zero() || max_response_bytes == 0 {
            return Err(RendererConfigError::InvalidLimits);
        }
        if expected_renderer.id.trim().is_empty() || expected_renderer.version.trim().is_empty() {
            return Err(RendererConfigError::MissingRendererIdentity);
        }
        Ok(Self {
            base_uri,
            deadline,
            max_response_bytes,
            expected_renderer,
        })
    }

    pub fn with_default_response_limit(
        base_uri: &str,
        deadline: Duration,
        expected_renderer: RendererIdentity,
    ) -> Result<Self, RendererConfigError> {
        Self::new(
            base_uri,
            deadline,
            DEFAULT_MAX_RESPONSE_BYTES,
            expected_renderer,
        )
    }
}

#[derive(Clone)]
pub struct HttpWebworkRenderer {
    client: Client,
    settings: HttpWebworkRendererConfig,
}

impl HttpWebworkRenderer {
    pub fn new(settings: HttpWebworkRendererConfig) -> Result<Self, RendererConfigError> {
        let client = Client::builder()
            .connect_timeout(settings.deadline)
            .timeout(settings.deadline)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| RendererConfigError::InvalidLimits)?;
        Ok(Self { client, settings })
    }

    async fn rpc(&self, mut fields: BTreeMap<String, String>) -> Result<Value, RendererFailure> {
        fields.insert("_format".into(), "json".into());
        fields.insert("outputFormat".into(), "default".into());
        let target = self
            .settings
            .base_uri
            .join(crate::standalone_render_api::PATH)
            .map_err(|_| RendererFailure::InvalidOutput("renderer URI is invalid".into()))?;
        let response = self
            .client
            .post(target)
            .header(ACCEPT, JSON_MEDIA_TYPE)
            .header(CONTENT_TYPE, crate::standalone_render_api::FORM_MEDIA_TYPE)
            .form(&fields)
            .send()
            .await
            .map_err(map_request_error)?;
        if response.status().is_redirection() || response.headers().contains_key(LOCATION) {
            return Err(RendererFailure::InvalidOutput(
                "renderer redirected request".into(),
            ));
        }
        map_status(response.status())?;
        validate_content_type(&response)?;
        let bytes = read_bounded(response, self.settings.max_response_bytes).await?;
        let value = parse_json_without_duplicates(&bytes)?;
        if value.get("error").is_some() {
            return Err(RendererFailure::InvalidOutput(
                "renderer rejected trusted request".into(),
            ));
        }
        Ok(value)
    }

    async fn parsed_render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<ParsedRender, RendererFailure> {
        validate_render_request(request)?;
        let expected = ExpectedEcho::from_request(&self.settings, request);
        let value = self.rpc(super::protocol::render_fields(request)).await?;
        parse_render_rpc(value, expected, request, &self.settings.base_uri)
    }
}

#[async_trait]
impl WebworkRenderer for HttpWebworkRenderer {
    fn identity(&self) -> &RendererIdentity {
        &self.settings.expected_renderer
    }

    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        let parsed = self.parsed_render(request).await?;
        Ok(RenderedWebworkQuestion {
            envelope: parsed.envelope,
            html: parsed.html,
            renderer: self.settings.expected_renderer.clone(),
            replay: Some(parsed.replay),
        })
    }

    async fn grade(
        &self,
        request: GradeRequest<'_>,
    ) -> Result<grading::QuestionGradingOutcome, RendererFailure> {
        if !request.points_possible.is_finite() || request.points_possible <= 0.0 {
            return Err(RendererFailure::InvalidOutput(
                "WeBWorK supported questions require positive finite points".into(),
            ));
        }
        let mut fields = super::protocol::render_fields(RenderRequest {
            pg_source: request.pg_source,
            pg_path: request.pg_path,
            question_version: request.question_version,
            seed: request.seed,
        });
        match (request.response, request.replay) {
            (
                StudentResponse::MultipleChoice { selected },
                WebworkReplayMappingV1::SingleChoice { controls },
            ) if selected.len() == 1 => {
                let control = controls.get(&selected[0]).ok_or_else(|| {
                    RendererFailure::InvalidOutput(
                        "WeBWorK response selected an unknown choice".into(),
                    )
                })?;
                fields.insert(control.field.clone(), control.value.clone());
            }
            (
                StudentResponse::Matching { matches },
                WebworkReplayMappingV1::Matching { prompts },
            ) if matches.len() == prompts.len() => {
                for pair in matches {
                    let prompt = prompts.get(&pair.prompt).ok_or_else(|| {
                        RendererFailure::InvalidOutput(
                            "WeBWorK response named an unknown matching prompt".into(),
                        )
                    })?;
                    let value = prompt.choices.get(&pair.choice).ok_or_else(|| {
                        RendererFailure::InvalidOutput(
                            "WeBWorK response named an unknown matching choice".into(),
                        )
                    })?;
                    if fields.insert(prompt.field.clone(), value.clone()).is_some() {
                        return Err(RendererFailure::InvalidOutput(
                            "WeBWorK response repeated a matching prompt".into(),
                        ));
                    }
                }
            }
            _ => {
                return Err(RendererFailure::InvalidOutput(
                    "WeBWorK response does not match its issued replay state".into(),
                ));
            }
        }
        fields.insert("submitAnswers".into(), "1".into());
        let response = self.rpc(fields).await?;
        let score = validate_grade_rpc(
            &response,
            &ExpectedEcho::from_request(
                &self.settings,
                RenderRequest {
                    pg_source: request.pg_source,
                    pg_path: request.pg_path,
                    question_version: request.question_version,
                    seed: request.seed,
                },
            ),
            &self.settings.base_uri,
        )?;
        super::grade::score(score, request.points_possible, request.partial_credit)
    }
}

fn validate_grade_rpc(
    value: &Value,
    expected: &ExpectedEcho,
    _service_base: &Url,
) -> Result<f64, RendererFailure> {
    let object = value
        .as_object()
        .ok_or_else(|| bad("renderer JSON is not an object"))?;
    validate_response_shape(object)?;
    reject_protected_values(object, expected)?;
    let jwt = object
        .get("JWT")
        .and_then(Value::as_object)
        .ok_or_else(|| bad("renderer omitted private JWT state"))?;
    let _ = validate_and_discard_jwt(jwt)?;
    object
        .get("problem_result")
        .and_then(Value::as_object)
        .and_then(|result| result.get("score"))
        .and_then(Value::as_f64)
        .filter(|score| score.is_finite() && (0.0..=1.0).contains(score))
        .map(|score| score * 100.0)
        .ok_or_else(|| {
            RendererFailure::InvalidOutput("renderer returned malformed normalized score".into())
        })
}

fn validate_render_request(request: RenderRequest<'_>) -> Result<(), RendererFailure> {
    if request.pg_source.is_empty() || request.pg_source.len() > MAX_PG_SOURCE_BYTES {
        return Err(bad("WeBWorK source exceeds the supported bound"));
    }
    if request.pg_path.is_empty()
        || request.pg_path.len() > MAX_PG_PATH_BYTES
        || request.pg_path.starts_with('/')
        || request.pg_path.contains('\\')
        || request.pg_path.contains('\0')
        || request
            .pg_path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(bad("WeBWorK source path is outside the supported contract"));
    }
    Ok(())
}

struct ParsedRender {
    envelope: QuestionPresentation,
    html: String,
    replay: WebworkReplayMappingV1,
}
#[derive(Debug)]
struct ExpectedEcho {
    source: String,
    file: String,
}
impl ExpectedEcho {
    fn from_request(_settings: &HttpWebworkRendererConfig, request: RenderRequest<'_>) -> Self {
        Self {
            source: base64::engine::general_purpose::STANDARD.encode(request.pg_source),
            file: request.pg_path.to_owned(),
        }
    }
}

fn parse_render_rpc(
    value: Value,
    expected: ExpectedEcho,
    request: RenderRequest<'_>,
    service_base: &Url,
) -> Result<ParsedRender, RendererFailure> {
    let object = value
        .as_object()
        .ok_or_else(|| bad("renderer JSON is not an object"))?;
    validate_response_shape(object)?;
    reject_protected_values(object, &expected)?;
    let jwt = object
        .get("JWT")
        .and_then(Value::as_object)
        .ok_or_else(|| bad("renderer omitted private JWT state"))?;
    let protected_html_values = validate_and_discard_jwt(jwt)?;
    let html = body_html(object, service_base)?;
    reject_protected_html(&html, &protected_html_values)?;
    let radio_error = match parse_single_radio_group(&html, &protected_html_values) {
        Ok(parsed_html) => return project_single_radio(parsed_html, request),
        Err(error) => error,
    };
    match parse_matching_group(&html, &protected_html_values) {
        Ok(parsed_html) => project_matching(parsed_html, request),
        Err(_) => Err(radio_error),
    }
}

fn project_single_radio(
    parsed_html: ParsedRadioHtml,
    request: RenderRequest<'_>,
) -> Result<ParsedRender, RendererFailure> {
    let prompt = parsed_html.prompt_text;
    if prompt.trim().is_empty() {
        return Err(bad("renderer prompt is empty"));
    }
    let mut choices = Vec::with_capacity(parsed_html.controls.len());
    let mut choice_fields = BTreeMap::new();
    for (index, control) in parsed_html.controls.into_iter().enumerate() {
        let id = opaque_choice_id(request, index)?;
        choices.push(ChoiceOption {
            id: id.clone(),
            body: vec![ContentBlock::Text {
                markdown: control.label,
            }],
        });
        choice_fields.insert(
            id,
            UpstreamControlV1 {
                field: control.name,
                value: control.value,
            },
        );
    }
    Ok(ParsedRender {
        envelope: QuestionPresentation {
            variation: question_model::QuestionVariation::static_variation(
                request.question_version.clone(),
                question_model::generation::Seed::new(request.seed),
            ),
            title: "WeBWorK question".into(),
            prompt: vec![ContentBlock::Text { markdown: prompt }],
            response: QuestionResponseFormat::MultipleChoice {
                choices,
                selection: ResponseSelectionRule::ExactlyOne,
            },
        },
        html: crate::sanitizer::sanitize_webwork_html(&parsed_html.prompt_html),
        replay: WebworkReplayMappingV1::SingleChoice {
            controls: choice_fields,
        },
    })
}

fn project_matching(
    parsed_html: ParsedMatchingHtml,
    request: RenderRequest<'_>,
) -> Result<ParsedRender, RendererFailure> {
    let mut prompts = Vec::with_capacity(parsed_html.prompts.len());
    let mut choices = Vec::with_capacity(parsed_html.choices.len());
    let mut replay_prompts = BTreeMap::new();
    let choice_ids: Vec<_> = parsed_html
        .choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            let id = opaque_item_id(request, 3, index)?;
            choices.push(ChoiceOption {
                id: id.clone(),
                body: vec![ContentBlock::Text {
                    markdown: choice.label.clone(),
                }],
            });
            Ok((id, choice.value.clone()))
        })
        .collect::<Result<_, RendererFailure>>()?;
    for (index, prompt) in parsed_html.prompts.into_iter().enumerate() {
        let id = opaque_item_id(request, 2, index)?;
        prompts.push(ChoiceOption {
            id: id.clone(),
            body: vec![ContentBlock::Text {
                markdown: prompt.label,
            }],
        });
        replay_prompts.insert(
            id,
            crate::renderer_contract::UpstreamMatchPromptV1 {
                field: prompt.field,
                choices: choice_ids.iter().cloned().collect(),
            },
        );
    }
    Ok(ParsedRender {
        envelope: QuestionPresentation {
            variation: question_model::QuestionVariation::static_variation(
                request.question_version.clone(),
                question_model::generation::Seed::new(request.seed),
            ),
            title: "WeBWorK question".into(),
            prompt: vec![ContentBlock::Text {
                markdown: parsed_html.prompt_text,
            }],
            response: QuestionResponseFormat::Matching { prompts, choices },
        },
        html: crate::sanitizer::sanitize_webwork_html(&parsed_html.prompt_html),
        replay: WebworkReplayMappingV1::Matching {
            prompts: replay_prompts,
        },
    })
}

fn validate_response_shape(object: &Map<String, Value>) -> Result<(), RendererFailure> {
    if object.len() != RESPONSE_KEYS.len()
        || RESPONSE_KEYS
            .iter()
            .any(|required| !object.contains_key(*required))
        || object
            .keys()
            .any(|key| !RESPONSE_KEYS.contains(&key.as_str()))
    {
        return Err(bad("renderer returned an unsupported response member"));
    }
    if object.get("renderedHTML").and_then(Value::as_str).is_none() {
        return Err(bad("renderer omitted rendered HTML"));
    }
    for key in [
        "JWT",
        "debug",
        "flags",
        "problem_result",
        "problem_state",
        "resources",
    ] {
        if object.get(key).and_then(Value::as_object).is_none() {
            return Err(bad("renderer response member has an unsupported type"));
        }
    }
    Ok(())
}

fn same_service_origin(url: &Url, service_base: &Url) -> bool {
    url.scheme() == service_base.scheme()
        && url.host_str() == service_base.host_str()
        && url.port_or_known_default() == service_base.port_or_known_default()
}

fn verify_site_url(value: &str, service_base: &Url) -> Result<(), RendererFailure> {
    let url = Url::parse(value).map_err(|_| bad("renderer returned malformed service URL"))?;
    if !same_service_origin(&url, service_base)
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(bad("renderer returned off-policy service URL"));
    }
    Ok(())
}

fn verify_form_action_url(value: &str, service_base: &Url) -> Result<(), RendererFailure> {
    let url = Url::parse(value).map_err(|_| bad("renderer returned malformed service URL"))?;
    let expected = service_base
        .join(crate::standalone_render_api::PATH)
        .map_err(|_| bad("renderer service URL is invalid"))?;
    if url.query().is_some() || url.fragment().is_some() || url != expected {
        return Err(bad("renderer returned off-policy service URL"));
    }
    Ok(())
}

fn parse_json_without_duplicates(bytes: &[u8]) -> Result<Value, RendererFailure> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = NoDuplicateJson::deserialize(&mut deserializer)
        .map_err(|_| bad("renderer returned malformed or duplicate JSON"))?
        .0;
    deserializer
        .end()
        .map_err(|_| bad("renderer returned malformed JSON"))?;
    Ok(value)
}

struct NoDuplicateJson(Value);
impl<'de> Deserialize<'de> for NoDuplicateJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NoDuplicateVisitor;
        impl<'de> Visitor<'de> for NoDuplicateVisitor {
            type Value = NoDuplicateJson;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a JSON value without duplicate object members")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(NoDuplicateJson(Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(NoDuplicateJson(Value::Number(value.into())))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(NoDuplicateJson(Value::Number(value.into())))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(Value::Number)
                    .map(NoDuplicateJson)
                    .ok_or_else(|| E::custom("non-finite number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(NoDuplicateJson(Value::String(value.to_owned())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(NoDuplicateJson(Value::String(value)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicateJson(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(NoDuplicateJson(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = access.next_element::<NoDuplicateJson>()? {
                    values.push(value.0);
                }
                Ok(NoDuplicateJson(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut map = Map::new();
                while let Some((key, value)) = access.next_entry::<String, NoDuplicateJson>()? {
                    if map.insert(key, value.0).is_some() {
                        return Err(de::Error::custom("duplicate object member"));
                    }
                }
                Ok(NoDuplicateJson(Value::Object(map)))
            }
        }
        deserializer.deserialize_any(NoDuplicateVisitor)
    }
}

fn reject_protected_values(
    object: &Map<String, Value>,
    expected: &ExpectedEcho,
) -> Result<(), RendererFailure> {
    let protected = [
        "passwd",
        "password",
        "problemSource",
        "rawProblemSource",
        "uriEncodedProblemSource",
        "PG_ANSWERS_HASH",
        "answers",
        "correct_answers",
        "correctAnswer",
    ];
    for (key, value) in object {
        if protected
            .iter()
            .any(|protected| key.eq_ignore_ascii_case(protected))
            || contains_protected(value, &protected)
        {
            return Err(bad("renderer response contained protected material"));
        }
    }
    if contains_expected_protected(object, expected) {
        return Err(bad("renderer response echoed trusted request material"));
    }
    Ok(())
}

fn contains_expected_protected(object: &Map<String, Value>, expected: &ExpectedEcho) -> bool {
    fn contains(value: &Value, protected: &[&str]) -> bool {
        match value {
            Value::String(value) => protected
                .iter()
                .any(|protected| !protected.is_empty() && value.contains(protected)),
            Value::Object(values) => values.values().any(|value| contains(value, protected)),
            Value::Array(values) => values.iter().any(|value| contains(value, protected)),
            _ => false,
        }
    }

    let protected = [expected.source.as_str(), expected.file.as_str()];
    object.values().any(|value| contains(value, &protected))
}

/// Private renderer JWTs must never appear in browser-projected body text or
/// attributes. Check this before
/// sanitizing: a sanitizer may remove the element while leaving an audit gap.
fn reject_protected_html(
    html: &str,
    protected_html_values: &BTreeSet<String>,
) -> Result<(), RendererFailure> {
    if protected_html_values
        .iter()
        .any(|protected| html.contains(protected))
    {
        return Err(bad("renderer HTML contained protected material"));
    }
    Ok(())
}
fn contains_protected(value: &Value, protected: &[&str]) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            protected.iter().any(|p| key.eq_ignore_ascii_case(p))
                || contains_protected(value, protected)
        }),
        Value::Array(items) => items.iter().any(|item| contains_protected(item, protected)),
        _ => false,
    }
}
fn validate_and_discard_jwt(jwt: &Map<String, Value>) -> Result<BTreeSet<String>, RendererFailure> {
    if jwt.len() != 3
        || ["problem", "session", "answer"]
            .iter()
            .any(|key| !jwt.contains_key(*key))
    {
        return Err(bad("renderer JWT state has an unsupported shape"));
    }
    let mut protected = BTreeSet::new();
    for key in ["problem", "session", "answer"] {
        let value = jwt
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= MAX_PRIVATE_JWT_BYTES
                    && value.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
                    })
                    && matches!(value.split('.').count(), 3 | 5)
                    && value.split('.').all(|part| !part.is_empty())
            })
            .ok_or_else(|| bad("renderer returned malformed private JWT state"))?;
        protected.insert(value.to_owned());
    }
    Ok(protected)
}
fn bad(message: &str) -> RendererFailure {
    RendererFailure::InvalidOutput(message.into())
}
fn map_request_error(error: reqwest::Error) -> RendererFailure {
    if error.is_timeout() {
        RendererFailure::TimedOut
    } else {
        RendererFailure::Unavailable
    }
}
fn map_status(status: StatusCode) -> Result<(), RendererFailure> {
    if status.is_success() {
        Ok(())
    } else if matches!(
        status,
        StatusCode::TOO_MANY_REQUESTS | StatusCode::PAYLOAD_TOO_LARGE
    ) {
        Err(RendererFailure::ResourceExhausted)
    } else if matches!(
        status,
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT
    ) {
        Err(RendererFailure::TimedOut)
    } else if status.is_server_error() {
        Err(RendererFailure::Unavailable)
    } else {
        Err(bad("renderer rejected trusted server request"))
    }
}
fn validate_content_type(response: &reqwest::Response) -> Result<(), RendererFailure> {
    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media| media.trim().eq_ignore_ascii_case(JSON_MEDIA_TYPE))
        })
        .then_some(())
        .ok_or_else(|| bad("renderer response was not JSON"))
}
async fn read_bounded(
    mut response: reqwest::Response,
    maximum: usize,
) -> Result<Vec<u8>, RendererFailure> {
    if response
        .content_length()
        .is_some_and(|length| length > maximum as u64)
    {
        return Err(RendererFailure::ResourceExhausted);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(map_request_error)? {
        if chunk.len() > maximum.saturating_sub(bytes.len()) {
            return Err(RendererFailure::ResourceExhausted);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
