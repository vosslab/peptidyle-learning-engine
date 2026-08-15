use super::*;

const FLAT_MATCHING_V2_SOURCE: &str = r#"{
  "format":"pleFlatQuestion",
  "version":2,
  "title":"Match inheritance terms",
  "prompt":"Match each term to its description.",
  "response":{
    "kind":"matching",
    "prompts":[
      {"id":"p1","text":"Two different alleles"},
      {"id":"p2","text":"Two identical alleles"}
    ],
    "choices":[
      {"id":"c1","text":"Heterozygous"},
      {"id":"c2","text":"Homozygous"}
    ],
    "matches":[
      {"prompt":"p1","choice":"c1"},
      {"prompt":"p2","choice":"c2"}
    ]
  },
  "feedback":{"correct":"Correct.","incorrect":"Review the allele pairs."},
  "points":2.0,
  "attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},
  "timingPolicy":{"kind":"untimed"},
  "license":{"kind":"cc0"},
  "language":"en-US"
}"#;

#[tokio::test]
async fn version_two_matching_saves_and_publishes_through_the_real_author_route() {
    let fixture = fixture().await;
    let (save_status, save_headers, save_body) = save(
        &fixture,
        &fixture.owner_cookie,
        FLAT_MATCHING_V2_SOURCE,
        None,
    )
    .await;
    assert_eq!(
        save_status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&save_body)
    );
    assert_no_store(&save_headers);
    assert_no_private_tokens(&save_body);
    let saved: serde_json::Value = serde_json::from_slice(&save_body).expect("public draft JSON");
    assert_eq!(saved["source"]["family"], "flat_matching_v2");
    assert_eq!(saved["response"]["kind"], "matching");

    let etag = save_headers
        .get("etag")
        .expect("matching save ETag")
        .to_str()
        .expect("matching ETag text");
    let (publish_status, publish_headers, publish_body) = publish(&fixture, etag).await;
    assert_eq!(
        publish_status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&publish_body)
    );
    assert_no_store(&publish_headers);
    assert_no_private_tokens(&publish_body);
    let published = published_record(&fixture, &publish_body).await;
    assert!(matches!(
        published.question.source,
        QuestionSource::Native { ref family } if family == "flat_matching_v2"
    ));
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    assert!(
        fixture
            .grader
            .flat_question_published_grading(fixture.context(), reference)
            .await
            .expect("matching grader lookup")
            .is_some(),
        "published matching retains grader-only material"
    );
}
