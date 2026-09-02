use serde_json::json;
use std::sync::LazyLock;

use question_model::QuestionRendererVersion;

use super::*;

#[path = "tests/current_matching.rs"]
mod current_matching;

fn request() -> RenderRequest<'static> {
    static QUESTION_REVISION: LazyLock<question_model::QuestionRevisionReference> =
        LazyLock::new(|| question_model::QuestionRevisionReference {
            question_id: question_model::QuestionId::from_canonical_parts("ABCDEF", 'G')
                .expect("Question ID"),
            revision_number: question_model::QuestionRevisionNumber::new(7)
                .expect("positive revision"),
        });
    RenderRequest {
        pg_source: b"DOCUMENT();",
        pg_path: "Library/OPL/select-one.pg",
        question_revision: &QUESTION_REVISION,
        seed: 19,
    }
}

fn config() -> HttpWebworkRendererConfig {
    HttpWebworkRendererConfig::new(
        "http://webwork.internal/",
        Duration::from_secs(1),
        1024,
        QuestionRendererVersion {
            name: "webwork-pg-renderer".into(),
            version: "renderer-a06111-pg-726ff4".into(),
        },
    )
    .expect("recorded private configuration is valid")
}

fn radio_replay() -> WebworkQuestionAttemptReplayDetails {
    WebworkQuestionAttemptReplayDetails::SingleChoice {
        controls: [
            (
                opaque_choice_id(request(), 0).expect("fixed choice ID"),
                WebworkUpstreamControl {
                    field: "AnSwEr0001".into(),
                    value: "0".into(),
                },
            ),
            (
                opaque_choice_id(request(), 1).expect("fixed choice ID"),
                WebworkUpstreamControl {
                    field: "AnSwEr0001".into(),
                    value: "1".into(),
                },
            ),
        ]
        .into_iter()
        .collect(),
    }
}

const MATCHING_BODY: &str = r#"<div class="PGML">
Match each description with its functional group.
Note: Each choice will be used exactly once.<div style="margin-top:1em"></div><div class="two-column"><div>
<div class="d-inline text-nowrap" data-feedback-insert-element="AnSwEr0001" data-feedback-insert-method="append_content"><select aria-label="answer 1 " class="pg-select" id="AnSwEr0001" name="AnSwEr0001" size="1"><option class="tex2jax_ignore" disabled selected value="">?</option><option class="tex2jax_ignore" selected value=""></option><option class="tex2jax_ignore" value="A">A</option><option class="tex2jax_ignore" value="B">B</option></select></div> <strong>1.</strong> Can carry a positive charge<div style="margin-top:1em"></div><div class="d-inline text-nowrap" data-feedback-insert-element="AnSwEr0002" data-feedback-insert-method="append_content"><select aria-label="answer 2 " class="pg-select" id="AnSwEr0002" name="AnSwEr0002" size="1"><option class="tex2jax_ignore" disabled selected value="">?</option><option class="tex2jax_ignore" selected value=""></option><option class="tex2jax_ignore" value="A">A</option><option class="tex2jax_ignore" value="B">B</option></select></div> <strong>2.</strong> -H<sub>2</sub>PO<sub>4</sub><sup>-</sup>
</div><div class="right-col">
A. <span style="color: #003fff; font-weight:700;">Amino</span><div style="margin-top:1em"></div>B. <span style="color: #935d00; font-weight:700;">Phosphate</span>
</div></div>
</div>"#;

fn parsed_matching() -> ParsedRender {
    let settings = config();
    parse_render_rpc(
        Value::Object(response(&settings, MATCHING_BODY)),
        ExpectedEcho::from_request(&settings, request()),
        request(),
        &settings.base_uri,
    )
    .expect("recorded matching response is supported")
}

fn private_jwt() -> Map<String, Value> {
    serde_json::from_value(json!({
        "problem":"problem.payload.signature",
        "session":"header.payload.signature",
        "answer":"answer.payload.signature"
    }))
    .expect("recorded standalone JWT map")
}

fn rendered_document(body: &str, service_base: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><base href="{service_base}"><title>Question</title></head><body><form action="{service_base}render-api" id="problemMainForm"><div id="problem_body" class="problem-content" lang="en" dir="ltr">{body}</div><input name="sessionJWT" type="hidden" value="header.payload.signature"></form></body></html>"#
    )
}

fn response(_settings: &HttpWebworkRendererConfig, body: &str) -> Map<String, Value> {
    serde_json::from_value(json!({
        "JWT": private_jwt(),
        "debug": {"debug": [], "internal": [], "perl_warn": null, "pg_warn": []},
        "flags": {},
        "problem_result": {"errors": "", "msg": "", "score": 0.0, "type": "avg_problem_grader"},
        "problem_state": {},
        "renderedHTML": rendered_document(body, "http://webwork.internal/"),
        "resources": {"alias": {}, "assets": [], "regex": []}
    }))
    .expect("recorded external renderer response")
}

fn response_for_service(
    settings: &HttpWebworkRendererConfig,
    body: &str,
    service_base: &str,
    score: f64,
) -> Value {
    let origin = Url::parse(service_base)
        .expect("test service URL parses")
        .origin()
        .ascii_serialization();
    let mut value = response(settings, body);
    value.insert(
        "renderedHTML".into(),
        Value::String(rendered_document(body, &format!("{origin}/"))),
    );
    value
        .get_mut("problem_result")
        .and_then(Value::as_object_mut)
        .expect("fixture result object")
        .insert("score".into(), json!(score / 100.0));
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
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
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
    (format!("http://{address}/"), task)
}

async fn start_http_fixture_for(
    response_for: impl FnOnce(std::net::SocketAddr) -> String + Send + 'static,
) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener binds");
    let address = listener.local_addr().expect("test listener has address");
    let response = response_for(address);
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
            if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
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
        String::from_utf8(bytes).expect("test request is UTF-8")
    });
    (format!("http://{address}/"), task)
}

fn http_response(status: &str, content_type: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[test]
fn recorded_upstream_radio_result_becomes_answer_free_multiple_choice() {
    let settings = config();
    // WeBWorK 2.21 PGbasicmacros NAMED_ANS_RADIO emits a wrapping
    // label; parserRadioButtons wraps the group in this container.
    let value = Value::Object(response(
        &settings,
        r#"<p>Which molecule is water?</p><div class="radio-buttons-container" data-feedback-insert-element="1"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001"><strong>H<sub>2</sub>O</strong></label><div style="margin-bottom: 0.7em;"></div><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1"><strong>CO<sub>2</sub></strong></label></div>"#,
    ));
    let parsed = parse_render_rpc(
        value.clone(),
        ExpectedEcho::from_request(&settings, request()),
        request(),
        &settings.base_uri,
    )
    .expect("recorded response is supported");
    let QuestionResponseFormat::MultipleChoice { choices, selection } =
        &parsed.presentation.response
    else {
        panic!("single-choice Question Presentation")
    };
    assert_eq!(*selection, ResponseSelectionRule::ExactlyOne);
    assert_eq!(choices.len(), 2);
    let prompt = match &parsed.presentation.prompt[0] {
        QuestionContentBlock::Text { markdown } => markdown,
        _ => panic!("text prompt"),
    };
    assert!(prompt.contains("Which molecule is water?"));
    assert!(!prompt.contains("H2O") && !prompt.contains("CO2"));
    let WebworkQuestionAttemptReplayDetails::SingleChoice { controls } = &parsed.replay else {
        panic!("single-choice replay mapping")
    };
    assert_eq!(controls.len(), 2);
    let serialized = serde_json::to_string(&parsed.presentation)
        .expect("public Question Presentation serializes");
    assert!(!serialized.contains("AnSwEr0001"));
    assert!(!serialized.contains("header.payload.signature"));
}

#[test]
fn standalone_pgml_radio_shape_becomes_answer_free_multiple_choice() {
    let settings = config();
    let value = Value::Object(response(
        &settings,
        r#"<div class="PGML">
Based on their molecular formula, which compound is most likely <span style="color:#997300;font-size:1.25em;font-weight:700;">hydrophobic</span>?
<div style="margin-top:1em"></div>
<label><input TYPE="RADIO" name="AnSwEr0001" id="AnSwEr0001" aria-label="answer 1 option 1 " value="B0"><strong>A</strong>. glucose, C<sub>6</sub>H<sub>12</sub>O<sub>6</sub></label><div style="margin-bottom: 0.7em;"></div><label><input TYPE="RADIO" name="AnSwEr0001" id="AnSwEr0001_1" aria-label="answer 1 option 2 " value="B1"><strong>B</strong>. <span style="color: #6c6c00; font-weight:700;">benzene</span>, C<sub>6</sub>H<sub>6</sub></label>
<div style="margin-top:1em"></div>
</div>"#,
    ));
    let parsed = parse_render_rpc(
        value.clone(),
        ExpectedEcho::from_request(&settings, request()),
        request(),
        &settings.base_uri,
    )
    .expect("standalone PGML RadioButtons output is supported");
    let QuestionResponseFormat::MultipleChoice { choices, selection } =
        &parsed.presentation.response
    else {
        panic!("single-choice Question Presentation")
    };
    assert_eq!(*selection, ResponseSelectionRule::ExactlyOne);
    assert_eq!(choices.len(), 2);
    assert!(matches!(
        &choices[1].body[0],
        QuestionContentBlock::Text { markdown } if markdown.contains("B. benzene")
    ));

    let hostile_label_style = value
        .as_object()
        .expect("recorded response object")
        .get("renderedHTML")
        .and_then(Value::as_str)
        .expect("recorded HTML")
        .replace(
            "color: #6c6c00; font-weight:700;",
            "background-image:url(https://attacker.example/)",
        );
    let mut hostile = response(&settings, "<p>placeholder</p>");
    hostile.insert("renderedHTML".into(), Value::String(hostile_label_style));
    assert!(
        parse_render_rpc(
            Value::Object(hostile),
            ExpectedEcho::from_request(&settings, request()),
            request(),
            &settings.base_uri,
        )
        .is_err()
    );
}

#[test]
fn recorded_upstream_matching_result_becomes_answer_free_typed_matching() {
    let parsed = parsed_matching();
    let QuestionResponseFormat::Matching { prompts, choices } = &parsed.presentation.response
    else {
        panic!("typed matching Question Presentation")
    };
    assert_eq!(prompts.len(), 2);
    assert_eq!(choices.len(), 2);
    assert_eq!(
        prompts[0].body,
        vec![QuestionContentBlock::Text {
            markdown: "Can carry a positive charge".into()
        }]
    );
    assert_eq!(
        prompts[1].body,
        vec![QuestionContentBlock::Text {
            markdown: "-H2PO4-".into()
        }]
    );
    assert_eq!(
        choices[0].body,
        vec![QuestionContentBlock::Text {
            markdown: "Amino".into()
        }]
    );
    assert_eq!(
        choices[1].body,
        vec![QuestionContentBlock::Text {
            markdown: "Phosphate".into()
        }]
    );
    let WebworkQuestionAttemptReplayDetails::Matching { prompts: replay } = &parsed.replay else {
        panic!("matching replay mapping")
    };
    assert_eq!(replay.len(), 2);
    assert!(replay.values().all(|prompt| prompt.choices.len() == 2));
    let public = serde_json::to_string(&parsed.presentation)
        .expect("public Question Presentation serializes");
    for protected in ["AnSwEr0001", "AnSwEr0002", "header.payload.signature"] {
        assert!(!public.contains(protected));
    }
}

#[test]
fn matching_refuses_mismatched_options_and_hostile_markup() {
    let settings = config();
    for body in [
        MATCHING_BODY.replacen("value=\"B\">B", "value=\"C\">C", 1),
        MATCHING_BODY.replacen("<span style=", "<script></script><span style=", 1),
        MATCHING_BODY.replacen("name=\"AnSwEr0002\"", "name=\"passwd\"", 1),
    ] {
        assert!(
            parse_render_rpc(
                Value::Object(response(&settings, &body)),
                ExpectedEcho::from_request(&settings, request()),
                request(),
                &settings.base_uri,
            )
            .is_err()
        );
    }
}

#[test]
fn private_jwt_mismatch_and_protected_top_level_data_refuse() {
    let settings = config();
    let expected = ExpectedEcho::from_request(&settings, request());
    let mut mismatch = response(&settings, "<p>ignored</p>");
    mismatch.insert(
        "JWT".into(),
        json!({"problem":"problem.payload.signature", "session":"header.payload.signature", "answer":"answer.payload.signature", "unexpected":"value"}),
    );
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
fn config_debug_contains_no_credential_surface() {
    let config = config();
    let debug = format!("{config:?}");
    assert!(debug.contains("webwork-pg-renderer"));
    assert!(!debug.contains("not-in-browser") && !debug.contains("[REDACTED]"));
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
    let html = off_origin
        .get("renderedHTML")
        .and_then(Value::as_str)
        .expect("fixture HTML")
        .replace("http://webwork.internal/", "https://attacker.example/");
    off_origin.insert("renderedHTML".into(), Value::String(html));
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
fn oversized_or_malformed_renderer_session_token_refuses() {
    let reviewed_pg_size = serde_json::from_value(json!({
        "problem":"problem.payload.signature",
        "session": format!("header.{}.signature", "x".repeat(84_000)),
        "answer": format!("answer.{}.signature", "x".repeat(114_000))
    }))
    .expect("reviewed-PG-size JWT map");
    assert!(validate_and_discard_jwt(&reviewed_pg_size).is_ok());
    let oversized = serde_json::from_value(json!({
        "problem":"problem.payload.signature",
        "session": format!("header.{}.signature", "x".repeat(MAX_PRIVATE_JWT_BYTES + 1)),
        "answer":"answer.payload.signature"
    }))
    .expect("oversized JWT map");
    assert!(validate_and_discard_jwt(&oversized).is_err());
    let mut malformed = private_jwt();
    malformed.insert("session".to_string(), Value::Bool(true));
    assert!(validate_and_discard_jwt(&malformed).is_err());
    let invalid_shape = serde_json::from_value(json!({
        "problem":"problem.payload.signature",
        "session":"only.two",
        "answer":"answer.payload.signature"
    }))
    .expect("invalid JWT map");
    assert!(validate_and_discard_jwt(&invalid_shape).is_err());
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
    missing.remove("resources");
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
    let html = wrong_form
        .get("renderedHTML")
        .and_then(Value::as_str)
        .expect("fixture HTML")
        .replace(
            "http://webwork.internal/render-api",
            "http://webwork.internal/",
        );
    wrong_form.insert("renderedHTML".into(), Value::String(html));
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
        r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a"><strong class="renderer-owned">A</strong></label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#,
        r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a"><span>A</span></label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#,
        r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a">A</label><div style="margin-bottom: 2em;"></div><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#,
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
        "header.payload.signature"
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
    let session_key_leaked = r#"<p>header.payload.signature</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr1" value="0" id="a">A</label><label><input type="radio" name="AnSwEr1" value="1" id="b">B</label></div>"#;
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
#[ignore = "opt-in loopback HTTP transport acceptance"]
async fn private_http_client_posts_only_to_render_api_with_form_fields() {
    let body = r#"{"score":0}"#;
    let (base, task) = start_http_fixture(http_response("200 OK", "application/json", body)).await;
    let settings = HttpWebworkRendererConfig::new(
        &base,
        Duration::from_secs(1),
        1024,
        QuestionRendererVersion {
            name: "test".into(),
            version: "1".into(),
        },
    )
    .expect("fixture config");
    let renderer = HttpWebworkRenderer::new(settings).expect("fixture client");
    let result = renderer
        .rpc(BTreeMap::from([("problemSeed".into(), "19".into())]))
        .await;
    assert_eq!(result.expect("fixture JSON")["score"], 0);
    let request = task.await.expect("fixture task completes");
    assert!(request.starts_with("POST /render-api HTTP/1.1\r\n"));
    let lower = request.to_ascii_lowercase();
    assert!(lower.contains("content-type: application/x-www-form-urlencoded"));
    assert!(request.contains("problemSeed=19"));
    assert!(!request.contains("courseID=") && !request.contains("passwd="));
    assert!(!request.contains("/v1/"));
}

#[tokio::test]
#[ignore = "opt-in loopback HTTP transport acceptance"]
async fn private_http_client_refuses_redirect_non_json_and_oversized_responses() {
    let redirect =
        "HTTP/1.1 302 Found\r\nlocation: /login\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
    let (base, task) = start_http_fixture(redirect.into()).await;
    let renderer = HttpWebworkRenderer::new(
        HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            16_384,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
        )
        .expect("fixture config"),
    )
    .expect("fixture client");
    assert!(matches!(
        renderer.rpc(BTreeMap::new()).await,
        Err(RendererFailure::InvalidOutput(_))
    ));
    let _ = task.await.expect("fixture task completes");

    let (base, task) = start_http_fixture(http_response("200 OK", "text/html", "not json")).await;
    let renderer = HttpWebworkRenderer::new(
        HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            1024,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
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
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
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
#[ignore = "opt-in loopback HTTP transport acceptance"]
async fn grade_submits_only_the_persisted_selected_upstream_radio_value() {
    const BODY: &str = r#"<p>Which molecule is water?</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">H2O</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">CO2</label></div>"#;
    const GRADED_BODY: &str =
        r#"<div class="ResultsWithoutAnswer"><span>Answer recorded.</span></div>"#;
    let (base, task) = start_http_fixture_for(|address| {
        let base = format!("http://{address}/");
        let settings = HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            1024,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
        )
        .expect("fixture config");
        let graded = response_for_service(&settings, GRADED_BODY, &base, 100.0);
        http_response("200 OK", "application/json", &graded.to_string())
    })
    .await;
    let settings = HttpWebworkRendererConfig::new(
        &base,
        Duration::from_secs(1),
        16_384,
        QuestionRendererVersion {
            name: "test".into(),
            version: "1".into(),
        },
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
            question_revision: request().question_revision,
            seed: request().seed,
            response: &student_response,
            replay: &radio_replay(),
            points_possible: 7.0,
            partial_credit: false,
        })
        .await
        .expect("100 percent answer grades");
    assert!(matches!(
        result,
        QuestionGradingOutcome::Graded(GradingResult {
            correct: true,
            points_earned: 7.0,
            points_possible: 7.0,
        })
    ));
    let wire_request = task.await.expect("fixture task completes");
    assert!(wire_request.starts_with("POST /render-api HTTP/1.1\r\n"));
    assert!(wire_request.contains("submitAnswers=1"));
    assert!(wire_request.contains("AnSwEr0001=1"));
    assert!(!wire_request.contains("AnSwEr0001=0"));

    let settings = config();
    let parsed = parse_render_rpc(
        response_for_service(&settings, BODY, "http://webwork.internal/", 0.0),
        ExpectedEcho::from_request(&settings, request()),
        request(),
        &settings.base_uri,
    )
    .expect("accepted result is safe to cache");
    let public = serde_json::to_string(&parsed.presentation)
        .expect("public Question Presentation serializes");
    for protected in ["AnSwEr0001", "header.payload.signature", "RE9DVU1FTlQoKTs="] {
        assert!(!public.contains(protected));
    }
}

#[tokio::test]
#[ignore = "opt-in loopback HTTP transport acceptance"]
async fn grade_refuses_fractional_upstream_score() {
    const BODY: &str = r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">A</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">B</label></div>"#;
    let (base, task) = start_http_fixture_for(|address| {
        let base = format!("http://{address}/");
        let settings = HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            16_384,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
        )
        .expect("fixture config");
        let fractional = response_for_service(&settings, BODY, &base, 50.0);
        http_response("200 OK", "application/json", &fractional.to_string())
    })
    .await;
    let settings = HttpWebworkRendererConfig::new(
        &base,
        Duration::from_secs(1),
        16_384,
        QuestionRendererVersion {
            name: "test".into(),
            version: "1".into(),
        },
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
                question_revision: request().question_revision,
                seed: request().seed,
                response: &response,
                replay: &radio_replay(),
                points_possible: 7.0,
                partial_credit: false,
            })
            .await,
        Err(RendererFailure::InvalidOutput(_))
    ));
    let _ = task.await.expect("fixture task completes");
}

#[tokio::test]
#[ignore = "opt-in loopback HTTP transport acceptance"]
async fn grade_maps_zero_percent_to_zero_earned_points() {
    const BODY: &str = r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">A</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">B</label></div>"#;
    let (base, task) = start_http_fixture_for(|address| {
        let base = format!("http://{address}/");
        let settings = HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            16_384,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
        )
        .expect("fixture config");
        let incorrect = response_for_service(&settings, BODY, &base, 0.0);
        http_response("200 OK", "application/json", &incorrect.to_string())
    })
    .await;
    let renderer = HttpWebworkRenderer::new(
        HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            16_384,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
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
            question_revision: request().question_revision,
            seed: request().seed,
            response: &response,
            replay: &radio_replay(),
            points_possible: 7.0,
            partial_credit: false,
        })
        .await
        .expect("zero percent answer grades");
    assert!(matches!(
        result,
        QuestionGradingOutcome::Graded(GradingResult {
            correct: false,
            points_earned: 0.0,
            points_possible: 7.0,
        })
    ));
    let _ = task.await.expect("fixture task completes");
}

#[tokio::test]
#[ignore = "opt-in loopback HTTP transport acceptance"]
async fn matching_grade_is_one_private_call_and_maps_fractional_credit() {
    const GRADED_BODY: &str =
        r#"<div class="ResultsWithoutAnswer"><span>Answer recorded.</span></div>"#;
    let (base, task) = start_http_fixture_for(|address| {
        let base = format!("http://{address}/");
        let settings = HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            16_384,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
        )
        .expect("fixture config");
        let graded = response_for_service(&settings, GRADED_BODY, &base, 50.0);
        http_response("200 OK", "application/json", &graded.to_string())
    })
    .await;
    let renderer = HttpWebworkRenderer::new(
        HttpWebworkRendererConfig::new(
            &base,
            Duration::from_secs(1),
            16_384,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
        )
        .expect("fixture config"),
    )
    .expect("fixture client");
    let parsed = parsed_matching();
    let QuestionResponseFormat::Matching { prompts, choices } = &parsed.presentation.response
    else {
        panic!("typed matching Question Presentation")
    };
    let response = StudentResponse::Matching {
        matches: vec![
            question_model::response::StudentMatch {
                prompt: prompts[0].id.clone(),
                choice: choices[1].id.clone(),
            },
            question_model::response::StudentMatch {
                prompt: prompts[1].id.clone(),
                choice: choices[0].id.clone(),
            },
        ],
    };
    let result = renderer
        .grade(GradeRequest {
            pg_source: request().pg_source,
            pg_path: request().pg_path,
            question_revision: request().question_revision,
            seed: request().seed,
            response: &response,
            replay: &parsed.replay,
            points_possible: 8.0,
            partial_credit: true,
        })
        .await
        .expect("fractional matching response grades");
    assert!(matches!(
        result,
        QuestionGradingOutcome::Graded(GradingResult {
            correct: false,
            points_earned: 4.0,
            points_possible: 8.0,
        })
    ));
    let request = task.await.expect("fixture task completes");
    assert!(request.contains("submitAnswers=1"));
    assert!(request.contains("AnSwEr0001=B"));
    assert!(request.contains("AnSwEr0002=A"));
}

#[tokio::test]
#[ignore = "opt-in loopback HTTP timeout acceptance"]
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
            &format!("http://{address}/"),
            Duration::from_millis(10),
            1024,
            QuestionRendererVersion {
                name: "test".into(),
                version: "1".into(),
            },
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
