//! Native source validation and raw private Draft source access.

use axum::{
    body::Bytes,
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::Response,
};
use learning_data_access::AuthoringDraft;
use objects::{ObjectAddress, ObjectStore, PutObject, Sha256Checksum};
use question_model::{ObjectId, QuestionBackend, QuestionFormat, QuestionType, WorkspaceId};
use serde::Serialize;

use crate::authoring::{now, private_error};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicationSourceBinding<'a> {
    Native,
    Webwork {
        question_format: QuestionFormat,
        pg_path: &'a str,
        question_type: QuestionType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicationSourceValidationError {
    InvalidBinding,
    MissingWebworkType,
    InvalidSource,
    NativeTypeMismatch,
    RendererUnavailable,
}

/// Selects publication behavior only from the persisted source binding and Draft Type.
pub(crate) fn publication_source_binding(
    draft: &AuthoringDraft,
) -> Result<PublicationSourceBinding<'_>, PublicationSourceValidationError> {
    if draft.source_media_type() != Some(draft.source_record.media_type.as_str()) {
        return Err(PublicationSourceValidationError::InvalidBinding);
    }
    match (
        draft.question_backend,
        draft.question_format,
        draft.webwork_pg_path.as_deref(),
    ) {
        (QuestionBackend::Ple, QuestionFormat::PleQuestionJson, None) => {
            Ok(PublicationSourceBinding::Native)
        }
        (
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg | QuestionFormat::WebworkPgml,
            Some(pg_path),
        ) => draft
            .question_type
            .map(|question_type| PublicationSourceBinding::Webwork {
                question_format: draft.question_format,
                pg_path,
                question_type,
            })
            .ok_or(PublicationSourceValidationError::MissingWebworkType),
        _ => Err(PublicationSourceValidationError::InvalidBinding),
    }
}

pub(crate) fn validated_native_publication_source(
    draft: &AuthoringDraft,
    bytes: &[u8],
) -> Result<QuestionType, PublicationSourceValidationError> {
    if publication_source_binding(draft)? != PublicationSourceBinding::Native {
        return Err(PublicationSourceValidationError::InvalidBinding);
    }
    let question_type =
        validated_source(bytes).map_err(|_| PublicationSourceValidationError::InvalidSource)?;
    if Some(question_type) != draft.question_type {
        return Err(PublicationSourceValidationError::NativeTypeMismatch);
    }
    Ok(question_type)
}

pub(crate) fn validated_source(bytes: &[u8]) -> Result<QuestionType, Box<Response>> {
    let document =
        adapter_ple::question_json::PleQuestionJsonDocument::parse(bytes).map_err(|_| {
            Box::new(private_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Draft Question source is invalid",
            ))
        })?;
    let compiled = document.compile().map_err(|_| {
        Box::new(private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question source is invalid",
        ))
    })?;
    document.canonical_bytes().map_err(|_| {
        Box::new(private_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Draft Question source is invalid",
        ))
    })?;
    Ok(compiled.presentation().question_type())
}

/// Reads the source-derived Native Type when the current bytes compile.
/// Incomplete or broken source has no derived metadata and remains valid Draft content.
pub(crate) fn inspect_source(bytes: &[u8], backend: QuestionBackend) -> Option<QuestionType> {
    if backend != QuestionBackend::Ple {
        return None;
    }
    let Ok(document) = adapter_ple::question_json::PleQuestionJsonDocument::parse(bytes) else {
        return None;
    };
    let Ok(compiled) = document.compile() else {
        return None;
    };
    Some(compiled.presentation().question_type())
}

pub(crate) fn matches_source_content_type(headers: &HeaderMap, expected_media_type: &str) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media_type| media_type.trim().eq_ignore_ascii_case(expected_media_type))
}

/// Adds explicit source-binding headers while keeping the response body as raw source bytes.
pub(crate) fn source_response_headers(draft: &AuthoringDraft) -> Result<HeaderMap, ()> {
    let media_type = draft.source_media_type().ok_or(())?;
    if draft.source_record.media_type != media_type {
        return Err(());
    }
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(media_type));
    insert_serialized_header(
        &mut headers,
        "x-ple-question-backend",
        &draft.question_backend,
    )?;
    insert_serialized_header(
        &mut headers,
        "x-ple-question-format",
        &draft.question_format,
    )?;
    let path = draft
        .webwork_pg_path
        .as_deref()
        .map(percent_encode_header_value)
        .unwrap_or_default();
    headers.insert(
        "x-ple-webwork-pg-path",
        HeaderValue::from_str(&path).map_err(|_| ())?,
    );
    Ok(headers)
}

fn insert_serialized_header<T: Serialize>(
    headers: &mut HeaderMap,
    name: &'static str,
    value: &T,
) -> Result<(), ()> {
    let value = serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or(())?;
    headers.insert(name, HeaderValue::from_str(&value).map_err(|_| ())?);
    Ok(())
}

fn percent_encode_header_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(b"0123456789ABCDEF"[usize::from(byte >> 4)]));
            encoded.push(char::from(b"0123456789ABCDEF"[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}

pub(crate) async fn put_workspace_source(
    objects: &impl ObjectStore,
    workspace: WorkspaceId,
    bytes: Vec<u8>,
    media_type: &str,
) -> Result<objects::ObjectRecord, ()> {
    objects
        .put(PutObject {
            address: ObjectAddress::WorkspaceQuestionSource {
                workspace_id: workspace,
                object_id: ObjectId::generate(),
            },
            bytes,
            media_type: media_type.to_owned(),
            created_at: now(),
        })
        .await
        .map_err(|_| ())
}

pub(crate) async fn load_verified_source(
    objects: &impl ObjectStore,
    draft: &AuthoringDraft,
) -> Result<Bytes, ()> {
    let media_type = draft.source_media_type().ok_or(())?;
    let source = objects
        .get(&draft.source_record.address)
        .await
        .map_err(|_| ())?;
    if source.record != draft.source_record
        || source.record.media_type != media_type
        || source.bytes.len() as u64 != source.record.size_bytes
        || Sha256Checksum::compute(&source.bytes) != source.record.sha256
    {
        return Err(());
    }
    Ok(Bytes::from(source.bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authoring::PLE_QUESTION_JSON_MEDIA_TYPE;
    use learning_data_access::{DraftQuestionEditNumber, DraftQuestionUuid};
    use objects::{
        ObjectAddress, ObjectDataClass, ObjectRecord, ObjectStorageArea, Sha256Checksum,
        memory::MemoryObjectStore,
    };
    use question_model::{
        PublishedQuestionId, PublishedQuestionRevisionTuple, QuestionFormat, QuestionMetadata,
        QuestionRevisionNumber, Timestamp,
    };
    use uuid::Uuid;

    fn publication_draft(
        backend: QuestionBackend,
        format: QuestionFormat,
        path: Option<&str>,
        question_type: Option<QuestionType>,
        is_correction: bool,
    ) -> AuthoringDraft {
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(0x15));
        let object_id = ObjectId::generate();
        let bytes = b"stored source";
        let media_type = match backend {
            QuestionBackend::Ple => PLE_QUESTION_JSON_MEDIA_TYPE,
            QuestionBackend::Webwork => "text/x-wework-pg",
            QuestionBackend::Imathas => "application/octet-stream",
        };
        AuthoringDraft {
            draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(0x16)),
            workspace,
            parent_published_question_revision_tuple: is_correction.then(|| {
                PublishedQuestionRevisionTuple {
                    published_question_id: PublishedQuestionId::from_random_identifier("ABCDEFG")
                        .expect("valid Question ID"),
                    revision_number: QuestionRevisionNumber::new(3)
                        .expect("positive revision number"),
                }
            }),
            edit_number: DraftQuestionEditNumber::new(2).expect("positive Edit Number"),
            metadata: QuestionMetadata {
                question_title: "Title".into(),
                question_description: "Description".into(),
                tags: Vec::new(),
                question_license: None,
                question_citation: None,
                language: None,
            },
            classification: None,
            question_backend: backend,
            question_format: format,
            webwork_pg_path: path.map(str::to_owned),
            general_feedback: None,
            hint: None,
            worked_solution: None,
            authors: Vec::new(),
            question_type,
            source_record: ObjectRecord {
                id: object_id,
                storage_area: ObjectStorageArea::PrivateContent,
                data_class: ObjectDataClass::AuthoringContent,
                address: ObjectAddress::WorkspaceQuestionSource {
                    workspace_id: workspace,
                    object_id,
                },
                sha256: Sha256Checksum::compute(bytes),
                size_bytes: bytes.len() as u64,
                media_type: media_type.to_owned(),
                published_question_revision_tuple: None,
                created_at: Timestamp::from_unix_millis(1),
            },
        }
    }

    #[test]
    fn native_publication_type_is_derived_from_compiled_source() {
        let native = publication_draft(
            QuestionBackend::Ple,
            QuestionFormat::PleQuestionJson,
            None,
            Some(QuestionType::MultipleChoice),
            false,
        );
        let native_bytes = include_bytes!(
            "../../adapters/ple/tests/fixtures/ple_question_json_single_choice.json"
        );
        let question_type = validated_native_publication_source(&native, native_bytes)
            .expect("valid source-derived Native Type");
        assert_eq!(question_type, QuestionType::MultipleChoice);

        let mut incorrect_type = native;
        incorrect_type.question_type = Some(QuestionType::Hotspot);
        assert!(matches!(
            validated_native_publication_source(&incorrect_type, native_bytes),
            Err(PublicationSourceValidationError::NativeTypeMismatch)
        ));
    }

    #[test]
    fn webwork_publication_requires_manual_type_and_keeps_correction_binding() {
        let missing_type = publication_draft(
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg,
            Some("Library/Genetics/exact.pg"),
            None,
            false,
        );
        assert_eq!(
            publication_source_binding(&missing_type),
            Err(PublicationSourceValidationError::MissingWebworkType)
        );

        let correction = publication_draft(
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPgml,
            Some("Library/Genetics/exact pgml.pgml"),
            Some(QuestionType::Hotspot),
            true,
        );
        assert!(
            correction
                .parent_published_question_revision_tuple
                .is_some()
        );
        assert_eq!(
            publication_source_binding(&correction),
            Ok(PublicationSourceBinding::Webwork {
                question_format: QuestionFormat::WebworkPgml,
                pg_path: "Library/Genetics/exact pgml.pgml",
                question_type: QuestionType::Hotspot,
            })
        );
    }

    #[test]
    fn unsupported_or_incoherent_publication_bindings_are_rejected() {
        let mut mismatched_media = publication_draft(
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPg,
            Some("Library/Algebra/item.pg"),
            Some(QuestionType::Numeric),
            false,
        );
        mismatched_media.source_record.media_type = PLE_QUESTION_JSON_MEDIA_TYPE.to_owned();
        assert_eq!(
            publication_source_binding(&mismatched_media),
            Err(PublicationSourceValidationError::InvalidBinding)
        );

        let unsafe_path = publication_draft(
            QuestionBackend::Webwork,
            QuestionFormat::WebworkPgml,
            Some("Library/../escape.pgml"),
            Some(QuestionType::Matching),
            false,
        );
        assert_eq!(
            publication_source_binding(&unsafe_path),
            Err(PublicationSourceValidationError::InvalidBinding)
        );

        let unsupported_backend = publication_draft(
            QuestionBackend::Imathas,
            QuestionFormat::Imathas,
            None,
            Some(QuestionType::MultipleChoice),
            false,
        );
        assert_eq!(
            publication_source_binding(&unsupported_backend),
            Err(PublicationSourceValidationError::InvalidBinding)
        );
    }

    #[tokio::test]
    async fn raw_draft_source_bytes_round_trip_with_registered_bindings() {
        let objects = MemoryObjectStore::default();
        let workspace = WorkspaceId::from_uuid(Uuid::from_u128(0x15));
        let cases = [
            (
                QuestionBackend::Ple,
                QuestionFormat::PleQuestionJson,
                None,
                PLE_QUESTION_JSON_MEDIA_TYPE,
                b"".as_slice(),
            ),
            (
                QuestionBackend::Ple,
                QuestionFormat::PleQuestionJson,
                None,
                PLE_QUESTION_JSON_MEDIA_TYPE,
                br#"{"response":{"kind":"hotspot"}"#.as_slice(),
            ),
            (
                QuestionBackend::Webwork,
                QuestionFormat::WebworkPg,
                Some("Library/Genetics/broken % source.pg"),
                "text/x-wework-pg",
                b"DOCUMENT();\nbroken regex: [\xff".as_slice(),
            ),
            (
                QuestionBackend::Webwork,
                QuestionFormat::WebworkPgml,
                Some("Library/Genetics/reviewed.pgml"),
                "text/x-wework-pg",
                b"[% incomplete".as_slice(),
            ),
        ];

        for (index, (backend, format, path, media_type, bytes)) in cases.into_iter().enumerate() {
            if backend == QuestionBackend::Ple {
                assert_eq!(
                    inspect_source(bytes, backend),
                    None,
                    "unfinished Native source has no derived binding"
                );
            }
            let source_record =
                put_workspace_source(&objects, workspace, bytes.to_vec(), media_type)
                    .await
                    .expect("raw source write");
            let draft = AuthoringDraft {
                draft_question_uuid: DraftQuestionUuid::from_uuid(Uuid::from_u128(
                    0x20 + index as u128,
                )),
                workspace,
                parent_published_question_revision_tuple: None,
                classification: None,
                edit_number: DraftQuestionEditNumber::new(1).expect("positive Edit Number"),
                metadata: QuestionMetadata {
                    question_title: String::new(),
                    question_description: String::new(),
                    tags: Vec::new(),
                    question_license: None,
                    question_citation: None,
                    language: None,
                },
                question_backend: backend,
                question_format: format,
                webwork_pg_path: path.map(str::to_owned),
                general_feedback: None,
                hint: None,
                worked_solution: None,
                authors: Vec::new(),
                question_type: None,
                source_record: source_record.clone(),
            };

            assert_eq!(draft.source_media_type(), Some(media_type));
            assert_eq!(
                load_verified_source(&objects, &draft)
                    .await
                    .unwrap()
                    .as_ref(),
                bytes
            );
            assert_eq!(source_record.media_type, media_type);
            assert_eq!(source_record.size_bytes, bytes.len() as u64);
            assert_eq!(source_record.sha256, Sha256Checksum::compute(bytes));

            let headers = source_response_headers(&draft).expect("valid source binding headers");
            assert_eq!(headers[CONTENT_TYPE], media_type);
            assert_eq!(headers["x-ple-question-backend"], backend.as_str());
            let expected_format = serde_json::to_value(format)
                .expect("format value")
                .as_str()
                .expect("format wire string")
                .to_owned();
            assert_eq!(headers["x-ple-question-format"], expected_format);
            assert_eq!(
                headers["x-ple-webwork-pg-path"],
                path.map(percent_encode_header_value).unwrap_or_default()
            );
        }
    }

    #[test]
    fn native_source_inspection_does_not_reject_unfinished_bytes() {
        for bytes in [b"".as_slice(), b"{".as_slice(), b"not JSON".as_slice()] {
            assert_eq!(inspect_source(bytes, QuestionBackend::Ple), None);
        }
    }

    #[test]
    fn hotspot_type_is_derived_without_admitting_its_image_reference() {
        let bytes = br#"{
            "format": "pleQuestionJson",
            "prompt": "Select the active site.",
            "response": {
                "kind": "hotspot",
                "surface": {
                    "questionImageAssetId": "00000000-0000-4000-8000-000000000001",
                    "checksum": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "description": "Protein structure"
                },
                "regions": [{
                    "id": "active-site",
                    "label": "Active site",
                    "x": 10,
                    "y": 10,
                    "width": 20,
                    "height": 20
                }],
                "correctRegions": ["active-site"]
            }
        }"#;
        assert_eq!(
            inspect_source(bytes, QuestionBackend::Ple),
            Some(QuestionType::Hotspot),
            "a valid Native HOTSPOT source keeps its Type even when storage has not admitted its image"
        );
    }

    #[test]
    fn draft_source_request_accepts_the_registered_media_type() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("TEXT/X-WEWORK-PG; charset=utf-8"),
        );
        assert!(matches_source_content_type(&headers, "text/x-wework-pg"));
        assert!(!matches_source_content_type(
            &headers,
            PLE_QUESTION_JSON_MEDIA_TYPE
        ));
    }
}
