use super::*;

#[tokio::test]
async fn memory_sysadmin_candidate_search_is_bounded_safe_and_tracks_approval() {
    let fixture = fixture().await;
    let candidates = "/api/teaching/instructor-approval-candidates";
    let denied = request(
        &fixture.app,
        Request::get(format!("{candidates}?query=x"))
            .header("cookie", &fixture.ordinary)
            .body(Body::empty())
            .expect("unauthorized candidate search"),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_safe(&denied);
    for invalid in [
        candidates.to_owned(),
        format!("{candidates}?query=x"),
        format!("{candidates}?query=target&size=0"),
        format!("{candidates}?query=target&unrecognized=value"),
    ] {
        let response = request(
            &fixture.app,
            Request::get(invalid)
                .header("cookie", &fixture.admin)
                .body(Body::empty())
                .expect("invalid candidate search"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_safe(&response);
    }
    let first = request(
        &fixture.app,
        Request::get(format!("{candidates}?query=target&size=1"))
            .header("cookie", &fixture.admin)
            .body(Body::empty())
            .expect("first candidate page"),
    )
    .await;
    assert_eq!(first.status(), StatusCode::OK);
    assert_safe(&first);
    let first = json(first).await;
    assert_eq!(first["candidates"][0]["approval"]["state"], "unapproved");
    assert!(first["candidates"][0]["approval"]["revision"].is_null());
    let reference = first["candidates"][0]["account"]["reference"]
        .as_str()
        .expect("opaque account reference")
        .to_owned();
    let cursor = first["nextCursor"].as_str().expect("candidate cursor");
    let second = request(
        &fixture.app,
        Request::get(format!("{candidates}?query=target&size=1&after={cursor}"))
            .header("cookie", &fixture.admin)
            .body(Body::empty())
            .expect("second candidate page"),
    )
    .await;
    assert_eq!(second.status(), StatusCode::OK);
    let serialized = serde_json::to_string(&json(second).await).expect("safe candidate JSON");
    assert!(!serialized.contains('@'));
    assert!(!serialized.contains("00000000-0000-0000-0000"));
    let approved = request(
        &fixture.app,
        Request::put(format!("/api/teaching/instructor-approvals/{reference}"))
            .header("cookie", &fixture.admin)
            .body(Body::empty())
            .expect("approve candidate"),
    )
    .await;
    assert_eq!(approved.status(), StatusCode::OK);
    let approved = json(approved).await;
    let revision = approved["revision"].as_str().expect("approval revision");
    let updated = request(
        &fixture.app,
        Request::get(format!("{candidates}?query=target"))
            .header("cookie", &fixture.admin)
            .body(Body::empty())
            .expect("approved candidate search"),
    )
    .await;
    let updated = json(updated).await;
    let candidate = updated["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .find(|candidate| candidate["account"]["reference"] == reference)
        .expect("approved candidate");
    assert_eq!(candidate["approval"]["state"], "approved");
    assert_eq!(candidate["approval"]["revision"], revision);
    let revoked = request(
        &fixture.app,
        Request::delete(format!("/api/teaching/instructor-approvals/{reference}"))
            .header("cookie", &fixture.admin)
            .header("if-match", format!("\"{revision}\""))
            .body(Body::empty())
            .expect("revoke candidate"),
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::OK);
    let revoked = json(revoked).await;
    let updated = request(
        &fixture.app,
        Request::get(format!("{candidates}?query=target"))
            .header("cookie", &fixture.admin)
            .body(Body::empty())
            .expect("revoked candidate search"),
    )
    .await;
    let updated = json(updated).await;
    let candidate = updated["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .find(|candidate| candidate["account"]["reference"] == reference)
        .expect("revoked candidate");
    assert_eq!(candidate["approval"]["state"], "revoked");
    assert_eq!(candidate["approval"]["revision"], revoked["revision"]);
}
