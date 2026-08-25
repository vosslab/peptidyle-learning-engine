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
    read("/api/auth/session"),
    mutation("/api/auth/logout", "POST"),
    mutation("/api/auth/passwordless/email/start", "POST"),
    mutation("/api/auth/passwordless/email/complete", "POST"),
    mutation("/api/auth/account/email/start", "POST"),
    mutation("/api/auth/account/email/complete", "POST"),
    read("/api/auth/live-demo/accounts"),
    mutation("/api/auth/live-demo/accounts", "POST"),
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
    read("/api/problem-collections"),
    mutation("/api/problem-collections", "POST"),
    mutation("/api/problem-collections/favorites", "POST"),
    mutation("/api/problem-collections/favorites", "PUT"),
    read("/api/problem-collections/{collection}"),
    mutation("/api/problem-collections/{collection}", "PUT"),
    mutation("/api/problem-collections/{collection}", "DELETE"),
    read("/api/problem-collections/{collection}/members"),
    read("/api/course-blueprints"),
    mutation("/api/course-blueprints", "POST"),
    read("/api/course-blueprints/{blueprint}"),
    mutation("/api/course-blueprints/{blueprint}", "PUT"),
    mutation("/api/course-blueprints/{blueprint}", "DELETE"),
    read("/api/alpha-courses"),
    mutation("/api/alpha-courses", "POST"),
    read("/api/alpha-courses/{alpha}"),
    mutation("/api/alpha-courses/{alpha}", "PUT"),
    read("/api/saved-problem-searches"),
    mutation("/api/saved-problem-searches", "POST"),
    read("/api/saved-problem-searches/{search}"),
    mutation("/api/saved-problem-searches/{search}", "PUT"),
    mutation("/api/saved-problem-searches/{search}", "DELETE"),
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
    read("/api/courses/{course}/grade-scheme"),
    mutation("/api/courses/{course}/grade-scheme", "PUT"),
    read("/api/courses/{course}/gradebook-totals"),
    mutation("/api/courses/{course}/grade-export.csv", "POST"),
    read("/api/assignments/{assignment}"),
    read("/api/assignments/{assignment}/learner"),
    read("/api/assignments/{assignment}/summary"),
    mutation("/api/courses/{course}/assignments/{assignment}", "PUT"),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/teaching-settings",
        "PUT",
    ),
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
    mutation("/api/course-banners/{banner}/delivery", "POST"),
    read("/api/courses/{course}/assignments/{assignment}/item-analysis"),
    read("/api/courses/{course}/retention"),
    mutation("/api/courses/{course}/retention/end", "POST"),
    mutation("/api/courses/{course}/retention/archive", "POST"),
    mutation("/api/courses/{course}/retention/delete", "POST"),
    mutation("/api/courses/{course}/retention/extend", "PATCH"),
    read("/api/courses/{course}/groups"),
    mutation("/api/courses/{course}/groups", "POST"),
    read("/api/courses/{course}/groups/{group}"),
    mutation("/api/courses/{course}/groups/{group}", "PUT"),
    mutation("/api/courses/{course}/groups/{group}", "DELETE"),
    read("/api/courses/{course}/group-purpose-policies/{purpose}"),
    mutation(
        "/api/courses/{course}/group-purpose-policies/{purpose}",
        "PUT",
    ),
    read("/api/courses/{course}/group-membership-warnings"),
    read("/api/courses/{course}/student-targets"),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/group-schedule-offsets/{group}",
        "PUT",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/group-schedule-offsets/{group}",
        "DELETE",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/group-accommodations/{group}",
        "PUT",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/group-accommodations/{group}",
        "DELETE",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/individual-policy-exceptions/{student}",
        "PUT",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/individual-policy-exceptions/{student}",
        "DELETE",
    ),
    read("/api/courses/{course}/assignments/{assignment}/policy-preview/{student}"),
    read("/api/courses/{course}/assignments/{assignment}/preview-schedule"),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/preview-pool-draw",
        "POST",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/preview-subjects/synthetic",
        "POST",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/preview-subjects/derived",
        "POST",
    ),
    mutation("/api/teaching/instructor-approvals/{account}", "PUT"),
    mutation("/api/teaching/instructor-approvals/{account}", "DELETE"),
    read("/api/teaching/instructor-approval-candidates"),
    read("/api/courses/{course}/co-instructor-targets"),
    read("/api/courses/{course}/co-instructor-invitations"),
    mutation("/api/courses/{course}/co-instructor-invitations", "POST"),
    mutation(
        "/api/courses/{course}/co-instructor-invitations/{invitation}",
        "DELETE",
    ),
    read("/api/account/co-instructor-invitations"),
    mutation(
        "/api/account/co-instructor-invitations/{invitation}",
        "POST",
    ),
    read("/api/courses/{course}/instructors"),
    mutation("/api/courses/{course}/instructors/{membership}", "DELETE"),
    mutation("/api/assignments/{assignment}/exports", "POST"),
    read("/api/exports/{export}"),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/runs",
        "POST",
    ),
    read("/api/runs/{run}"),
    read("/api/runs/{run}/summary"),
    read("/api/runs/{run}/attempts"),
    read("/api/attempts/{attempt}"),
    read("/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/question"),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/prefetch-next",
        "POST",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submissions",
        "POST",
    ),
    read("/api/attempts/{attempt}/manual-grade"),
    mutation("/api/attempts/{attempt}/manual-grade", "PUT"),
    mutation("/api/attempts/{attempt}/feedback-release", "POST"),
    read("/api/grading/summaries/{enrollment}"),
    read("/api/enrollments/{enrollment}"),
    read("/api/enrollments/{enrollment}/runs"),
    // The shell is a representation only after the POST below has created its
    // path-bound launch capability. It never creates that capability itself.
    read("/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch"),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch",
        "POST",
    ),
    read(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/activity",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/activity",
        "POST",
    ),
    mutation(
        "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/submission",
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
            route_policy(
                "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch",
                "GET",
            ),
            Some(RouteIntent::Representation),
        );
        assert_eq!(
            route_policy(
                "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch",
                "POST",
            ),
            Some(RouteIntent::StateTransition),
        );
        assert_eq!(
            route_policy(
                "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/activity",
                "GET",
            ),
            Some(RouteIntent::Representation),
        );
        assert_eq!(
            route_policy(
                "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/activity",
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

    #[test]
    fn problem_curation_routes_keep_reads_and_revisioned_mutations_distinct() {
        for path in [
            "/api/problem-collections",
            "/api/problem-collections/{collection}",
            "/api/problem-collections/{collection}/members",
            "/api/saved-problem-searches",
            "/api/saved-problem-searches/{search}",
        ] {
            assert_eq!(route_policy(path, "GET"), Some(RouteIntent::Representation));
        }
        for (path, method) in [
            ("/api/problem-collections", "POST"),
            ("/api/problem-collections/favorites", "POST"),
            ("/api/problem-collections/favorites", "PUT"),
            ("/api/problem-collections/{collection}", "PUT"),
            ("/api/problem-collections/{collection}", "DELETE"),
            ("/api/saved-problem-searches", "POST"),
            ("/api/saved-problem-searches/{search}", "PUT"),
            ("/api/saved-problem-searches/{search}", "DELETE"),
        ] {
            assert_eq!(
                route_policy(path, method),
                Some(RouteIntent::StateTransition)
            );
        }
    }

    #[test]
    fn learner_work_mutations_require_nested_course_and_assignment_routes() {
        for path in [
            "/api/courses/{course}/assignments/{assignment}/runs",
            "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/prefetch-next",
            "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/submissions",
            "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch",
            "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/activity",
            "/api/courses/{course}/assignments/{assignment}/attempts/{attempt}/external-tool/launch/submission",
        ] {
            assert_eq!(
                route_policy(path, "POST"),
                Some(RouteIntent::StateTransition),
            );
        }
        for retired in [
            "/api/runs",
            "/api/attempts/{attempt}/prefetch-next",
            "/api/submissions/{attempt}",
            "/api/attempts/{attempt}/external-tool/launch",
            "/api/attempts/{attempt}/external-tool/launch/activity",
            "/api/attempts/{attempt}/external-tool/launch/submission",
        ] {
            assert_eq!(route_policy(retired, "POST"), None);
        }
    }

    #[test]
    fn retired_provider_login_has_no_route_authority() {
        assert_eq!(route_policy("/api/auth/login", "POST"), None);
    }

    #[tokio::test]
    async fn composition_gate_refuses_an_unreviewed_browser_route() {
        let app = apply_route_method_policy(
            Router::new()
                .route("/health", get(|| async { StatusCode::NO_CONTENT }))
                .route(
                    "/api/courses/{course}/groups/{group}/unreviewed",
                    get(|| async { StatusCode::NO_CONTENT }),
                ),
        );
        let health = app
            .clone()
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::NO_CONTENT);
        let unreviewed = app
            .oneshot(
                Request::get("/api/courses/course/groups/group/unreviewed")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unreviewed.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn live_demo_selector_operations_reach_the_composed_policy_router() {
        async fn no_content() -> StatusCode {
            StatusCode::NO_CONTENT
        }

        let app = apply_route_method_policy(Router::new().route(
            "/api/auth/live-demo/accounts",
            get(no_content).post(no_content),
        ));
        for (method, intent) in [
            ("GET", RouteIntent::Representation),
            ("POST", RouteIntent::StateTransition),
        ] {
            assert_eq!(
                route_policy("/api/auth/live-demo/accounts", method),
                Some(intent)
            );
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/api/auth/live-demo/accounts")
                        .body(Body::empty())
                        .expect("selector operation request"),
                )
                .await
                .expect("selector operation response");
            assert_eq!(response.status(), StatusCode::NO_CONTENT);
        }
    }

    #[tokio::test]
    async fn teaching_operations_routes_reach_the_composed_policy_router_with_reviewed_intents() {
        async fn no_content() -> StatusCode {
            StatusCode::NO_CONTENT
        }

        let app = apply_route_method_policy(
            Router::new()
                .route(
                    "/api/courses/{course}/groups",
                    get(no_content).post(no_content),
                )
                .route(
                    "/api/courses/{course}/groups/{group}",
                    get(no_content).put(no_content).delete(no_content),
                )
                .route(
                    "/api/courses/{course}/group-purpose-policies/{purpose}",
                    get(no_content).put(no_content),
                )
                .route(
                    "/api/courses/{course}/group-membership-warnings",
                    get(no_content),
                )
                .route(
                    "/api/courses/{course}/student-targets",
                    get(no_content),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/group-schedule-offsets/{group}",
                    put(no_content).delete(no_content),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/group-accommodations/{group}",
                    put(no_content).delete(no_content),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/individual-policy-exceptions/{student}",
                    put(no_content).delete(no_content),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/policy-preview/{student}",
                    get(no_content),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/preview-schedule",
                    get(no_content),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/preview-pool-draw",
                    post(no_content),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/preview-subjects/synthetic",
                    post(no_content),
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/preview-subjects/derived",
                    post(no_content),
                )
                .route(
                    "/api/teaching/instructor-approvals/{account}",
                    put(no_content).delete(no_content),
                )
                .route(
                    "/api/teaching/instructor-approval-candidates",
                    get(no_content),
                )
                .route(
                    "/api/courses/{course}/co-instructor-targets",
                    get(no_content),
                )
                .route(
                    "/api/courses/{course}/co-instructor-invitations",
                    get(no_content).post(no_content),
                )
                .route(
                    "/api/courses/{course}/co-instructor-invitations/{invitation}",
                    delete(no_content),
                )
                .route(
                    "/api/account/co-instructor-invitations",
                    get(no_content),
                )
                .route(
                    "/api/account/co-instructor-invitations/{invitation}",
                    post(no_content),
                )
                .route(
                    "/api/courses/{course}/instructors",
                    get(no_content),
                )
                .route(
                    "/api/courses/{course}/instructors/{membership}",
                    delete(no_content),
                ),
        );
        for (path, method, request_path, intent) in [
            (
                "/api/courses/{course}/groups",
                "GET",
                "/api/courses/course/groups",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/groups",
                "POST",
                "/api/courses/course/groups",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/groups/{group}",
                "GET",
                "/api/courses/course/groups/group",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/groups/{group}",
                "PUT",
                "/api/courses/course/groups/group",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/groups/{group}",
                "DELETE",
                "/api/courses/course/groups/group",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/group-purpose-policies/{purpose}",
                "GET",
                "/api/courses/course/group-purpose-policies/section",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/group-purpose-policies/{purpose}",
                "PUT",
                "/api/courses/course/group-purpose-policies/section",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/group-membership-warnings",
                "GET",
                "/api/courses/course/group-membership-warnings",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/student-targets",
                "GET",
                "/api/courses/course/student-targets",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/group-schedule-offsets/{group}",
                "PUT",
                "/api/courses/course/assignments/assignment/group-schedule-offsets/group",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/group-schedule-offsets/{group}",
                "DELETE",
                "/api/courses/course/assignments/assignment/group-schedule-offsets/group",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/group-accommodations/{group}",
                "PUT",
                "/api/courses/course/assignments/assignment/group-accommodations/group",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/group-accommodations/{group}",
                "DELETE",
                "/api/courses/course/assignments/assignment/group-accommodations/group",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/individual-policy-exceptions/{student}",
                "PUT",
                "/api/courses/course/assignments/assignment/individual-policy-exceptions/student",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/individual-policy-exceptions/{student}",
                "DELETE",
                "/api/courses/course/assignments/assignment/individual-policy-exceptions/student",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/policy-preview/{student}",
                "GET",
                "/api/courses/course/assignments/assignment/policy-preview/student",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/preview-schedule",
                "GET",
                "/api/courses/course/assignments/assignment/preview-schedule",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/preview-pool-draw",
                "POST",
                "/api/courses/course/assignments/assignment/preview-pool-draw",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/preview-subjects/synthetic",
                "POST",
                "/api/courses/course/assignments/assignment/preview-subjects/synthetic",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/assignments/{assignment}/preview-subjects/derived",
                "POST",
                "/api/courses/course/assignments/assignment/preview-subjects/derived",
                RouteIntent::StateTransition,
            ),
            (
                "/api/teaching/instructor-approvals/{account}",
                "PUT",
                "/api/teaching/instructor-approvals/account",
                RouteIntent::StateTransition,
            ),
            (
                "/api/teaching/instructor-approvals/{account}",
                "DELETE",
                "/api/teaching/instructor-approvals/account",
                RouteIntent::StateTransition,
            ),
            (
                "/api/teaching/instructor-approval-candidates",
                "GET",
                "/api/teaching/instructor-approval-candidates",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/co-instructor-targets",
                "GET",
                "/api/courses/course/co-instructor-targets",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/co-instructor-invitations",
                "GET",
                "/api/courses/course/co-instructor-invitations",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/co-instructor-invitations",
                "POST",
                "/api/courses/course/co-instructor-invitations",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/co-instructor-invitations/{invitation}",
                "DELETE",
                "/api/courses/course/co-instructor-invitations/invitation",
                RouteIntent::StateTransition,
            ),
            (
                "/api/account/co-instructor-invitations",
                "GET",
                "/api/account/co-instructor-invitations",
                RouteIntent::Representation,
            ),
            (
                "/api/account/co-instructor-invitations/{invitation}",
                "POST",
                "/api/account/co-instructor-invitations/invitation",
                RouteIntent::StateTransition,
            ),
            (
                "/api/courses/{course}/instructors",
                "GET",
                "/api/courses/course/instructors",
                RouteIntent::Representation,
            ),
            (
                "/api/courses/{course}/instructors/{membership}",
                "DELETE",
                "/api/courses/course/instructors/membership",
                RouteIntent::StateTransition,
            ),
        ] {
            assert_eq!(route_policy(path, method), Some(intent), "{method} {path}");
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(request_path)
                        .body(Body::empty())
                        .expect("teaching operation request"),
                )
                .await
                .expect("composed route response");
            assert_eq!(response.status(), StatusCode::NO_CONTENT, "{method} {path}");
        }
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
                )
                .route(
                    "/api/courses/{course}/assignments/{assignment}/teaching-settings",
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
            Request::put("/api/courses/course/assignments/assignment/teaching-settings"),
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
