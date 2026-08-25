//! Strict HTTP grammar for retention mutation requests.

use axum::body::{Bytes, to_bytes};
use axum::extract::Request;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::http::header::{HeaderValue, IF_MATCH};
use learning_data_access::RetentionRevision;
use serde::Serialize;
use serde::de::{self, DeserializeOwned, MapAccess, Visitor};

use super::MAX_RETENTION_BODY_BYTES;
use super::projection::error_response;
use crate::http_refusal::HttpResult;

const JSON_MIME_TYPE: &str = "application/json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IfMatchError {
    Missing,
    Malformed,
}

pub(super) fn required_if_match_revision(
    headers: &HeaderMap,
) -> Result<RetentionRevision, IfMatchError> {
    let mut values = headers.get_all(IF_MATCH).iter();
    let Some(value) = values.next() else {
        return Err(IfMatchError::Missing);
    };
    if values.next().is_some() {
        return Err(IfMatchError::Malformed);
    }
    let quoted = value.to_str().map_err(|_| IfMatchError::Malformed)?;
    let Some(value) = quoted
        .strip_prefix('\"')
        .and_then(|value| value.strip_suffix('\"'))
    else {
        return Err(IfMatchError::Malformed);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(IfMatchError::Malformed);
    }
    let revision = value.parse::<u64>().map_err(|_| IfMatchError::Malformed)?;
    RetentionRevision::new(revision).map_err(|_| IfMatchError::Malformed)
}

pub(super) fn is_application_json_content_type(content_type: Option<&HeaderValue>) -> bool {
    content_type
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value
                .split(';')
                .next()
                .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(JSON_MIME_TYPE))
        })
}

pub(super) async fn read_body(request: Request) -> HttpResult<Bytes> {
    to_bytes(request.into_body(), MAX_RETENTION_BODY_BYTES)
        .await
        .map_err(|_| {
            error_response(StatusCode::PAYLOAD_TOO_LARGE, "request body is too large").into()
        })
}

pub(super) fn parse_strict_json<T>(body: Bytes) -> Result<T, ()>
where
    T: DeserializeOwned + Serialize,
{
    let request: T = serde_json::from_slice(&body).map_err(|_| ())?;
    let _: DuplicateKeyJsonValue = serde_json::from_slice(&body).map_err(|_| ())?;
    let value: serde_json::Value = serde_json::from_slice(&body).map_err(|_| ())?;
    if serde_json::to_value(&request).map_err(|_| ())? == value {
        Ok(request)
    } else {
        Err(())
    }
}

#[derive(Debug)]
enum DuplicateKeyJsonValue {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl<'de> de::Deserialize<'de> for DuplicateKeyJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct DuplicateKeyVisitor;
        impl<'de> Visitor<'de> for DuplicateKeyVisitor {
            type Value = DuplicateKeyJsonValue;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("valid JSON")
            }
            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Null)
            }
            fn visit_bool<E>(self, _: bool) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Bool)
            }
            fn visit_i64<E>(self, _: i64) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Number)
            }
            fn visit_u64<E>(self, _: u64) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Number)
            }
            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                serde_json::Number::from_f64(value)
                    .ok_or_else(|| de::Error::custom("invalid JSON number"))?;
                Ok(DuplicateKeyJsonValue::Number)
            }
            fn visit_str<E>(self, _: &str) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::String)
            }
            fn visit_string<E>(self, _: String) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::String)
            }
            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(DuplicateKeyJsonValue::Null)
            }
            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                de::Deserialize::deserialize(deserializer)
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                while seq.next_element::<DuplicateKeyJsonValue>()?.is_some() {}
                Ok(DuplicateKeyJsonValue::Array)
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                use std::collections::HashSet;
                let mut fields = HashSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !fields.insert(key.clone()) {
                        return Err(de::Error::custom(format!(
                            "JSON object has duplicate key: {key}"
                        )));
                    }
                    let _: DuplicateKeyJsonValue = map.next_value()?;
                }
                Ok(DuplicateKeyJsonValue::Object)
            }
        }
        deserializer.deserialize_any(DuplicateKeyVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;
    use question_model::{
        RetentionArchiveRequest, RetentionDispositionView, RetentionExtendRequest,
    };

    #[test]
    fn parse_strict_json_only_accepts_strict_body() {
        let retain = serde_json::to_vec(&RetentionArchiveRequest {
            assignment_definitions: RetentionDispositionView::Retain,
        })
        .expect("serialize archive fixture");
        assert!(parse_strict_json::<RetentionArchiveRequest>(Bytes::from(retain)).is_ok());
        let expanded = serde_json::to_vec(
            &serde_json::json!({ "assignmentDefinitions": "retain", "extra": "ignored" }),
        )
        .expect("serialize archive fixture");
        assert!(parse_strict_json::<RetentionArchiveRequest>(Bytes::from(expanded)).is_err());
    }
    #[test]
    fn parse_strict_json_rejects_wrong_types_and_duplicate_members() {
        assert!(parse_strict_json::<RetentionExtendRequest>(Bytes::from_static(br#"{}"#)).is_err());
        assert!(
            parse_strict_json::<RetentionExtendRequest>(Bytes::from_static(
                br#"{"additionalDays":3.5}"#
            ))
            .is_err()
        );
        assert!(
            parse_strict_json::<RetentionArchiveRequest>(Bytes::from_static(
                br#"{"assignmentDefinitions":"retain","assignmentDefinitions":"delete"}"#
            ))
            .is_err()
        );
    }
    #[test]
    fn content_type_and_if_match_require_exact_grammar() {
        assert!(is_application_json_content_type(Some(
            &HeaderValue::from_static("application/json; charset=utf-8")
        )));
        assert!(!is_application_json_content_type(Some(
            &HeaderValue::from_static("text/plain")
        )));
        let missing = HeaderMap::new();
        assert_eq!(
            required_if_match_revision(&missing),
            Err(IfMatchError::Missing)
        );
        for value in ["W/\"1\"", "bad", "\"0\"", "\"9223372036854775808\""] {
            let mut headers = HeaderMap::new();
            headers.insert(IF_MATCH, HeaderValue::from_str(value).expect("header"));
            assert_eq!(
                required_if_match_revision(&headers),
                Err(IfMatchError::Malformed)
            );
        }
        let mut headers = HeaderMap::new();
        headers.insert(IF_MATCH, HeaderValue::from_static("\"123\""));
        assert_eq!(
            required_if_match_revision(&headers).expect("etag").value(),
            123
        );
    }
}
