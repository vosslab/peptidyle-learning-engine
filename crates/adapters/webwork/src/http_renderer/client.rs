//! Private HTTP transport for the standalone PG renderer.
//!
//! This boundary validates the renderer envelope and forwards opaque backend
//! documents and form pairs. It deliberately has no PG-control or HTML
//! projection vocabulary.

use std::time::Duration;

use async_trait::async_trait;
use question_model::QuestionRendererVersion;
use reqwest::header::{ACCEPT, CONTENT_TYPE, LOCATION};
use reqwest::{Client, StatusCode, Url};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use super::response_shape::RESPONSE_KEYS;
use crate::renderer_contract::{
    GradeRequest, RenderRequest, RenderedWebworkQuestion, RendererFailure, ResumeRenderRequest,
    WebworkRenderer,
};

const JSON_MEDIA_TYPE: &str = "application/json";
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1_048_576;
const MAX_PG_SOURCE_BYTES: usize = 262_144;
const MAX_PG_PATH_BYTES: usize = 1_024;
const MAX_RESPONSE_PAYLOAD_BYTES: usize = 65_536;
const MAX_FIELD_NAME_BYTES: usize = 1_024;
const MAX_FIELD_VALUE_BYTES: usize = 65_536;
const MAX_FIELD_PAIRS: usize = 1_024;
const SHOW_CORRECT_ANSWERS_PREFIX: &str = "showCorrectAnswers";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RendererConfigError {
    InvalidBaseUri,
    InvalidPleOrigin,
    InvalidLimits,
    MissingQuestionRendererVersion,
}

impl std::fmt::Display for RendererConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidBaseUri => {
                "renderer base URI must be an absolute credential-free http(s) origin"
            }
            Self::InvalidPleOrigin => {
                "PLE origin must be an absolute credential-free http(s) origin"
            }
            Self::InvalidLimits => "renderer deadlines and response limit must be positive",
            Self::MissingQuestionRendererVersion => "Question Renderer Version must be configured",
        })
    }
}
impl std::error::Error for RendererConfigError {}

#[derive(Clone)]
pub struct HttpWebworkRendererConfig {
    base_uri: Url,
    ple_origin: Url,
    ple_asset_base: String,
    deadline: Duration,
    max_response_bytes: usize,
    expected_renderer: QuestionRendererVersion,
}

impl HttpWebworkRendererConfig {
    pub fn new(
        base_uri: &str,
        deadline: Duration,
        max_response_bytes: usize,
        expected_renderer: QuestionRendererVersion,
        ple_origin: &str,
    ) -> Result<Self, RendererConfigError> {
        let base_uri = validated_origin(base_uri, RendererConfigError::InvalidBaseUri)?;
        let ple_origin = validated_origin(ple_origin, RendererConfigError::InvalidPleOrigin)?;
        if deadline.is_zero() || max_response_bytes == 0 {
            return Err(RendererConfigError::InvalidLimits);
        }
        if expected_renderer.name.trim().is_empty() || expected_renderer.version.trim().is_empty() {
            return Err(RendererConfigError::MissingQuestionRendererVersion);
        }
        let ple_asset_base = format!("{}api/webwork-assets", ple_origin.as_str());
        Ok(Self {
            base_uri,
            ple_origin,
            ple_asset_base,
            deadline,
            max_response_bytes,
            expected_renderer,
        })
    }

    pub fn with_default_response_limit(
        base_uri: &str,
        deadline: Duration,
        expected_renderer: QuestionRendererVersion,
        ple_origin: &str,
    ) -> Result<Self, RendererConfigError> {
        Self::new(
            base_uri,
            deadline,
            DEFAULT_MAX_RESPONSE_BYTES,
            expected_renderer,
            ple_origin,
        )
    }
}

fn validated_origin(value: &str, error: RendererConfigError) -> Result<Url, RendererConfigError> {
    let uri = Url::parse(value).map_err(|_| error.clone())?;
    if !matches!(uri.scheme(), "http" | "https")
        || uri.host_str().is_none()
        || !uri.username().is_empty()
        || uri.password().is_some()
        || uri.query().is_some()
        || uri.fragment().is_some()
        || uri.path() != "/"
    {
        return Err(error);
    }
    Ok(uri)
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

    async fn rpc(&self, fields: Vec<(String, String)>) -> Result<Value, RendererFailure> {
        let target = self
            .settings
            .base_uri
            .join(crate::standalone_render_api::PATH)
            .map_err(|_| bad("renderer URI is invalid"))?;
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
            return Err(bad("renderer redirected request"));
        }
        map_status(response.status())?;
        validate_content_type(&response)?;
        let bytes = read_bounded(response, self.settings.max_response_bytes).await?;
        let value = parse_json_without_duplicates(&bytes)?;
        if value.get("error").is_some() {
            return Err(bad("renderer rejected trusted request"));
        }
        Ok(value)
    }
}

#[async_trait]
impl WebworkRenderer for HttpWebworkRenderer {
    fn identity(&self) -> &QuestionRendererVersion {
        &self.settings.expected_renderer
    }

    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        validate_render_request(request)?;
        let value = self
            .rpc(super::protocol::render_fields(
                request,
                &self.settings.ple_origin,
                &self.settings.ple_asset_base,
            ))
            .await?;
        let document = validate_render_rpc(value)?;
        Ok(RenderedWebworkQuestion::from_document(
            document,
            self.settings.expected_renderer.clone(),
        ))
    }

    async fn render_saved_response(
        &self,
        request: ResumeRenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        let render_request = RenderRequest {
            pg_source: request.pg_source,
            pg_path: request.pg_path,
            question_revision: request.question_revision,
            seed: request.seed,
        };
        validate_render_request(render_request)?;
        let mut fields = super::protocol::render_fields(
            render_request,
            &self.settings.ple_origin,
            &self.settings.ple_asset_base,
        );
        fields.extend(decode_response_pairs(request.response_payload)?);
        let document = validate_render_rpc(self.rpc(fields).await?)?;
        Ok(RenderedWebworkQuestion::from_document(
            document,
            self.settings.expected_renderer.clone(),
        ))
    }

    async fn grade(
        &self,
        request: GradeRequest<'_>,
    ) -> Result<grading::QuestionGradingOutcome, RendererFailure> {
        validate_render_request(RenderRequest {
            pg_source: request.pg_source,
            pg_path: request.pg_path,
            question_revision: request.question_revision,
            seed: request.seed,
        })?;
        if request.lifecycle_state.as_deref().is_some() {
            return Err(bad("WeBWorK uses stateless grading"));
        }
        let mut fields = super::protocol::render_fields(
            RenderRequest {
                pg_source: request.pg_source,
                pg_path: request.pg_path,
                question_revision: request.question_revision,
                seed: request.seed,
            },
            &self.settings.ple_origin,
            &self.settings.ple_asset_base,
        );
        fields.extend(decode_response_pairs(request.response_payload)?);
        fields.push(("submitAnswers".into(), "1".into()));
        let score = validate_grade_rpc(self.rpc(fields).await?)?;
        super::grade::score(score)
    }
}

fn validate_render_request(request: RenderRequest<'_>) -> Result<(), RendererFailure> {
    if request.pg_source.is_empty() || request.pg_source.len() > MAX_PG_SOURCE_BYTES {
        return Err(bad("WeBWorK source exceeds the supported bound"));
    }
    if request.pg_path.is_empty()
        || request.pg_path.len() > MAX_PG_PATH_BYTES
        || request.pg_path.starts_with('/')
        || request.pg_path.contains(['\\', '\0'])
        || request
            .pg_path
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err(bad("WeBWorK source path is outside the supported contract"));
    }
    Ok(())
}

fn validate_render_rpc(value: Value) -> Result<Vec<u8>, RendererFailure> {
    let object = value
        .as_object()
        .ok_or_else(|| bad("renderer JSON is not an object"))?;
    validate_response_shape(object)?;
    let private_jwts = validate_private_jwt_values(
        object
            .get("JWT")
            .and_then(Value::as_object)
            .ok_or_else(|| bad("renderer omitted private JWT state"))?,
    )?;
    let html = object
        .get("renderedHTML")
        .and_then(Value::as_str)
        .filter(|html| !html.is_empty())
        .ok_or_else(|| bad("renderer omitted rendered HTML"))?;
    if html.len() > DEFAULT_MAX_RESPONSE_BYTES {
        return Err(bad("renderer HTML exceeds the supported bound"));
    }
    reject_reflected_private_jwts(html, &private_jwts)?;
    Ok(html.as_bytes().to_vec())
}

fn validate_grade_rpc(value: Value) -> Result<f64, RendererFailure> {
    let object = value
        .as_object()
        .ok_or_else(|| bad("renderer JSON is not an object"))?;
    validate_response_shape(object)?;
    let _ = validate_private_jwt_values(
        object
            .get("JWT")
            .and_then(Value::as_object)
            .ok_or_else(|| bad("renderer omitted private JWT state"))?,
    )?;
    object
        .get("problem_result")
        .and_then(Value::as_object)
        .and_then(|result| result.get("score"))
        .and_then(Value::as_f64)
        .filter(|score| score.is_finite() && (0.0..=1.0).contains(score))
        .map(|score| score * 100.0)
        .ok_or_else(|| bad("renderer returned malformed normalized score"))
}

fn validate_response_shape(object: &Map<String, Value>) -> Result<(), RendererFailure> {
    if object.len() != RESPONSE_KEYS.len()
        || RESPONSE_KEYS.iter().any(|key| !object.contains_key(*key))
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

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value without duplicate object members")
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

fn decode_response_pairs(payload: &[u8]) -> Result<Vec<(String, String)>, RendererFailure> {
    if payload.len() > MAX_RESPONSE_PAYLOAD_BYTES {
        return Err(bad("backend-owned response exceeds the supported bound"));
    }
    let pairs: Vec<(String, String)> = serde_json::from_slice(payload)
        .map_err(|_| bad("backend-owned response is not a pair array"))?;
    if pairs.len() > MAX_FIELD_PAIRS
        || serde_json::to_vec(&pairs)
            .expect("pair serialization")
            .as_slice()
            != payload
    {
        return Err(bad("backend-owned response is not canonical"));
    }
    for (name, value) in &pairs {
        if name.is_empty()
            || name.len() > MAX_FIELD_NAME_BYTES
            || value.len() > MAX_FIELD_VALUE_BYTES
            || reserved_field(name)
        {
            return Err(bad(
                "backend-owned response contains a reserved or invalid field",
            ));
        }
    }
    Ok(pairs)
}

fn reserved_field(name: &str) -> bool {
    let is_show_correct_answers = name
        .get(..SHOW_CORRECT_ANSWERS_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(SHOW_CORRECT_ANSWERS_PREFIX));
    is_show_correct_answers
        || [
            "_format",
            "outputformat",
            "problemsource",
            "rawproblemsource",
            "uriencodedproblemsource",
            "sourcefilepath",
            "problemseed",
            "submitanswers",
            "answerssubmitted",
            "previewanswers",
            "processanswers",
            "problemsourceurl",
            "displaymode",
            "isinstructor",
            "showsummary",
            "showhints",
            "showsolutions",
            "hidepreviewbutton",
            "hidecheckanswersbutton",
            "hideattemptstable",
            "hidemessages",
            "showcorrectanswersbutton",
            "showfooter",
            "pleassetbase",
            "pleorigin",
            "jwt",
            "sessionjwt",
            "problemjwt",
            "answer",
            "passwd",
            "password",
        ]
        .iter()
        .any(|reserved| name.eq_ignore_ascii_case(reserved))
}

fn validate_private_jwt_values(jwt: &Map<String, Value>) -> Result<[&str; 3], RendererFailure> {
    if jwt.len() != 3
        || ["problem", "session", "answer"]
            .iter()
            .any(|key| !jwt.contains_key(*key))
    {
        return Err(bad("renderer JWT state has an unsupported shape"));
    }
    Ok([
        private_jwt_value(jwt, "problem")?,
        private_jwt_value(jwt, "session")?,
        private_jwt_value(jwt, "answer")?,
    ])
}

fn private_jwt_value<'a>(
    jwt: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, RendererFailure> {
    jwt.get(key)
        .and_then(Value::as_str)
        .filter(|token| !token.is_empty() && token.len() <= DEFAULT_MAX_RESPONSE_BYTES)
        .ok_or_else(|| bad("renderer returned malformed private JWT state"))
}

fn reject_reflected_private_jwts(
    html: &str,
    private_jwts: &[&str; 3],
) -> Result<(), RendererFailure> {
    if private_jwts.iter().any(|token| html.contains(token)) {
        return Err(bad("renderer HTML contained private JWT state"));
    }
    Ok(())
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
