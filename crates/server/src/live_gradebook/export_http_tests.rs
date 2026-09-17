//! Query rejection and unauthenticated concealment must never produce cacheable downloads.

use std::sync::Arc;

use axum::{body::Body, http::Request};
use learning_data_access::postgres::{
    PostgresCourseGradebookStore, PostgresSessionStore, lazy_pool,
};
use tower::ServiceExt;

#[tokio::test]
async fn export_rejections_are_no_store_and_never_attachments() {
    // A closed lazy pool cannot perform database or network work in this boundary test.
    let pool = lazy_pool("postgres://unused:unused@localhost/unused").unwrap();
    pool.close().await;
    let router = super::live_gradebook_router(
        Arc::new(PostgresSessionStore::new(pool.clone())),
        PostgresCourseGradebookStore::new(pool),
    );
    let base = "/api/course-instances/CI7K3M2QAZ/gradebook/export";
    for query in [
        "",
        "?format=",
        "?format=xlsx",
        "?format=CSV",
        "?format=csv&format=tsv",
        "?format=csv&format=csv",
        "?format=csv&filename=grades.csv",
        "?format=%FF",
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("{base}{query}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 400, "{query}");
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(!response.headers().contains_key("content-disposition"));
    }
    for (method, path, status) in [
        ("GET", format!("{base}?format=csv"), 404),
        ("GET", format!("{base}?format=tsv"), 404),
        (
            "GET",
            "/api/course-instances/invalid/gradebook/export?format=csv".to_owned(),
            404,
        ),
        (
            "GET",
            "/api/course-instances/%FF/gradebook/export?format=csv".to_owned(),
            400,
        ),
        ("POST", format!("{base}?format=csv"), 405),
    ] {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert!(!response.headers().contains_key("content-disposition"));
    }
}
