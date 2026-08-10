//! Bounded client for the upstream WeBWorK `render_rpc` endpoint.
//!
//! The upstream endpoint is deliberately treated as an untrusted private
//! service: it receives only server-owned credentials and source, and PLE
//! translates its form/JSON dialect into an answer-free question envelope.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use grading::GradeOutcome;
use html5ever::tendril::StrTendril;
use html5ever::tokenizer::{
    BufferQueue, Tag, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};
use question_model::answer::SelectionCardinality;
use question_model::envelope::ContentBlock;
use question_model::response::{ChoiceId, ChoiceOption, ResponseDefinition};
use question_model::{AttemptResult, QuestionEnvelope, StudentResponse};
use reqwest::header::{CONTENT_TYPE, LOCATION};
use reqwest::{Client, StatusCode, Url};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::renderer_contract::{
    GradeRequest, RenderRequest, RenderedWebworkQuestion, RendererFailure, RendererIdentity,
    WebworkRenderer,
};

const JSON_MEDIA_TYPE: &str = "application/json";
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1_048_576;
const MAX_PG_SOURCE_BYTES: usize = 262_144;
const MAX_PG_PATH_BYTES: usize = 1_024;
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
    MissingCourseCredentials,
}

impl std::fmt::Display for RendererConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidBaseUri => {
                "renderer base URI must be absolute http(s), query-free, and fragment-free"
            }
            Self::InvalidLimits => "renderer deadlines and response limit must be positive",
            Self::MissingRendererIdentity => "renderer identity must be configured",
            Self::MissingCourseCredentials => {
                "WeBWorK course, user, and password must be configured"
            }
        })
    }
}
impl std::error::Error for RendererConfigError {}

/// Server-owned upstream WeBWorK credentials and resource limits.
#[derive(Clone)]
pub struct HttpWebworkRendererConfig {
    base_uri: Url,
    deadline: Duration,
    max_response_bytes: usize,
    expected_renderer: RendererIdentity,
    course_id: String,
    user: String,
    password: String,
}

impl std::fmt::Debug for HttpWebworkRendererConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpWebworkRendererConfig")
            .field("base_uri", &self.base_uri)
            .field("deadline", &self.deadline)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("expected_renderer", &self.expected_renderer)
            .field("course_id", &self.course_id)
            .field("user", &self.user)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

impl HttpWebworkRendererConfig {
    /// Builds a private `/render_rpc` configuration.  The base can be a host
    /// root or a WebWork application base, but may never carry a token.
    pub fn new(
        base_uri: &str,
        deadline: Duration,
        max_response_bytes: usize,
        expected_renderer: RendererIdentity,
        course_id: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, RendererConfigError> {
        let base_uri = Url::parse(base_uri).map_err(|_| RendererConfigError::InvalidBaseUri)?;
        if !matches!(base_uri.scheme(), "http" | "https")
            || base_uri.host_str().is_none()
            || !base_uri.username().is_empty()
            || base_uri.password().is_some()
            || base_uri.query().is_some()
            || base_uri.fragment().is_some()
            || !base_uri.path().ends_with('/')
            || base_uri.path() == "/"
        {
            return Err(RendererConfigError::InvalidBaseUri);
        }
        if deadline.is_zero() || max_response_bytes == 0 {
            return Err(RendererConfigError::InvalidLimits);
        }
        if expected_renderer.id.trim().is_empty() || expected_renderer.version.trim().is_empty() {
            return Err(RendererConfigError::MissingRendererIdentity);
        }
        if [course_id, user, password]
            .iter()
            .any(|value| value.trim().is_empty())
        {
            return Err(RendererConfigError::MissingCourseCredentials);
        }
        Ok(Self {
            base_uri,
            deadline,
            max_response_bytes,
            expected_renderer,
            course_id: course_id.to_owned(),
            user: user.to_owned(),
            password: password.to_owned(),
        })
    }

    pub fn with_default_response_limit(
        base_uri: &str,
        deadline: Duration,
        expected_renderer: RendererIdentity,
        course_id: &str,
        user: &str,
        password: &str,
    ) -> Result<Self, RendererConfigError> {
        Self::new(
            base_uri,
            deadline,
            DEFAULT_MAX_RESPONSE_BYTES,
            expected_renderer,
            course_id,
            user,
            password,
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
        fields.insert("courseID".into(), self.settings.course_id.clone());
        fields.insert("user".into(), self.settings.user.clone());
        fields.insert("passwd".into(), self.settings.password.clone());
        fields.insert("outputformat".into(), "json".into());
        let target = self
            .settings
            .base_uri
            .join(crate::shipped_render_rpc::PATH)
            .map_err(|_| RendererFailure::InvalidOutput("renderer URI is invalid".into()))?;
        let response = self
            .client
            .post(target)
            .header(CONTENT_TYPE, crate::shipped_render_rpc::FORM_MEDIA_TYPE)
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

    fn render_fields(&self, request: RenderRequest<'_>) -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "problemSource".into(),
                base64::engine::general_purpose::STANDARD.encode(request.pg_source),
            ),
            ("fileName".into(), request.pg_path.to_owned()),
            ("problemSeed".into(), request.seed.to_string()),
            ("displayMode".into(), "MathJax".into()),
            ("showSummary".into(), "0".into()),
            ("showHints".into(), "0".into()),
            ("showSolutions".into(), "0".into()),
            ("showPreviewButton".into(), "0".into()),
            ("showCheckAnswersButton".into(), "0".into()),
            ("showCorrectAnswersButton".into(), "0".into()),
            ("showFooter".into(), "0".into()),
        ])
    }

    async fn parsed_render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<ParsedRender, RendererFailure> {
        validate_render_request(request)?;
        let expected = ExpectedEcho::from_request(&self.settings, request);
        let value = self.rpc(self.render_fields(request)).await?;
        parse_render_rpc(value, expected, request, &self.settings.base_uri)
    }
}

#[async_trait]
impl WebworkRenderer for HttpWebworkRenderer {
    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        let parsed = self.parsed_render(request).await?;
        Ok(RenderedWebworkQuestion {
            envelope: parsed.envelope,
            html: parsed.html,
            renderer: self.settings.expected_renderer.clone(),
        })
    }

    async fn grade(&self, request: GradeRequest<'_>) -> Result<GradeOutcome, RendererFailure> {
        if !request.points_possible.is_finite() || request.points_possible <= 0.0 {
            return Err(RendererFailure::InvalidOutput(
                "WeBWorK supported questions require positive finite points".into(),
            ));
        }
        let rendered = self
            .parsed_render(RenderRequest {
                pg_source: request.pg_source,
                pg_path: request.pg_path,
                version: request.version,
                seed: request.seed,
            })
            .await?;
        let StudentResponse::MultipleChoice { selected } = request.response else {
            return Err(RendererFailure::InvalidOutput(
                "WeBWorK response is not single-choice".into(),
            ));
        };
        if selected.len() != 1 {
            return Err(RendererFailure::InvalidOutput(
                "WeBWorK requires exactly one selected choice".into(),
            ));
        }
        let selected = selected
            .first()
            .ok_or_else(|| RendererFailure::InvalidOutput("WeBWorK response is empty".into()))?;
        let (field, value) = rendered.choice_fields.get(selected).ok_or_else(|| {
            RendererFailure::InvalidOutput("WeBWorK response selected an unknown choice".into())
        })?;
        let mut fields = self.render_fields(RenderRequest {
            pg_source: request.pg_source,
            pg_path: request.pg_path,
            version: request.version,
            seed: request.seed,
        });
        fields.insert(field.clone(), value.clone());
        fields.insert("WWsubmit".into(), "1".into());
        let response = self.rpc(fields).await?;
        let _validated = parse_render_rpc(
            response.clone(),
            ExpectedEcho::from_request(
                &self.settings,
                RenderRequest {
                    pg_source: request.pg_source,
                    pg_path: request.pg_path,
                    version: request.version,
                    seed: request.seed,
                },
            ),
            RenderRequest {
                pg_source: request.pg_source,
                pg_path: request.pg_path,
                version: request.version,
                seed: request.seed,
            },
            &self.settings.base_uri,
        )?;
        let score = response
            .get("score")
            .and_then(Value::as_f64)
            .filter(|score| score.is_finite() && (0.0..=100.0).contains(score))
            .ok_or_else(|| {
                RendererFailure::InvalidOutput("renderer returned malformed score".into())
            })?;
        if score != 0.0 && score != 100.0 {
            return Err(RendererFailure::InvalidOutput(
                "renderer returned unsupported partial score".into(),
            ));
        }
        let normalized = score / 100.0;
        Ok(GradeOutcome::Graded(AttemptResult {
            correct: normalized == 1.0,
            points_earned: if normalized == 1.0 {
                request.points_possible
            } else {
                0.0
            },
            points_possible: request.points_possible,
        }))
    }
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
    if uuid::Uuid::parse_str(request.version).is_err() {
        return Err(bad("invalid immutable version"));
    }
    Ok(())
}

#[derive(Debug)]
struct ParsedRender {
    envelope: QuestionEnvelope,
    html: String,
    choice_fields: BTreeMap<ChoiceId, (String, String)>,
}
#[derive(Debug)]
struct ExpectedEcho {
    course_id: String,
    user: String,
    password: String,
    source: String,
    file: String,
    seed: String,
}
impl ExpectedEcho {
    fn from_request(settings: &HttpWebworkRendererConfig, request: RenderRequest<'_>) -> Self {
        Self {
            course_id: settings.course_id.clone(),
            user: settings.user.clone(),
            password: settings.password.clone(),
            source: base64::engine::general_purpose::STANDARD.encode(request.pg_source),
            file: request.pg_path.to_owned(),
            seed: request.seed.to_string(),
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
    validate_response_shape(object, service_base)?;
    reject_protected_values(object, &expected)?;
    let hidden = object
        .get("hidden_input_field")
        .and_then(Value::as_object)
        .ok_or_else(|| bad("renderer omitted hidden fields"))?;
    let protected_html_values = validate_and_discard_hidden(hidden, &expected)?;
    let html = body_html(object)?;
    reject_protected_html(&html, &protected_html_values)?;
    let parsed_html = parse_single_radio_group(&html, &protected_html_values)?;
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
        choice_fields.insert(id, (control.name, control.value));
    }
    Ok(ParsedRender {
        envelope: QuestionEnvelope {
            version: question_model::VersionId::from_uuid(
                uuid::Uuid::parse_str(request.version)
                    .map_err(|_| bad("invalid immutable version"))?,
            ),
            seed: question_model::generation::Seed::new(request.seed),
            title: "WeBWorK question".into(),
            prompt: vec![ContentBlock::Text { markdown: prompt }],
            response: ResponseDefinition::MultipleChoice {
                choices,
                selection: SelectionCardinality::ExactlyOne,
            },
        },
        html: crate::sanitizer::sanitize_webwork_html(&parsed_html.prompt_html),
        choice_fields,
    })
}

const RESPONSE_KEYS: &[&str] = &[
    "head_part001",
    "head_part010",
    "head_part300",
    "head_part400",
    "head_part999",
    "body_part001",
    "body_part100",
    "body_part300",
    "body_part500",
    "body_part530",
    "body_part550",
    "body_part590",
    "body_part650",
    "body_part700",
    "body_part999",
    "hidden_input_field",
    "score",
    "real_webwork_SITE_URL",
    "real_webwork_FORM_ACTION_URL",
    "internal_problem_lang_and_dir",
];

fn validate_response_shape(
    object: &Map<String, Value>,
    service_base: &Url,
) -> Result<(), RendererFailure> {
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
    for (key, value) in object {
        if key == "score" {
            if !value.is_number() || !value.as_f64().is_some_and(f64::is_finite) {
                return Err(bad("renderer score is not numeric"));
            }
        } else if key != "hidden_input_field" && !value.is_string() {
            return Err(bad("renderer response member is not text"));
        }
    }
    let site = object
        .get("real_webwork_SITE_URL")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("renderer omitted site URL"))?;
    verify_site_url(site, service_base)?;
    let action = object
        .get("real_webwork_FORM_ACTION_URL")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("renderer omitted form action URL"))?;
    verify_form_action_url(action, service_base)?;
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
        .join(crate::shipped_render_rpc::PATH)
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
        if key == "hidden_input_field" {
            continue;
        }
        if protected
            .iter()
            .any(|protected| key.eq_ignore_ascii_case(protected))
            || contains_protected(value, &protected)
        {
            return Err(bad("renderer response contained protected material"));
        }
    }
    let _ = expected;
    Ok(())
}

/// Credentials and source echoed by the upstream hidden map must never appear
/// in any browser-projected body text or attribute.  Check this before
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
fn validate_and_discard_hidden(
    hidden: &Map<String, Value>,
    expected: &ExpectedEcho,
) -> Result<BTreeSet<String>, RendererFailure> {
    const BOUNDED_DISCARDED: &[(&str, usize)] = &[
        ("key", 4096),
        ("sourceFilePath", 4096),
        ("problemUUID", 128),
        ("psvn", 128),
        ("theme", 128),
        ("language", 128),
        ("extraHeaderText", 2048),
    ];
    let expected_values = BTreeMap::from([
        ("courseID", expected.course_id.as_str()),
        ("user", expected.user.as_str()),
        ("passwd", expected.password.as_str()),
        ("problemSource", expected.source.as_str()),
        ("pathToProblemFile", expected.file.as_str()),
        ("problemSeed", expected.seed.as_str()),
        ("outputformat", "json"),
        ("displayMode", "MathJax"),
        ("showSummary", "0"),
        ("showHints", "0"),
        ("showSolutions", "0"),
        ("showPreviewButton", "0"),
        ("showCheckAnswersButton", "0"),
        ("showCorrectAnswersButton", "0"),
        ("showFooter", "0"),
    ]);
    let allowed: BTreeSet<_> = expected_values
        .keys()
        .chain(BOUNDED_DISCARDED.iter().map(|(name, _)| name))
        .copied()
        .collect();
    if hidden.len() != allowed.len() || hidden.keys().any(|name| !allowed.contains(name.as_str())) {
        return Err(bad(
            "renderer hidden fields do not match the official template",
        ));
    }
    let mut protected_html_values = BTreeSet::new();
    for (name, value) in hidden {
        let value = value
            .as_str()
            .ok_or_else(|| bad("renderer hidden value is not text"))?;
        if let Some(expected_value) = expected_values.get(name.as_str()) {
            if value != *expected_value {
                return Err(bad("renderer hidden request echo did not match"));
            }
        } else if let Some((_, maximum)) = BOUNDED_DISCARDED
            .iter()
            .find(|(allowed, _)| *allowed == name)
        {
            if value.len() > *maximum {
                return Err(bad("renderer hidden value exceeds bound"));
            }
        } else {
            return Err(bad("renderer returned unexpected hidden material"));
        }
        if matches!(
            name.as_str(),
            "key"
                | "sourceFilePath"
                | "problemSource"
                | "problemUUID"
                | "psvn"
                | "pathToProblemFile"
                | "courseID"
                | "user"
                | "passwd"
        ) && !value.is_empty()
        {
            protected_html_values.insert(value.to_owned());
        }
    }
    Ok(protected_html_values)
}
fn body_html(object: &Map<String, Value>) -> Result<String, RendererFailure> {
    object
        .get("body_part550")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| bad("renderer omitted question body"))
}
#[derive(Debug)]
struct Radio {
    name: String,
    value: String,
    label: String,
}
#[derive(Debug)]
struct ParsedRadioHtml {
    controls: Vec<Radio>,
    prompt_text: String,
    prompt_html: String,
}

/// Tokenize the renderer fragment with html5ever and accept only the exact PG
/// RadioButtons shape we ship: one container and direct, wrapping labels.
/// A tokenizer is intentionally used instead of an HTML DOM because browser
/// error recovery would turn malformed hostile markup into a different tree.
fn parse_single_radio_group(
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
                    let is_container = name == "div" && has_class(&tag, "radio-buttons-container");
                    if is_container {
                        if radio_container_seen || radio_container_depth.is_some() {
                            return Err(bad("renderer returned more than one radio group"));
                        }
                        radio_container_seen = true;
                        radio_container_depth = Some(stack.len() + 1);
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
                    } else if radio_container_depth.is_some() {
                        return Err(bad("radio group has unsupported nesting"));
                    } else {
                        append_start_tag(&mut prompt_html, &tag);
                    }
                    if name != "input" && !is_void_element(&name) {
                        if stack.len() >= MAX_HTML_NESTING {
                            return Err(bad("renderer markup exceeds nesting bound"));
                        }
                        stack.push(OpenElement { name, is_container });
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
                    } else if radio_container_depth.is_none() {
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

fn reject_protected_text(
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
}

#[derive(Debug, Default)]
struct ActiveLabel {
    radio: Option<Radio>,
    text: String,
}

fn tokenize_html(html: &str) -> Result<Vec<Token>, RendererFailure> {
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

fn validate_tag(tag: &Tag) -> Result<(), RendererFailure> {
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
    if tag.self_closing || attribute(tag, "type").as_deref() != Some("radio") {
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

fn attribute(tag: &Tag, name: &str) -> Option<String> {
    tag.attrs
        .iter()
        .find(|attribute| attribute.name.local.as_ref() == name)
        .map(|attribute| attribute.value.to_string())
}

fn required_bounded_attribute(
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

fn has_class(tag: &Tag, wanted: &str) -> bool {
    attribute(tag, "class").is_some_and(|classes| {
        classes
            .split_ascii_whitespace()
            .any(|class| class == wanted)
    })
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
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn push_bounded(target: &mut String, value: &str, maximum: usize) -> Result<(), RendererFailure> {
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

fn opaque_choice_id(
    request: RenderRequest<'_>,
    ordinal: usize,
) -> Result<ChoiceId, RendererFailure> {
    let mut hash = Sha256::new();
    hash.update(b"ple:webwork:choice:v1\0");
    let version =
        uuid::Uuid::parse_str(request.version).map_err(|_| bad("invalid immutable version"))?;
    hash.update(version.as_bytes());
    hash.update(request.seed.to_be_bytes());
    hash.update(0_u32.to_be_bytes());
    hash.update((ordinal as u32).to_be_bytes());
    let digest = hash.finalize();
    let mut encoded = String::with_capacity(32);
    for byte in &digest[..16] {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    Ok(ChoiceId::new(format!("ww-{encoded}")))
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
mod tests {
    use serde_json::json;

    use super::*;

    fn request() -> RenderRequest<'static> {
        RenderRequest {
            pg_source: b"DOCUMENT();",
            pg_path: "Library/OPL/select-one.pg",
            version: "00000000-0000-0000-0000-000000000007",
            seed: 19,
        }
    }

    fn config() -> HttpWebworkRendererConfig {
        HttpWebworkRendererConfig::new(
            "http://webwork.internal/webwork2/",
            Duration::from_secs(1),
            1024,
            RendererIdentity {
                id: "webwork-source-pin".into(),
                version: "2.21".into(),
            },
            "ple_render",
            "ple_service",
            "not-in-browser",
        )
        .expect("recorded private configuration is valid")
    }

    fn hidden(settings: &HttpWebworkRendererConfig) -> Map<String, Value> {
        let expected = ExpectedEcho::from_request(settings, request());
        serde_json::from_value(json!({
            "sourceFilePath":"", "problemSource": expected.source, "problemSeed": expected.seed,
            "problemUUID":"", "psvn":"", "pathToProblemFile": expected.file,
            "courseID": expected.course_id, "user": expected.user, "passwd": expected.password,
            "displayMode":"MathJax", "key":"upstream-session-key", "outputformat":"json",
            "theme":"", "language":"", "showSummary":"0", "showHints":"0", "showSolutions":"0",
            "showPreviewButton":"0", "showCheckAnswersButton":"0", "showCorrectAnswersButton":"0",
            "showFooter":"0", "extraHeaderText":""
        }))
        .expect("recorded official hidden map")
    }

    fn response(settings: &HttpWebworkRendererConfig, body: &str) -> Map<String, Value> {
        let mut response = Map::new();
        for key in RESPONSE_KEYS {
            response.insert((*key).to_owned(), Value::String(String::new()));
        }
        response.insert("hidden_input_field".into(), Value::Object(hidden(settings)));
        response.insert("body_part550".into(), Value::String(body.to_owned()));
        response.insert("score".into(), json!(0));
        response.insert(
            "real_webwork_SITE_URL".into(),
            Value::String("http://webwork.internal/".into()),
        );
        response.insert(
            "real_webwork_FORM_ACTION_URL".into(),
            Value::String("http://webwork.internal/webwork2/render_rpc".into()),
        );
        response
    }

    fn response_for_service(
        settings: &HttpWebworkRendererConfig,
        body: &str,
        service_base: &str,
        score: f64,
    ) -> Value {
        let mut value = response(settings, body);
        let origin = Url::parse(service_base)
            .expect("test service URL parses")
            .origin()
            .ascii_serialization();
        value.insert(
            "real_webwork_SITE_URL".into(),
            Value::String(format!("{origin}/")),
        );
        value.insert(
            "real_webwork_FORM_ACTION_URL".into(),
            Value::String(format!("{service_base}render_rpc")),
        );
        value.insert("score".into(), json!(score));
        Value::Object(value)
    }

    async fn start_http_fixture(response: String) -> (String, tokio::task::JoinHandle<String>) {
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener binds");
        let address = listener.local_addr().expect("test listener has address");
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("test listener accepts");
            let mut bytes = Vec::new();
            let mut chunk = [0_u8; 1024];
            loop {
                let read = stream.read(&mut chunk).await.expect("test request reads");
                if read == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..read]);
                if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&bytes[..header_end + 4]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .expect("test request has content length");
                    if bytes.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
            }
            stream
                .write_all(response.as_bytes())
                .await
                .expect("test response writes");
            String::from_utf8(bytes).expect("test request is UTF-8")
        });
        (format!("http://{address}/webwork2/"), task)
    }

    fn http_response(status: &str, content_type: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    async fn start_two_http_fixture(
        responses_for: impl FnOnce(std::net::SocketAddr) -> [String; 2] + Send + 'static,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<Vec<String>>) {
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener binds");
        let address = listener.local_addr().expect("test listener has address");
        let responses = responses_for(address);
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (mut stream, _) = listener.accept().await.expect("test listener accepts");
                let mut bytes = Vec::new();
                let mut chunk = [0_u8; 1024];
                loop {
                    let read = stream.read(&mut chunk).await.expect("test request reads");
                    if read == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&chunk[..read]);
                    if let Some(header_end) =
                        bytes.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        let headers = String::from_utf8_lossy(&bytes[..header_end + 4]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                line.split_once(':').and_then(|(name, value)| {
                                    name.eq_ignore_ascii_case("content-length")
                                        .then(|| value.trim().parse::<usize>().ok())
                                        .flatten()
                                })
                            })
                            .expect("test request has content length");
                        if bytes.len() >= header_end + 4 + content_length {
                            break;
                        }
                    }
                }
                stream
                    .write_all(response.as_bytes())
                    .await
                    .expect("test response writes");
                requests.push(String::from_utf8(bytes).expect("test request is UTF-8"));
            }
            requests
        });
        (address, task)
    }

    #[test]
    fn recorded_upstream_radio_result_becomes_answer_free_multiple_choice() {
        let settings = config();
        // WeBWorK 2.21 PGbasicmacros NAMED_ANS_RADIO emits a wrapping
        // label; parserRadioButtons wraps the group in this container.
        let value = Value::Object(response(
            &settings,
            r#"<p>Which molecule is water?</p><div class="radio-buttons-container" data-feedback-insert-element="1"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">H2O</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">CO2</label></div>"#,
        ));
        let parsed = parse_render_rpc(
            value,
            ExpectedEcho::from_request(&settings, request()),
            request(),
            &settings.base_uri,
        )
        .expect("recorded response is supported");
        let ResponseDefinition::MultipleChoice { choices, selection } = &parsed.envelope.response
        else {
            panic!("single-choice envelope")
        };
        assert_eq!(*selection, SelectionCardinality::ExactlyOne);
        assert_eq!(choices.len(), 2);
        let prompt = match &parsed.envelope.prompt[0] {
            ContentBlock::Text { markdown } => markdown,
            _ => panic!("text prompt"),
        };
        assert!(prompt.contains("Which molecule is water?"));
        assert!(!prompt.contains("H2O") && !prompt.contains("CO2"));
        assert_eq!(parsed.choice_fields.len(), 2);
        let serialized =
            serde_json::to_string(&parsed.envelope).expect("public envelope serializes");
        assert!(!serialized.contains("AnSwEr0001"));
        assert!(!serialized.contains("not-in-browser"));
        assert!(!serialized.contains("upstream-session-key"));
        assert!(!parsed.html.contains("AnSwEr0001"));
        assert!(!parsed.html.contains("upstream-session-key"));
    }

    #[test]
    fn hidden_credential_mismatch_and_protected_top_level_data_refuse() {
        let settings = config();
        let expected = ExpectedEcho::from_request(&settings, request());
        let mut mismatch = response(&settings, "<p>ignored</p>");
        mismatch.insert("hidden_input_field".into(), json!({"sourceFilePath":"", "problemSource":"", "problemSeed":"", "problemUUID":"", "psvn":"", "pathToProblemFile":"", "courseID":"", "user":"", "passwd":"attacker", "displayMode":"MathJax", "key":"", "outputformat":"json", "theme":"", "language":"", "showSummary":"0", "showHints":"0", "showSolutions":"0", "showPreviewButton":"0", "showCheckAnswersButton":"0", "showCorrectAnswersButton":"0", "showFooter":"0", "extraHeaderText":""}));
        assert!(
            parse_render_rpc(
                Value::Object(mismatch),
                expected,
                request(),
                &settings.base_uri,
            )
            .is_err()
        );
        let mut protected = response(&settings, "<p>ignored</p>");
        protected.insert("PG_ANSWERS_HASH".into(), Value::String("secret".into()));
        assert!(
            parse_render_rpc(
                Value::Object(protected),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri,
            )
            .is_err()
        );
    }

    #[test]
    fn config_debug_redacts_direct_webwork_password() {
        let config = config();
        let debug = format!("{config:?}");
        assert!(!debug.contains("not-in-browser"));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn duplicate_unknown_and_off_origin_upstream_members_refuse() {
        assert!(parse_json_without_duplicates(br#"{"score":0,"score":1}"#).is_err());
        let settings = config();
        let mut unknown = response(&settings, "<p>x</p>");
        unknown.insert("answerHash".into(), Value::String("no".into()));
        assert!(
            parse_render_rpc(
                Value::Object(unknown),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri
            )
            .is_err()
        );
        let mut off_origin = response(&settings, "<p>x</p>");
        off_origin.insert(
            "real_webwork_SITE_URL".into(),
            Value::String("https://attacker.example/webwork2/".into()),
        );
        assert!(
            parse_render_rpc(
                Value::Object(off_origin),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri
            )
            .is_err()
        );
    }

    #[test]
    fn oversized_or_malformed_upstream_session_key_refuses() {
        let settings = config();
        let mut oversized = hidden(&settings);
        oversized.insert("key".to_string(), Value::String("x".repeat(4097)));
        assert!(
            validate_and_discard_hidden(
                &oversized,
                &ExpectedEcho::from_request(&settings, request())
            )
            .is_err()
        );
        let mut malformed = hidden(&settings);
        malformed.insert("key".to_string(), Value::Bool(true));
        assert!(
            validate_and_discard_hidden(
                &malformed,
                &ExpectedEcho::from_request(&settings, request())
            )
            .is_err()
        );
    }

    #[test]
    fn official_shape_requires_every_member_and_separates_site_from_form_url() {
        let settings = config();
        let supported_body = r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a">A</label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#;
        assert!(
            parse_render_rpc(
                Value::Object(response(&settings, supported_body)),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri,
            )
            .is_ok()
        );

        let mut missing = response(&settings, supported_body);
        missing.remove("head_part999");
        assert!(
            parse_render_rpc(
                Value::Object(missing),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri,
            )
            .is_err()
        );

        let mut wrong_form = response(&settings, supported_body);
        wrong_form.insert(
            "real_webwork_FORM_ACTION_URL".into(),
            Value::String("http://webwork.internal/".into()),
        );
        assert!(
            parse_render_rpc(
                Value::Object(wrong_form),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri,
            )
            .is_err()
        );
    }

    #[test]
    fn hostile_radio_markup_and_protected_html_refuse_before_browser_projection() {
        let settings = config();
        for body in [
            r#"<p>Question</p><!-- fake <input> --><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a">A</label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#,
            r#"<p>Question</p><script><input type="radio"></script><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a">A</label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#,
            r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a" id="again">A</label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#,
            r#"<p>Question</p><div class="radio-buttons-container"><span><label><input type="radio" name="AnSwEr1" value="0" id="a">A</label></span><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#,
        ] {
            assert!(
                parse_render_rpc(
                    Value::Object(response(&settings, body)),
                    ExpectedEcho::from_request(&settings, request()),
                    request(),
                    &settings.base_uri,
                )
                .is_err()
            );
        }
        let leaked = format!(
            r#"<p>{}</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a">A</label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#,
            settings.password
        );
        assert!(
            parse_render_rpc(
                Value::Object(response(&settings, &leaked)),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri,
            )
            .is_err()
        );
        let session_key_leaked = r#"<p>upstream-session-key</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a">A</label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#;
        assert!(
            parse_render_rpc(
                Value::Object(response(&settings, session_key_leaked)),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri,
            )
            .is_err()
        );
    }

    #[test]
    fn request_and_score_limits_fail_before_network_use() {
        let invalid_path = RenderRequest {
            pg_path: "../outside.pg",
            ..request()
        };
        assert!(validate_render_request(invalid_path).is_err());
        let oversized = vec![b'x'; MAX_PG_SOURCE_BYTES + 1];
        assert!(
            validate_render_request(RenderRequest {
                pg_source: &oversized,
                ..request()
            })
            .is_err()
        );
    }

    #[tokio::test]
    async fn private_http_client_posts_only_to_render_rpc_with_form_fields() {
        let body = r#"{"score":0}"#;
        let (base, task) =
            start_http_fixture(http_response("200 OK", "application/json", body)).await;
        let settings = HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            1024,
            RendererIdentity {
                id: "test".into(),
                version: "1".into(),
            },
            "course",
            "user",
            "password",
        )
        .expect("fixture config");
        let renderer = HttpWebworkRenderer::new(settings).expect("fixture client");
        let result = renderer
            .rpc(BTreeMap::from([("problemSeed".into(), "19".into())]))
            .await;
        assert_eq!(result.expect("fixture JSON")["score"], 0);
        let request = task.await.expect("fixture task completes");
        assert!(request.starts_with("POST /webwork2/render_rpc HTTP/1.1\r\n"));
        let lower = request.to_ascii_lowercase();
        assert!(lower.contains("content-type: application/x-www-form-urlencoded"));
        assert!(request.contains("courseID=course"));
        assert!(request.contains("user=user"));
        assert!(request.contains("passwd=password"));
        assert!(request.contains("outputformat=json"));
        assert!(!request.contains("/v1/"));
    }

    #[tokio::test]
    async fn private_http_client_refuses_redirect_non_json_and_oversized_responses() {
        let redirect = "HTTP/1.1 302 Found\r\nlocation: /login\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
        let (base, task) = start_http_fixture(redirect.into()).await;
        let renderer = HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                16_384,
                RendererIdentity {
                    id: "test".into(),
                    version: "1".into(),
                },
                "course",
                "user",
                "password",
            )
            .expect("fixture config"),
        )
        .expect("fixture client");
        assert!(matches!(
            renderer.rpc(BTreeMap::new()).await,
            Err(RendererFailure::InvalidOutput(_))
        ));
        let _ = task.await.expect("fixture task completes");

        let (base, task) =
            start_http_fixture(http_response("200 OK", "text/html", "not json")).await;
        let renderer = HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                1024,
                RendererIdentity {
                    id: "test".into(),
                    version: "1".into(),
                },
                "course",
                "user",
                "password",
            )
            .expect("fixture config"),
        )
        .expect("fixture client");
        assert!(matches!(
            renderer.rpc(BTreeMap::new()).await,
            Err(RendererFailure::InvalidOutput(_))
        ));
        let _ = task.await.expect("fixture task completes");

        let oversized = "x".repeat(1025);
        let (base, task) =
            start_http_fixture(http_response("200 OK", "application/json", &oversized)).await;
        let renderer = HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                1024,
                RendererIdentity {
                    id: "test".into(),
                    version: "1".into(),
                },
                "course",
                "user",
                "password",
            )
            .expect("fixture config"),
        )
        .expect("fixture client");
        assert!(matches!(
            renderer.rpc(BTreeMap::new()).await,
            Err(RendererFailure::ResourceExhausted)
        ));
        let _ = task.await.expect("fixture task completes");
    }

    #[tokio::test]
    async fn grade_rerenders_then_submits_only_the_selected_upstream_radio_value() {
        const BODY: &str = r#"<p>Which molecule is water?</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">H2O</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">CO2</label></div>"#;
        let (address, task) = start_two_http_fixture(move |address| {
            let base = format!("http://{address}/webwork2/");
            let settings = HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                1024,
                RendererIdentity {
                    id: "test".into(),
                    version: "1".into(),
                },
                "course",
                "user",
                "password",
            )
            .expect("fixture config");
            let rendered = response_for_service(&settings, BODY, &base, 0.0);
            let graded = response_for_service(&settings, BODY, &base, 100.0);
            [
                http_response("200 OK", "application/json", &rendered.to_string()),
                http_response("200 OK", "application/json", &graded.to_string()),
            ]
        })
        .await;
        let base = format!("http://{address}/webwork2/");
        let settings = HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            16_384,
            RendererIdentity {
                id: "test".into(),
                version: "1".into(),
            },
            "course",
            "user",
            "password",
        )
        .expect("fixture config");
        let selected = opaque_choice_id(request(), 1).expect("fixed choice ID");
        let student_response = StudentResponse::MultipleChoice {
            selected: vec![selected],
        };
        let renderer = HttpWebworkRenderer::new(settings).expect("fixture client");
        let result = renderer
            .grade(GradeRequest {
                pg_source: request().pg_source,
                pg_path: request().pg_path,
                version: request().version,
                seed: request().seed,
                response: &student_response,
                points_possible: 7.0,
            })
            .await
            .expect("100 percent answer grades");
        assert!(matches!(
            result,
            GradeOutcome::Graded(AttemptResult {
                correct: true,
                points_earned: 7.0,
                points_possible: 7.0,
            })
        ));
        let requests = task.await.expect("fixture task completes");
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("POST /webwork2/render_rpc HTTP/1.1\r\n"));
        assert!(!requests[0].contains("WWsubmit=1"));
        assert!(!requests[0].contains("AnSwEr0001"));
        assert!(requests[1].starts_with("POST /webwork2/render_rpc HTTP/1.1\r\n"));
        assert!(requests[1].contains("WWsubmit=1"));
        assert!(requests[1].contains("AnSwEr0001=1"));
        assert!(!requests[1].contains("AnSwEr0001=0"));

        let settings = config();
        let parsed = parse_render_rpc(
            response_for_service(&settings, BODY, "http://webwork.internal/webwork2/", 0.0),
            ExpectedEcho::from_request(&settings, request()),
            request(),
            &settings.base_uri,
        )
        .expect("accepted result is safe to cache");
        let public = serde_json::to_string(&parsed.envelope).expect("public envelope serializes");
        for protected in [
            "AnSwEr0001",
            "upstream-session-key",
            "not-in-browser",
            "RE9DVU1FTlQoKTs=",
        ] {
            assert!(!public.contains(protected));
            assert!(!parsed.html.contains(protected));
        }
    }

    #[tokio::test]
    async fn grade_refuses_fractional_upstream_score() {
        const BODY: &str = r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">A</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">B</label></div>"#;
        let (address, task) = start_two_http_fixture(move |address| {
            let base = format!("http://{address}/webwork2/");
            let settings = HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                16_384,
                RendererIdentity {
                    id: "test".into(),
                    version: "1".into(),
                },
                "course",
                "user",
                "password",
            )
            .expect("fixture config");
            let render = response_for_service(&settings, BODY, &base, 0.0);
            let fractional = response_for_service(&settings, BODY, &base, 50.0);
            [
                http_response("200 OK", "application/json", &render.to_string()),
                http_response("200 OK", "application/json", &fractional.to_string()),
            ]
        })
        .await;
        let base = format!("http://{address}/webwork2/");
        let settings = HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            16_384,
            RendererIdentity {
                id: "test".into(),
                version: "1".into(),
            },
            "course",
            "user",
            "password",
        )
        .expect("fixture config");
        let selected = opaque_choice_id(request(), 0).expect("fixed choice ID");
        let response = StudentResponse::MultipleChoice {
            selected: vec![selected],
        };
        let renderer = HttpWebworkRenderer::new(settings).expect("fixture client");
        assert!(matches!(
            renderer
                .grade(GradeRequest {
                    pg_source: request().pg_source,
                    pg_path: request().pg_path,
                    version: request().version,
                    seed: request().seed,
                    response: &response,
                    points_possible: 7.0,
                })
                .await,
            Err(RendererFailure::InvalidOutput(_))
        ));
        assert_eq!(task.await.expect("fixture task completes").len(), 2);
    }

    #[tokio::test]
    async fn grade_maps_zero_percent_to_zero_earned_points() {
        const BODY: &str = r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">A</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">B</label></div>"#;
        let (address, task) = start_two_http_fixture(move |address| {
            let base = format!("http://{address}/webwork2/");
            let settings = HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                16_384,
                RendererIdentity {
                    id: "test".into(),
                    version: "1".into(),
                },
                "course",
                "user",
                "password",
            )
            .expect("fixture config");
            let render = response_for_service(&settings, BODY, &base, 0.0);
            let incorrect = response_for_service(&settings, BODY, &base, 0.0);
            [
                http_response("200 OK", "application/json", &render.to_string()),
                http_response("200 OK", "application/json", &incorrect.to_string()),
            ]
        })
        .await;
        let base = format!("http://{address}/webwork2/");
        let renderer = HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                16_384,
                RendererIdentity {
                    id: "test".into(),
                    version: "1".into(),
                },
                "course",
                "user",
                "password",
            )
            .expect("fixture config"),
        )
        .expect("fixture client");
        let response = StudentResponse::MultipleChoice {
            selected: vec![opaque_choice_id(request(), 0).expect("fixed choice ID")],
        };
        let result = renderer
            .grade(GradeRequest {
                pg_source: request().pg_source,
                pg_path: request().pg_path,
                version: request().version,
                seed: request().seed,
                response: &response,
                points_possible: 7.0,
            })
            .await
            .expect("zero percent answer grades");
        assert!(matches!(
            result,
            GradeOutcome::Graded(AttemptResult {
                correct: false,
                points_earned: 0.0,
                points_possible: 7.0,
            })
        ));
        assert_eq!(task.await.expect("fixture task completes").len(), 2);
    }

    #[tokio::test]
    async fn private_http_client_maps_deadline_to_timeout() {
        use tokio::io::AsyncReadExt as _;

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("test listener binds");
        let address = listener.local_addr().expect("test listener has address");
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("test listener accepts");
            let mut bytes = [0_u8; 1024];
            let _ = stream.read(&mut bytes).await.expect("test request reads");
            tokio::time::sleep(Duration::from_millis(100)).await;
        });
        let renderer = HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(
                &format!("http://{address}/webwork2/"),
                Duration::from_millis(10),
                1024,
                RendererIdentity {
                    id: "test".into(),
                    version: "1".into(),
                },
                "course",
                "user",
                "password",
            )
            .expect("fixture config"),
        )
        .expect("fixture client");
        assert!(matches!(
            renderer.rpc(BTreeMap::new()).await,
            Err(RendererFailure::TimedOut)
        ));
        task.await.expect("fixture task completes");
    }
}
