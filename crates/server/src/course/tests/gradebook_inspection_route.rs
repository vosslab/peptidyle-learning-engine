use super::fixtures::{id, publish_fixture};
use super::gradebook::{
    EnrolledStudent, create_assignment, created_assignment_id, enrolled_student, fixture,
};
use super::gradebook_route_support::{
    AlwaysCorrectBackend, CompletedRunIdentity, GradebookRouteHarness, course_app_with_runs,
    same_origin,
};
use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, IF_MATCH};
use axum::http::{Request, StatusCode};
use learning_data_access::{
    CatalogStore, MaterializeAssignmentEntitlementCommand, NavigationReferenceStore, PageRequest,
    PageSize, Store, TeachingAuthorityReferenceStore, TenantContext,
};
use question_model::{AssignmentId, UserId};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn registered_inspection_conceals_rejected_origins_before_audit_and_returns_exact_context() {
    let (store, instructor_cookie, tenant, instructor, course) = fixture().await;
    let backend = Arc::new(AlwaysCorrectBackend);
    let app = course_app_with_runs(&store, &backend);
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
    let created = create_assignment(
        &app,
        &instructor_cookie,
        course,
        &question,
        "Inspection contract",
    )
    .await;
    let assignment = AssignmentId::from_uuid(
        uuid::Uuid::parse_str(&created_assignment_id(created).await).expect("assignment UUID"),
    );
    super::fixtures::publish_assignment(
        &store,
        TenantContext::from_authenticated_session(tenant),
        instructor,
        course,
        assignment,
        question_model::AssignmentTeachingSettings {
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy: question_model::BaseAssignmentPolicy::default(),
        },
    )
    .await;
    let assignment_revision = format!(
        "\"{}\"",
        store
            .get_assignment_for_edit(
                TenantContext::from_authenticated_session(tenant),
                assignment,
            )
            .await
            .expect("published assignment lookup")
            .expect("published assignment")
            .revision
            .value()
    );
    let operation = app
        .clone()
        .oneshot(
            Request::post(format!(
                "/api/courses/{course}/assignments/{assignment}/grading-operations/recalculate"
            ))
            .header("cookie", &instructor_cookie)
            .header(IF_MATCH, assignment_revision)
            .header("idempotency-key", uuid::Uuid::from_u128(9_108).to_string())
            .body(Body::empty())
            .expect("recalculation request"),
        )
        .await
        .expect("recalculation response");
    assert_eq!(operation.status(), StatusCode::OK);
    let operation: serde_json::Value = serde_json::from_slice(
        &to_bytes(operation.into_body(), 64 * 1024)
            .await
            .expect("recalculation body"),
    )
    .expect("recalculation JSON");
    let operation = serde_json::from_value::<question_model::GradingOperationReference>(
        operation["operation"].clone(),
    )
    .expect("operation reference");
    let student = UserId::from_uuid(id(9_107));
    let student_cookie = enrolled_student(
        &store,
        tenant,
        instructor,
        course,
        EnrolledStudent {
            user: student,
            email: "inspection.student@example.edu",
            roster_id: "910010004",
            display_name: "Inspection Student",
        },
    )
    .await;
    store
        .issue_assignment_entitlement(
            TenantContext::from_authenticated_session(tenant),
            MaterializeAssignmentEntitlementCommand::for_instructor_action(
                student,
                course,
                assignment,
                instructor,
                question_model::EntitlementPurpose::InstructorIssue,
            )
            .expect("assignment entitlement command"),
        )
        .await
        .expect("Student assignment entitlement");
    let harness = GradebookRouteHarness::new(&app, &store, &backend);
    let run = harness
        .completed_run_reference(CompletedRunIdentity {
            tenant,
            student,
            course,
            assignment,
            cookie: &student_cookie,
        })
        .await;
    let assignment_reference = store
        .assignment_reference(
            TenantContext::from_authenticated_session(tenant),
            instructor,
            assignment,
        )
        .await
        .expect("assignment reference lookup")
        .expect("assignment reference");
    let membership = store
        .list_course_active_student_membership_reference_views(
            TenantContext::from_authenticated_session(tenant),
            instructor,
            course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("Student membership reference")
        .items
        .into_iter()
        .next()
        .expect("Student membership")
        .reference;
    let second_assignment = AssignmentId::from_uuid(
        uuid::Uuid::parse_str(
            &created_assignment_id(
                create_assignment(&app, &instructor_cookie, course, &question, "Wrong origin")
                    .await,
            )
            .await,
        )
        .expect("second assignment UUID"),
    );
    let second_assignment = store
        .assignment_reference(
            TenantContext::from_authenticated_session(tenant),
            instructor,
            second_assignment,
        )
        .await
        .expect("second assignment reference lookup")
        .expect("second assignment reference");
    let before = store
        .student_work_inspection_audit_facts()
        .expect("audit facts before inspection");
    let path = format!(
        "/api/courses/{course}/gradebook/students/{membership}/assignments/{assignment_reference}/runs/{run}?operationRef={operation}"
    );
    let rejected_fetch_metadata = app
        .clone()
        .oneshot(
            Request::get(&path)
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("rejected Fetch Metadata request"),
        )
        .await
        .expect("rejected Fetch Metadata response");
    assert_eq!(rejected_fetch_metadata.status(), StatusCode::NOT_FOUND);
    assert_eq!(rejected_fetch_metadata.headers()[CACHE_CONTROL], "no-store");
    let mismatch = app
        .clone()
        .oneshot(same_origin(
            Request::get(format!(
                "/api/courses/{course}/gradebook/students/{membership}/assignments/{second_assignment}/runs/{run}?operationRef={operation}"
            ))
            .header("cookie", &instructor_cookie)
            .body(Body::empty())
            .expect("operation mismatch request"),
        ))
        .await
        .expect("operation mismatch response");
    assert_eq!(mismatch.status(), StatusCode::NOT_FOUND);
    assert_eq!(mismatch.headers()[CACHE_CONTROL], "no-store");
    assert_eq!(
        store
            .student_work_inspection_audit_facts()
            .expect("audit facts after concealed requests"),
        before,
        "rejected Fetch Metadata and an operation-origin mismatch must not inspect or audit Student work"
    );
    let exact = app
        .oneshot(same_origin(
            Request::get(path)
                .header("cookie", instructor_cookie)
                .body(Body::empty())
                .expect("exact inspection request"),
        ))
        .await
        .expect("exact inspection response");
    assert_eq!(exact.status(), StatusCode::OK);
    assert_eq!(exact.headers()[CACHE_CONTROL], "no-store");
    let detail: serde_json::Value = serde_json::from_slice(
        &to_bytes(exact.into_body(), 64 * 1024)
            .await
            .expect("inspection body"),
    )
    .expect("inspection JSON");
    assert_eq!(detail["studentDisplayLabel"], "Inspection Student");
    assert_eq!(detail["assignmentTitle"], "Inspection contract");
    assert_eq!(detail["returnContext"]["kind"], "gradingOperation");
    assert_eq!(detail["returnContext"]["operation"], operation.to_string());
    assert_eq!(
        detail["returnContext"]["focus"]["kind"],
        "gradingOperationControl"
    );
    let after = store
        .student_work_inspection_audit_facts()
        .expect("audit facts after exact inspection");
    assert_eq!(after.0.len(), before.0.len() + 1);
    assert_eq!(after.1.len(), before.1.len() + 1);
}
