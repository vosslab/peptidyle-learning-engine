use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use base64::Engine;
use learning_data_access::{AccountSessionLifetime, AccountSessionStore, AccountSessionTokenHash};
use question_model::UserId;
use tower::ServiceExt;

use super::{account_fixture, composed_memory_router_and_store};

#[tokio::test]
async fn final_route_policy_allows_authenticated_account_presentation_read_and_save() {
    let (app, store) = composed_memory_router_and_store(false);
    let user = UserId::from_uuid(uuid::Uuid::from_u128(0x936));
    account_fixture::provision_account(store.as_ref(), user, "Presentation User").await;
    let account_secret = [0xa7; 32];
    store
        .create_account_session(
            AccountSessionTokenHash::compute(&account_secret),
            user,
            AccountSessionLifetime::from_seconds(900).expect("lifetime"),
        )
        .await
        .expect("account session");
    let cookie = format!(
        "ple_account_session={}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(account_secret)
    );

    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/auth/account/presentation")
                .header("cookie", &cookie)
                .body(Body::empty())
                .expect("presentation read"),
        )
        .await
        .expect("presentation read response");
    assert_eq!(get.status(), StatusCode::OK);
    let get_body = to_bytes(get.into_body(), 1_024)
        .await
        .expect("presentation read body");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&get_body).expect("presentation JSON"),
        serde_json::json!({ "contrast": "standard" })
    );

    let save = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/auth/account/presentation")
                .header("content-type", "application/json")
                .header("cookie", &cookie)
                .body(Body::from(r#"{"contrast":"increased"}"#))
                .expect("presentation save"),
        )
        .await
        .expect("presentation save response");
    assert_eq!(save.status(), StatusCode::OK);
    let save_body = to_bytes(save.into_body(), 1_024)
        .await
        .expect("presentation save body");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&save_body).expect("presentation JSON"),
        serde_json::json!({ "contrast": "increased" })
    );
}
