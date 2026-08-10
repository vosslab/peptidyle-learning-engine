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

fn radio_replay() -> WebworkReplayMappingV1 {
    WebworkReplayMappingV1::SingleChoice {
        controls: [
            (
                opaque_choice_id(request(), 0).expect("fixed choice ID"),
                UpstreamControlV1 {
                    field: "AnSwEr0001".into(),
                    value: "0".into(),
                },
            ),
            (
                opaque_choice_id(request(), 1).expect("fixed choice ID"),
                UpstreamControlV1 {
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

fn hidden(settings: &HttpWebworkRendererConfig) -> Map<String, Value> {
    let expected = ExpectedEcho::from_request(settings, request());
    serde_json::from_value(json!({
        "sourceFilePath":"", "problemSource": expected.source, "problemSeed": expected.seed,
        "problemUUID":"", "psvn":54321, "pathToProblemFile": expected.file,
        "courseID": expected.course_id, "user": expected.user, "passwd":"",
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
    (format!("http://{address}/webwork2/"), task)
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
    (format!("http://{address}/webwork2/"), task)
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
    let WebworkReplayMappingV1::SingleChoice { controls } = &parsed.replay else {
        panic!("single-choice replay mapping")
    };
    assert_eq!(controls.len(), 2);
    let serialized = serde_json::to_string(&parsed.envelope).expect("public envelope serializes");
    assert!(!serialized.contains("AnSwEr0001"));
    assert!(!serialized.contains("not-in-browser"));
    assert!(!serialized.contains("upstream-session-key"));
    assert!(!parsed.html.contains("AnSwEr0001"));
    assert!(!parsed.html.contains("upstream-session-key"));
}

#[test]
fn recorded_upstream_matching_result_becomes_answer_free_typed_matching() {
    let parsed = parsed_matching();
    let ResponseDefinition::Matching { prompts, choices } = &parsed.envelope.response else {
        panic!("typed matching envelope")
    };
    assert_eq!(prompts.len(), 2);
    assert_eq!(choices.len(), 2);
    assert_eq!(
        prompts[0].body,
        vec![ContentBlock::Text {
            markdown: "Can carry a positive charge".into()
        }]
    );
    assert_eq!(
        prompts[1].body,
        vec![ContentBlock::Text {
            markdown: "-H2PO4-".into()
        }]
    );
    assert_eq!(
        choices[0].body,
        vec![ContentBlock::Text {
            markdown: "Amino".into()
        }]
    );
    assert_eq!(
        choices[1].body,
        vec![ContentBlock::Text {
            markdown: "Phosphate".into()
        }]
    );
    let WebworkReplayMappingV1::Matching { prompts: replay } = &parsed.replay else {
        panic!("matching replay mapping")
    };
    assert_eq!(replay.len(), 2);
    assert!(replay.values().all(|prompt| prompt.choices.len() == 2));
    let public = serde_json::to_string(&parsed.envelope).expect("public envelope serializes");
    for protected in ["AnSwEr0001", "AnSwEr0002", "upstream-session-key"] {
        assert!(!public.contains(protected));
        assert!(!parsed.html.contains(protected));
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
fn hidden_credential_mismatch_and_protected_top_level_data_refuse() {
    let settings = config();
    let expected = ExpectedEcho::from_request(&settings, request());
    let mut mismatch = response(&settings, "<p>ignored</p>");
    mismatch.insert("hidden_input_field".into(), json!({"sourceFilePath":"", "problemSource":"", "problemSeed":"", "problemUUID":"", "psvn":54321, "pathToProblemFile":"", "courseID":"", "user":"", "passwd":"attacker", "displayMode":"MathJax", "key":"", "outputformat":"json", "theme":"", "language":"", "showSummary":"0", "showHints":"0", "showSolutions":"0", "showPreviewButton":"0", "showCheckAnswersButton":"0", "showCorrectAnswersButton":"0", "showFooter":"0", "extraHeaderText":""}));
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
    let mut invalid_psvn = hidden(&settings);
    invalid_psvn.insert("psvn".to_string(), Value::String("54321".into()));
    assert!(
        validate_and_discard_hidden(
            &invalid_psvn,
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
    let (base, task) = start_http_fixture(http_response("200 OK", "application/json", body)).await;
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
    let redirect =
        "HTTP/1.1 302 Found\r\nlocation: /login\r\ncontent-length: 0\r\nconnection: close\r\n\r\n";
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

    let (base, task) = start_http_fixture(http_response("200 OK", "text/html", "not json")).await;
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
async fn grade_submits_only_the_persisted_selected_upstream_radio_value() {
    const BODY: &str = r#"<p>Which molecule is water?</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">H2O</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">CO2</label></div>"#;
    const GRADED_BODY: &str =
        r#"<div class="ResultsWithoutAnswer"><span>Answer recorded.</span></div>"#;
    let (base, task) = start_http_fixture_for(|address| {
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
        let graded = response_for_service(&settings, GRADED_BODY, &base, 100.0);
        http_response("200 OK", "application/json", &graded.to_string())
    })
    .await;
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
            replay: &radio_replay(),
            points_possible: 7.0,
            partial_credit: false,
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
    let wire_request = task.await.expect("fixture task completes");
    assert!(wire_request.starts_with("POST /webwork2/render_rpc HTTP/1.1\r\n"));
    assert!(wire_request.contains("WWsubmit=1"));
    assert!(wire_request.contains("AnSwEr0001=1"));
    assert!(!wire_request.contains("AnSwEr0001=0"));

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
    let (base, task) = start_http_fixture_for(|address| {
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
        let fractional = response_for_service(&settings, BODY, &base, 50.0);
        http_response("200 OK", "application/json", &fractional.to_string())
    })
    .await;
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
async fn grade_maps_zero_percent_to_zero_earned_points() {
    const BODY: &str = r#"<p>Question</p><div class="radio-buttons-container"><label><input type="radio" name="AnSwEr0001" value="0" id="AnSwEr0001">A</label><label><input type="radio" name="AnSwEr0001" value="1" id="AnSwEr0001_1">B</label></div>"#;
    let (base, task) = start_http_fixture_for(|address| {
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
        let incorrect = response_for_service(&settings, BODY, &base, 0.0);
        http_response("200 OK", "application/json", &incorrect.to_string())
    })
    .await;
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
            replay: &radio_replay(),
            points_possible: 7.0,
            partial_credit: false,
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
    let _ = task.await.expect("fixture task completes");
}

#[tokio::test]
async fn matching_grade_is_one_private_call_and_maps_fractional_credit() {
    const GRADED_BODY: &str =
        r#"<div class="ResultsWithoutAnswer"><span>Answer recorded.</span></div>"#;
    let (base, task) = start_http_fixture_for(|address| {
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
        let graded = response_for_service(&settings, GRADED_BODY, &base, 50.0);
        http_response("200 OK", "application/json", &graded.to_string())
    })
    .await;
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
    let parsed = parsed_matching();
    let ResponseDefinition::Matching { prompts, choices } = &parsed.envelope.response else {
        panic!("typed matching envelope")
    };
    let response = StudentResponse::Matching {
        matches: vec![
            question_model::response::MatchPair {
                prompt: prompts[0].id.clone(),
                choice: choices[1].id.clone(),
            },
            question_model::response::MatchPair {
                prompt: prompts[1].id.clone(),
                choice: choices[0].id.clone(),
            },
        ],
    };
    let result = renderer
        .grade(GradeRequest {
            pg_source: request().pg_source,
            pg_path: request().pg_path,
            version: request().version,
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
        GradeOutcome::Graded(AttemptResult {
            correct: false,
            points_earned: 4.0,
            points_possible: 8.0,
        })
    ));
    let request = task.await.expect("fixture task completes");
    assert!(request.contains("WWsubmit=1"));
    assert!(request.contains("AnSwEr0001=B"));
    assert!(request.contains("AnSwEr0002=A"));
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
