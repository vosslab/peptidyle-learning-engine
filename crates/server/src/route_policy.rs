//! The application-wide HTTP method security policy.
//!
//! A browser can attach `SameSite=Lax` cookies to a top-level cross-site GET.
//! Therefore a GET is not merely a convenient spelling for an action: it is a
//! promise that the route is a read-only representation.  Keep that promise
//! visible in one typed inventory, rather than relying on a reviewer to infer
//! it from an individual Axum handler.

use axum::extract::MatchedPath;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Router, middleware};

/// The security meaning of one externally reachable HTTP operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteIntent {
    /// A read-only representation. GET handlers must not create sessions,
    /// consume capabilities, enqueue work, sign private URLs, or write audit
    /// records.
    Representation,
    /// A state transition. It must use a non-safe HTTP method so the browser
    /// origin check applies before it reaches the handler.
    StateTransition,
}

/// An externally reachable operation and its security contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutePolicy {
    pub path: &'static str,
    pub method: &'static str,
    pub intent: RouteIntent,
}

const fn read(path: &'static str) -> RoutePolicy {
    RoutePolicy {
        path,
        method: "GET",
        intent: RouteIntent::Representation,
    }
}

const fn mutation(path: &'static str, method: &'static str) -> RoutePolicy {
    RoutePolicy {
        path,
        method,
        intent: RouteIntent::StateTransition,
    }
}

/// The authoritative public route-method inventory.
///
/// Update this list in the same patch as a route declaration. Tests enforce
/// that normal reads and state transitions retain their HTTP safety contract.
pub const APPLICATION_ROUTE_POLICY: &[RoutePolicy] = &[
    read("/health"),
    mutation("/api/auth/login", "POST"),
    read("/api/auth/session"),
    mutation("/api/auth/logout", "POST"),
    mutation("/api/auth/passwordless/email/start", "POST"),
    mutation("/api/auth/passwordless/email/complete", "POST"),
    mutation("/api/auth/account/email/start", "POST"),
    mutation("/api/auth/account/email/complete", "POST"),
    mutation("/api/course-invitations/redeem", "POST"),
    read("/api/auth/account/courses"),
    read("/api/auth/account/presentation"),
    mutation("/api/auth/account/presentation", "PUT"),
    mutation("/api/auth/account/course-session", "POST"),
    mutation("/api/auth/passkeys/registration/start", "POST"),
    mutation("/api/auth/passkeys/registration/complete", "POST"),
    mutation("/api/auth/passkeys/authentication/start", "POST"),
    mutation("/api/auth/passkeys/authentication/complete", "POST"),
    read("/api/auth/passkeys"),
    mutation("/api/auth/passkeys/{passkey}", "DELETE"),
    read("/api/navigation/{reference}"),
    read("/api/problems"),
    read("/api/problems/search"),
    read("/api/problems/by-id/{reference}"),
    read("/api/problems/by-id/{reference}/detail"),
    mutation("/api/problems/{workspace}/publish", "POST"),
    mutation("/api/problems/{workspace}/qti-publish", "POST"),
    mutation("/api/problems/{workspace}/flat-question-publish", "POST"),
    mutation("/api/problems/by-id/{reference}/deprecate", "POST"),
    mutation("/api/problems/by-id/{reference}/archive", "POST"),
    read("/api/taxonomy"),
    read("/api/workspaces"),
    read("/api/workspaces/{workspace}"),
    mutation("/api/workspaces/{workspace}", "PUT"),
    mutation("/api/workspaces/{workspace}", "DELETE"),
    mutation("/api/workspaces/{workspace}/publication-validation", "POST"),
    read("/api/workspaces/{workspace}/publication-diff"),
    read("/api/workspaces/{workspace}/author-preview"),
    read("/api/workspaces/{workspace}/flat-question"),
    mutation("/api/workspaces/{workspace}/flat-question", "PUT"),
    read("/api/workspaces/{workspace}/flat-question-assets"),
    mutation("/api/workspaces/{workspace}/flat-question-assets", "POST"),
    read("/api/workspaces/{workspace}/qti-imports/{import}"),
    mutation("/api/workspaces/{workspace}/qti-imports/{import}", "PUT"),
    mutation(
        "/api/workspaces/{workspace}/qti-imports/{import}/items/{item}/convert-flat",
        "POST",
    ),
    read("/api/courses"),
    mutation("/api/courses", "POST"),
    read("/api/courses/{course}"),
    read("/api/courses/{course}/assignments"),
    mutation("/api/courses/{course}/assignments", "POST"),
    read("/api/courses/{course}/gradebook"),
    read("/api/assignments/{assignment}"),
    mutation("/api/courses/{course}/assignments/{assignment}", "PUT"),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/items",
        "POST",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/items/{item}",
        "DELETE",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/items/{item}/question",
        "PUT",
    ),
    read("/api/courses/{course}/roster"),
    mutation("/api/courses/{course}/members/{member}", "DELETE"),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/grade-export.csv",
        "POST",
    ),
    mutation("/api/courses/{course}/local-teaching-members", "POST"),
    mutation("/api/courses/{course}/invitations", "POST"),
    mutation("/api/courses/{course}/invitations/{invitation}", "DELETE"),
    mutation("/api/courses/{course}/enrollment-policy", "PUT"),
    mutation("/api/courses/{course}/roster-imports/preview", "POST"),
    mutation(
        "/api/courses/{course}/roster-imports/{import}/commit",
        "POST",
    ),
    read("/api/courses/{course}/appearance"),
    mutation("/api/courses/{course}/appearance", "PUT"),
    mutation("/api/courses/{course}/appearance/banner-candidates", "POST"),
    read("/api/courses/{course}/assignments/{assignment}/item-analysis"),
    read("/api/courses/{course}/retention"),
    mutation("/api/courses/{course}/retention/end", "POST"),
    mutation("/api/courses/{course}/retention/archive", "POST"),
    mutation("/api/courses/{course}/retention/delete", "POST"),
    mutation("/api/courses/{course}/retention/extend", "PATCH"),
    mutation("/api/assignments/{assignment}/exports", "POST"),
    read("/api/exports/{export}"),
    mutation("/api/runs", "POST"),
    read("/api/runs/{run}"),
    read("/api/runs/{run}/summary"),
    read("/api/runs/{run}/attempts"),
    read("/api/attempts/{attempt}"),
    read("/api/attempts/{attempt}/question"),
    mutation("/api/attempts/{attempt}/prefetch-next", "POST"),
    mutation("/api/submissions/{attempt}", "POST"),
    read("/api/attempts/{attempt}/manual-grade"),
    mutation("/api/attempts/{attempt}/manual-grade", "PUT"),
    mutation("/api/attempts/{attempt}/feedback-release", "POST"),
    read("/api/grading/summaries/{enrollment}"),
    read("/api/enrollments/{enrollment}"),
    read("/api/enrollments/{enrollment}/runs"),
    // The shell is a representation only after the POST below has created its
    // path-bound launch capability. It never creates that capability itself.
    read("/api/attempts/{attempt}/external-tool/launch"),
    mutation("/api/attempts/{attempt}/external-tool/launch", "POST"),
    read("/api/attempts/{attempt}/external-tool/launch/activity"),
    mutation(
        "/api/attempts/{attempt}/external-tool/launch/activity",
        "POST",
    ),
    mutation(
        "/api/attempts/{attempt}/external-tool/launch/submission",
        "POST",
    ),
    // Public immutable assets are a representation. Protected delivery is a
    // state transition because it authorizes access, appends an audit event,
    // and issues a short-lived object capability.
    read("/api/assets/{id}"),
    mutation("/api/assets/{id}/delivery", "POST"),
    mutation("/api/validation/response-format", "POST"),
    mutation("/api/validation/timer", "POST"),
    mutation("/api/validation/assignment-capabilities", "POST"),
];

/// Returns a route policy only when the method/path pair has been explicitly
/// reviewed. This supports composition-level enforcement once every route is
/// registered through the policy router.
pub fn route_policy(path: &str, method: &str) -> Option<RouteIntent> {
    APPLICATION_ROUTE_POLICY
        .iter()
        .find(|policy| policy.path == path && policy.method == method)
        .map(|policy| policy.intent)
}

/// Applies the inventory as a fail-closed composition boundary.
///
/// Axum attaches [`MatchedPath`] after routing and before `Router::layer`
/// middleware runs. Thus an endpoint that is added to a route group but not
/// reviewed in [`APPLICATION_ROUTE_POLICY`] is unavailable, rather than
/// quietly becoming a browser-reachable exception. HEAD is Axum's
/// representation alias for a GET route and inherits the GET policy.
pub fn apply_route_method_policy(router: Router) -> Router {
    router.layer(middleware::from_fn(enforce_route_method_policy))
}

async fn enforce_route_method_policy(request: axum::extract::Request, next: Next) -> Response {
    let Some(matched_path) = request.extensions().get::<MatchedPath>() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let policy_method = match *request.method() {
        Method::HEAD => "GET",
        _ => request.method().as_str(),
    };
    if route_policy(matched_path.as_str(), policy_method).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::{delete, get, post, put};
    use tower::ServiceExt;

    #[test]
    fn normal_reads_are_only_get_representations() {
        for policy in APPLICATION_ROUTE_POLICY {
            if policy.intent == RouteIntent::Representation {
                assert_eq!(policy.method, "GET", "{}", policy.path);
            }
        }
    }

    #[test]
    fn state_transitions_are_never_safe_http_methods() {
        for policy in APPLICATION_ROUTE_POLICY {
            if policy.intent == RouteIntent::StateTransition {
                assert!(
                    !matches!(policy.method, "GET" | "HEAD" | "OPTIONS"),
                    "{} {} is a state transition behind a safe method",
                    policy.method,
                    policy.path,
                );
            }
        }
    }

    #[test]
    fn policy_covers_each_operation_once() {
        let mut operations = HashSet::new();
        for policy in APPLICATION_ROUTE_POLICY {
            assert!(
                operations.insert((policy.path, policy.method)),
                "duplicate route policy: {} {}",
                policy.method,
                policy.path,
            );
        }
    }

    #[test]
    fn external_launch_gets_are_shell_or_activity_representations_not_launch_creation() {
        assert_eq!(
            route_policy("/api/attempts/{attempt}/external-tool/launch", "GET"),
            Some(RouteIntent::Representation),
        );
        assert_eq!(
            route_policy("/api/attempts/{attempt}/external-tool/launch", "POST"),
            Some(RouteIntent::StateTransition),
        );
        assert_eq!(
            route_policy(
                "/api/attempts/{attempt}/external-tool/launch/activity",
                "GET",
            ),
            Some(RouteIntent::Representation),
        );
        assert_eq!(
            route_policy(
                "/api/attempts/{attempt}/external-tool/launch/activity",
                "POST",
            ),
            Some(RouteIntent::StateTransition),
        );
    }

    #[test]
    fn private_asset_delivery_is_a_state_transition_not_a_get_exception() {
        assert_eq!(
            route_policy("/api/assets/{id}", "GET"),
            Some(RouteIntent::Representation),
        );
        assert_eq!(
            route_policy("/api/assets/{id}/delivery", "POST"),
            Some(RouteIntent::StateTransition),
        );
    }

    #[test]
    fn navigation_reference_resolution_is_a_read_only_representation() {
        assert_eq!(
            route_policy("/api/navigation/{reference}", "GET"),
            Some(RouteIntent::Representation),
        );
    }

    #[tokio::test]
    async fn composition_gate_refuses_an_unreviewed_browser_route() {
        let app = apply_route_method_policy(
            Router::new()
                .route("/health", get(|| async { StatusCode::NO_CONTENT }))
                .route("/unreviewed", get(|| async { StatusCode::NO_CONTENT })),
        );
        let health = app
            .clone()
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::NO_CONTENT);
        let unreviewed = app
            .oneshot(Request::get("/unreviewed").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(unreviewed.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn qid_catalog_and_assignment_item_operations_reach_the_composed_policy_router() {
        let app = apply_route_method_policy(
            Router::new()
                .route(
                    "/api/problems/by-id/{reference}/detail",
                    get(|| async { StatusCode::NO_CONTENT }),
                )
                .route(
                    "/api/problems/by-id/{reference}/deprecate",
                    post(|| async { StatusCode::NO_CONTENT }),
                )
                .route(
                    "/api/problems/by-id/{reference}/archive",
                    post(|| async { StatusCode::NO_CONTENT }),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/items",
                    post(|| async { StatusCode::NO_CONTENT }),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/items/{item}",
                    delete(|| async { StatusCode::NO_CONTENT }),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/items/{item}/question",
                    put(|| async { StatusCode::NO_CONTENT }),
                ),
        );
        for request in [
            Request::get("/api/problems/by-id/ABC-1234/detail"),
            Request::post("/api/problems/by-id/ABC-1234/deprecate"),
            Request::post("/api/problems/by-id/ABC-1234/archive"),
            Request::post("/api/courses/course/assignments/assignment/items"),
            Request::delete("/api/courses/course/assignments/assignment/items/item"),
            Request::put("/api/courses/course/assignments/assignment/items/item/question"),
        ] {
            let response = app
                .clone()
                .oneshot(request.body(Body::empty()).expect("request body"))
                .await
                .expect("composed route response");
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }
    }
}
