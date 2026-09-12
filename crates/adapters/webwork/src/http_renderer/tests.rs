use serde_json::{Map, Value, json};
use std::sync::LazyLock;

use super::*;

fn renderer_version() -> QuestionRendererVersion {
    QuestionRendererVersion {
        name: "webwork-pg-renderer".into(),
        version: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
    }
}

fn config() -> HttpWebworkRendererConfig {
    HttpWebworkRendererConfig::new(
        "http://webwork.internal/",
        Duration::from_secs(1),
        1024,
        renderer_version(),
        "https://ple.example/",
    )
    .expect("valid deployment settings")
}

fn request() -> RenderRequest<'static> {
    static QUESTION_REVISION: LazyLock<question_model::QuestionRevisionReference> =
        LazyLock::new(|| question_model::QuestionRevisionReference {
            question_id: question_model::QuestionId::from_canonical_parts("ABCDEF", 'G')
                .expect("fixed Question ID is valid"),
            revision_number: question_model::QuestionRevisionNumber::new(1)
                .expect("fixed Question Revision Number is valid"),
        });
    RenderRequest {
        pg_source: b"DOCUMENT();",
        pg_path: "Library/opaque.pg",
        question_revision: &QUESTION_REVISION,
        seed: 7,
    }
}

fn envelope(html: &str, score: f64) -> Value {
    json!({
        "JWT": {"problem":"problem.token.value", "session":"session.token.value", "answer":"answer.token.value"},
        "debug": {}, "flags": {}, "problem_result": {"score": score},
        "problem_state": {}, "renderedHTML": html, "resources": {},
    })
}

#[test]
/// Prevents a renderer document or identity from being altered before backend-owned presentation reaches PLE.
fn render_retains_the_exact_backend_document_and_identity() {
    let document = "<!doctype html><form><input name=\"AnSwEr0001\"></form>";
    let rendered = validate_render_rpc(envelope(document, 0.0)).expect("closed renderer envelope");
    let result = RenderedWebworkQuestion::from_document(rendered, renderer_version());
    assert_eq!(result.document, document.as_bytes());
    assert_eq!(result.lifecycle_state.as_deref(), None);
    assert_eq!(result.renderer_version, renderer_version());
}

#[test]
/// Prevents the renderer's private JWT state from becoming durable browser-visible document bytes.
fn render_refuses_exact_private_jwt_reflection() {
    let ordinary_document =
        "<form><input name=\"AnSwEr0001\" value=\"problem.token.different\"></form>";
    assert_eq!(
        validate_render_rpc(envelope(ordinary_document, 0.0)).expect("credential-free document"),
        ordinary_document.as_bytes()
    );

    for private_jwt in [
        "problem.token.value",
        "session.token.value",
        "answer.token.value",
    ] {
        let reflected_document = format!("<p>{private_jwt}</p>");
        assert_eq!(
            validate_render_rpc(envelope(&reflected_document, 0.0)),
            Err(RendererFailure::InvalidOutput(
                "renderer HTML contained private JWT state".into()
            ))
        );
    }
}

#[test]
/// Prevents form submissions from losing PG-required order or repeated field values before grading.
fn response_pairs_preserve_order_and_duplicates() {
    let payload = br#"[["AnSwEr0001","A"],["AnSwEr0001","B"],["ordinary","value"]]"#;
    assert_eq!(
        decode_response_pairs(payload).expect("canonical pair array"),
        vec![
            ("AnSwEr0001".into(), "A".into()),
            ("AnSwEr0001".into(), "B".into()),
            ("ordinary".into(), "value".into()),
        ]
    );
}

#[test]
/// Prevents browser-owned payloads from bypassing the canonical bounded pair-array boundary.
fn response_pairs_refuse_noncanonical_and_server_owned_names() {
    assert!(decode_response_pairs(br#"[["answer","x"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["problemSource","DOCUMENT();"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["problemSeed","9"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["isInstructor","1"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["showSolutions","1"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["problemJWT","x"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["PrObLeMjWt","x"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["outputFormat","html"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["pleOrigin","https://elsewhere.example"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["showCorrectAnswersPreview","1"]]"#).is_err());
    assert!(decode_response_pairs(br#"[["sHoWcOrReCtAnSwErSPreview","1"]]"#).is_err());
    assert_eq!(
        decode_response_pairs(br#"[["showCorrectAnswer","1"]]"#).expect("ordinary field"),
        vec![("showCorrectAnswer".into(), "1".into())]
    );
    assert!(decode_response_pairs(br#"[ ["AnSwEr1","x"] ]"#).is_err());
    assert_eq!(
        decode_response_pairs(br#"{"AnSwEr0001":"A"}"#),
        Err(RendererFailure::InvalidOutput(
            "backend-owned response is not a pair array".into()
        ))
    );
    let oversized = format!(
        r#"[["AnSwEr0001","{}"]]"#,
        "x".repeat(MAX_RESPONSE_PAYLOAD_BYTES)
    );
    assert!(oversized.len() > MAX_RESPONSE_PAYLOAD_BYTES);
    assert_eq!(
        decode_response_pairs(oversized.as_bytes()),
        Err(RendererFailure::InvalidOutput(
            "backend-owned response exceeds the supported bound".into()
        ))
    );
}

#[test]
/// Prevents transport configuration from drifting away from the generic embed and deployment boundary.
fn protocol_uses_embed_format_and_deployment_owned_urls() {
    let settings = config();
    let revision = question_model::QuestionRevisionReference {
        question_id: question_model::QuestionId::from_canonical_parts("ABCDEF", 'G').unwrap(),
        revision_number: question_model::QuestionRevisionNumber::new(1).unwrap(),
    };
    let fields = super::super::protocol::render_fields(
        RenderRequest {
            pg_source: b"DOCUMENT();",
            pg_path: "Library/a.pg",
            question_revision: &revision,
            seed: 7,
        },
        &settings.ple_origin,
        &settings.ple_asset_base,
    );
    assert!(fields.contains(&("outputFormat".into(), "ple_embed".into())));
    assert!(fields.contains(&("pleOrigin".into(), "https://ple.example".into())));
    assert!(fields.contains(&(
        "pleAssetBase".into(),
        "https://ple.example/api/webwork-assets".into()
    )));
}

#[test]
/// Prevents malformed renderer output from becoming a trusted document or grade outcome.
fn malformed_envelopes_and_scores_refuse() {
    let mut unknown = envelope("<p>Question</p>", 1.0)
        .as_object()
        .unwrap()
        .clone();
    unknown.insert("unexpected".into(), Value::Null);
    assert!(validate_render_rpc(Value::Object(unknown)).is_err());
    assert!(validate_grade_rpc(envelope("<p>Question</p>", 1.1)).is_err());
    let mut malformed_jwt: Map<String, Value> = envelope("<p>Question</p>", 1.0)
        .as_object()
        .unwrap()
        .clone();
    malformed_jwt.insert("JWT".into(), json!({"session":"x"}));
    assert!(validate_render_rpc(Value::Object(malformed_jwt)).is_err());
    assert!(parse_json_without_duplicates(br#"{"JWT":{},"JWT":{}}"#).is_err());
}

#[test]
/// Prevents renderer outages from being mistaken for valid grading results.
fn transport_statuses_refuse_by_backend_failure_class() {
    assert_eq!(
        map_status(reqwest::StatusCode::GATEWAY_TIMEOUT),
        Err(RendererFailure::TimedOut)
    );
    assert_eq!(
        map_status(reqwest::StatusCode::SERVICE_UNAVAILABLE),
        Err(RendererFailure::Unavailable)
    );
    assert_eq!(
        map_status(reqwest::StatusCode::TOO_MANY_REQUESTS),
        Err(RendererFailure::ResourceExhausted)
    );
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
            let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers.lines().find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
            });
            if content_length.is_none_or(|length| bytes.len() >= header_end + 4 + length) {
                break;
            }
        }
        stream
            .write_all(response.as_bytes())
            .await
            .expect("test response writes");
        String::from_utf8(bytes).expect("renderer request is UTF-8")
    });
    (format!("http://{address}/"), task)
}

fn http_response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[tokio::test]
/// Prevents redirects, non-JSON, and oversized renderer replies from crossing the opaque transport boundary.
async fn transport_refuses_redirect_non_json_and_oversized_responses() {
    let cases = [
        (
            "HTTP/1.1 302 Found\r\nlocation: /login\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
                .to_string(),
            RendererFailure::InvalidOutput("renderer redirected request".to_string()),
        ),
        (
            http_response("200 OK", "text/html", "not JSON"),
            RendererFailure::InvalidOutput("renderer response was not JSON".to_string()),
        ),
        (
            http_response("200 OK", "application/json", &"x".repeat(1025)),
            RendererFailure::ResourceExhausted,
        ),
        (
            http_response(
                "200 OK",
                "application/json",
                &json!({"error": "renderer rejected the request"}).to_string(),
            ),
            RendererFailure::InvalidOutput("renderer rejected trusted request".to_string()),
        ),
    ];
    for (response, expected) in cases {
        let (base, task) = start_http_fixture(response).await;
        let renderer = HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                1024,
                renderer_version(),
                "https://ple.example/",
            )
            .expect("loopback deployment settings are valid"),
        )
        .expect("loopback renderer client builds");

        assert_eq!(renderer.rpc(Vec::new()).await, Err(expected));
        let _ = task.await.expect("fixture completes");
    }
}

#[tokio::test]
/// Prevents PLE from changing ordered PG answers or the renderer's normalized score semantics.
async fn grade_forwards_ordered_pairs_once_with_trusted_fields() {
    for (renderer_score, expected_correct, expected_credit) in
        [(0.0, false, 0.0), (0.5, false, 0.5), (1.0, true, 1.0)]
    {
        let response = envelope("<p>Question</p>", renderer_score).to_string();
        let (base, task) =
            start_http_fixture(http_response("200 OK", "application/json", &response)).await;
        let renderer = HttpWebworkRenderer::new(
            HttpWebworkRendererConfig::new(
                &base,
                Duration::from_secs(1),
                16_384,
                renderer_version(),
                "https://ple.example/",
            )
            .expect("loopback deployment settings are valid"),
        )
        .expect("loopback renderer client builds");
        let request = request();

        let outcome = renderer
            .grade(GradeRequest {
                pg_source: request.pg_source,
                pg_path: request.pg_path,
                question_revision: request.question_revision,
                seed: request.seed,
                response_payload:
                    br#"[["AnSwEr0001","A"],["AnSwEr0001","B"],["ordinary","value"]]"#,
                lifecycle_state: &question_model::BackendOwnedLifecycleState::none(),
            })
            .await
            .expect("bounded renderer score grades");
        assert!(matches!(
            outcome,
            grading::QuestionGradingOutcome::Evaluated(result)
                if result.correct() == expected_correct
                    && result.normalized_credit() == expected_credit
        ));

        let wire_request = task.await.expect("fixture completes");
        assert!(wire_request.starts_with("POST /render-api HTTP/1.1\r\n"));
        let body = wire_request
            .split("\r\n\r\n")
            .nth(1)
            .expect("request has form body");
        let pairs: Vec<_> = body.split('&').collect();
        let first_answer = pairs
            .iter()
            .position(|pair| *pair == "AnSwEr0001=A")
            .expect("first opaque pair forwards");
        let second_answer = pairs
            .iter()
            .position(|pair| *pair == "AnSwEr0001=B")
            .expect("duplicate opaque pair forwards");
        assert!(first_answer < second_answer);
        assert!(pairs.contains(&"ordinary=value"));
        assert_eq!(pairs.last(), Some(&"submitAnswers=1"));
        assert!(pairs.contains(&"outputFormat=ple_embed"));
        assert!(pairs.contains(&"problemSeed=7"));
    }
}
