use super::fixtures::publish_fixture;
use super::gradebook::{
    EnrolledStudent, create_assignment, created_assignment_id, enrolled_student, fixture,
};
use super::gradebook_route_support::same_origin;
use super::gradebook_route_support::{
    AlwaysCorrectBackend, CompletedRunIdentity, GradebookRouteHarness, course_app_with_runs,
};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use learning_data_access::{
    CatalogStore, MaterializeAssignmentEntitlementCommand, NavigationReferenceStore, Store,
    TeachingAuthorityReferenceStore, TenantContext,
};
use question_model::{AssignmentId, UserId};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn registered_gradebook_selection_returns_the_closed_no_store_projection() {
    let (store, instructor_cookie, tenant, instructor, course) = fixture().await;
    let app = crate::course::router(Arc::clone(&store));
    let reference = publish_fixture(
        &store,
        TenantContext::from_authenticated_session(tenant),
        tenant,
        instructor,
    )
    .await;
    let question = store
        .get_catalog_problem(TenantContext::from_authenticated_session(tenant), reference)
        .await
        .expect("catalog lookup")
        .expect("fixture publication")
        .question_id;
    let assignment_id = AssignmentId::from_uuid(
        uuid::Uuid::parse_str(
            &created_assignment_id(
                create_assignment(
                    &app,
                    &instructor_cookie,
                    course,
                    &question,
                    "Selection contract",
                )
                .await,
            )
            .await,
        )
        .expect("assignment UUID"),
    );
    super::fixtures::publish_assignment(
        &store,
        TenantContext::from_authenticated_session(tenant),
        instructor,
        course,
        assignment_id,
        question_model::AssignmentTeachingSettings {
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy: question_model::BaseAssignmentPolicy::default(),
        },
    )
    .await;
    let student = UserId::from_uuid(super::fixtures::id(9_106));
    let student_cookie = enrolled_student(
        &store,
        tenant,
        instructor,
        course,
        EnrolledStudent {
            user: student,
            email: "selection.student@example.edu",
            roster_id: "910010003",
            display_name: "Selection Student",
        },
    )
    .await;
    let membership = store
        .list_course_active_student_membership_reference_views(
            TenantContext::from_authenticated_session(tenant),
            instructor,
            course,
            learning_data_access::PageRequest::first(
                learning_data_access::PageSize::new(100).expect("page size"),
            ),
        )
        .await
        .expect("membership references")
        .items
        .into_iter()
        .find(|view| view.display_name == "Selection Student")
        .expect("student membership")
        .reference;
    store
        .issue_assignment_entitlement(
            TenantContext::from_authenticated_session(tenant),
            MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                assignment_id,
                instructor,
                question_model::EntitlementPurpose::InstructorIssue,
            )
            .expect("assignment entitlement command"),
        )
        .await
        .expect("Student assignment entitlement");
    let assignment = store
        .assignment_reference(
            TenantContext::from_authenticated_session(tenant),
            instructor,
            assignment_id,
        )
        .await
        .expect("assignment reference lookup")
        .expect("assignment reference");
    let backend = Arc::new(AlwaysCorrectBackend);
    let app = course_app_with_runs(&store, &backend);
    let harness = GradebookRouteHarness::new(&app, &store, &backend);
    let run = harness
        .completed_run_reference(CompletedRunIdentity {
            tenant,
            student,
            course,
            assignment: assignment_id,
            cookie: &student_cookie,
        })
        .await;
    let response = app
        .oneshot(same_origin(
            Request::get(format!(
                "/api/courses/{course}/gradebook/selection?assignmentRef={assignment}"
            ))
            .header("cookie", instructor_cookie)
            .body(Body::empty())
            .expect("selection request"),
        ))
        .await
        .expect("selection response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let view: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("selection body"),
    )
    .expect("selection JSON");
    assert_eq!(view["kind"], "studentSelection");
    assert_eq!(view["rows"][0]["membership"], membership.to_string());
    assert_eq!(view["rows"][0]["displayLabel"], "Selection Student");
    assert_eq!(view["rows"][0]["assignment"], assignment.to_string());
    assert_eq!(view["rows"][0]["inspectionChoice"]["kind"], "selectedRun");
    assert_eq!(view["rows"][0]["inspectionChoice"]["run"], run.to_string());
}

#[tokio::test]
async fn registered_submitted_run_chooser_returns_public_choices_without_caching() {
    let (store, instructor_cookie, tenant, instructor, course) = fixture().await;
    let setup_app = crate::course::router(Arc::clone(&store));
    let reference = publish_fixture(
        &store,
        TenantContext::from_authenticated_session(tenant),
        tenant,
        instructor,
    )
    .await;
    let question = store
        .get_catalog_problem(TenantContext::from_authenticated_session(tenant), reference)
        .await
        .expect("catalog lookup")
        .expect("fixture publication")
        .question_id;
    let assignment_id = AssignmentId::from_uuid(
        uuid::Uuid::parse_str(
            &created_assignment_id(
                create_assignment(
                    &setup_app,
                    &instructor_cookie,
                    course,
                    &question,
                    "Run chooser contract",
                )
                .await,
            )
            .await,
        )
        .expect("assignment UUID"),
    );
    super::fixtures::publish_assignment(
        &store,
        TenantContext::from_authenticated_session(tenant),
        instructor,
        course,
        assignment_id,
        question_model::AssignmentTeachingSettings {
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy: question_model::BaseAssignmentPolicy::default(),
        },
    )
    .await;
    let student = UserId::from_uuid(super::fixtures::id(9_107));
    let student_cookie = enrolled_student(
        &store,
        tenant,
        instructor,
        course,
        EnrolledStudent {
            user: student,
            email: "chooser.student@example.edu",
            roster_id: "910010004",
            display_name: "Chooser Student",
        },
    )
    .await;
    store
        .issue_assignment_entitlement(
            TenantContext::from_authenticated_session(tenant),
            MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                assignment_id,
                instructor,
                question_model::EntitlementPurpose::InstructorIssue,
            )
            .expect("assignment entitlement command"),
        )
        .await
        .expect("Student assignment entitlement");
    let assignment = store
        .assignment_reference(
            TenantContext::from_authenticated_session(tenant),
            instructor,
            assignment_id,
        )
        .await
        .expect("assignment reference lookup")
        .expect("assignment reference");
    let membership = store
        .list_course_active_student_membership_reference_views(
            TenantContext::from_authenticated_session(tenant),
            instructor,
            course,
            learning_data_access::PageRequest::first(
                learning_data_access::PageSize::new(100).expect("page size"),
            ),
        )
        .await
        .expect("membership references")
        .items
        .into_iter()
        .find(|view| view.display_name == "Chooser Student")
        .expect("student membership")
        .reference;
    let backend = Arc::new(AlwaysCorrectBackend);
    let app = course_app_with_runs(&store, &backend);
    let harness = GradebookRouteHarness::new(&app, &store, &backend);
    let run = harness
        .completed_run_reference(CompletedRunIdentity {
            tenant,
            student,
            course,
            assignment: assignment_id,
            cookie: &student_cookie,
        })
        .await;
    let response = app
        .oneshot(same_origin(
            Request::get(format!(
                "/api/courses/{course}/gradebook/students/{membership}/assignments/{assignment}/runs"
            ))
            .header("cookie", instructor_cookie)
            .body(Body::empty())
            .expect("run chooser request"),
        ))
        .await
        .expect("run chooser response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let view: serde_json::Value = serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("run chooser body"),
    )
    .expect("run chooser JSON");
    assert_eq!(view["rows"][0]["run"], run.to_string());
    assert!(view["rows"][0]["submittedAt"].as_i64().is_some());
    assert_eq!(view["rows"][0]["scoreSelected"], true);
}
