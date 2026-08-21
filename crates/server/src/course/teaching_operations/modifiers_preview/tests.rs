use super::*;

use axum::body::{Body, to_bytes};
use axum::http::header::{CACHE_CONTROL, ETAG};
use axum::http::{Request, StatusCode};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CourseRecord, CourseRosterStore, CreateCourseCommand, Store, TeachingAuthorityReferenceStore,
    TenantContext, UpsertCourseMember,
};
use question_model::{
    AssignmentAudience, AssignmentId, AssignmentItem, AssignmentItemId, AssignmentLifecycle,
    AssignmentScoringMode, AssignmentTeachingSettings, CourseId, CourseMembershipReference,
    CourseTerm, PointValue, TenantId, UserId, UserRole,
};
use tower::ServiceExt;
use uuid::Uuid;

use crate::course::tests::fixtures::publish_assignment;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

struct Fixture {
    store: Arc<MemoryStore>,
    app: axum::Router,
    context: TenantContext,
    course: CourseId,
    assignment: AssignmentId,
    instructor_cookie: String,
    student_cookie: String,
    outsider_cookie: String,
    student: CourseMembershipReference,
    group: String,
    accommodation: String,
}

async fn cookie(store: &MemoryStore, tenant: TenantId, user: UserId, role: UserRole) -> String {
    let subject =
        learning_data_access::SessionSubject::new(tenant, user, "Modifier test", vec![role])
            .expect("session");
    crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            learning_data_access::SessionLifetime::from_seconds(3_600).expect("lifetime"),
            crate::auth::CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("issue session")
    .set_cookie
    .split(';')
    .next()
    .expect("cookie")
    .to_owned()
}

async fn json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("bounded response"),
    )
    .expect("JSON response")
}

fn request(method: &str, uri: String, cookie: &str, body: Body) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("cookie", cookie)
        .body(body)
        .expect("request")
}

fn policies() -> question_model::RunPolicies {
    use question_model::run_policy::{
        CompletionRequirement, ContinuedPractice, GradePolicy, VariationPolicy,
    };
    question_model::RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

async fn publish_fixture(
    store: &MemoryStore,
    context: TenantContext,
    tenant: TenantId,
    publisher: UserId,
) -> question_model::ProblemVersionRef {
    use learning_data_access::{CatalogStore, DraftRecord, PublishDraftCommand};
    use question_model::answer::NumericTolerance;
    use question_model::envelope::ContentBlock;
    use question_model::generation::RandomizationDefinition;
    use question_model::response::ResponseDefinition;
    use question_model::taxonomy::License;
    use question_model::{
        BackendCapabilities, Capability, DraftQuestionDefinition, DraftQuestionSource,
        GradingDefinition, ProblemId, PublicationScope, QuestionMetadata, QuestionSource,
        VersionId, WorkspaceId,
    };
    let reference = question_model::ProblemVersionRef {
        problem: ProblemId::from_uuid(id(810)),
        version: VersionId::from_uuid(id(811)),
    };
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(id(812)),
            source: DraftQuestionSource::Native {
                family: "modifier-fixture".to_owned(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "Peptide fixture".to_owned(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
                unit: None,
            },
            attempt_policy: question_model::run_policy::AttemptPolicy { max_attempts: None },
            timing_policy: question_model::run_policy::TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Modifier fixture".to_owned(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_owned(),
            },
        },
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("draft");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: QuestionSource::Native {
                    family: "modifier-fixture".to_owned(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                byline: question_model::PublicByline::new(vec![
                    question_model::PublicAuthorName::new("PLE fixture".to_owned())
                        .expect("byline"),
                ])
                .expect("byline"),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("publish");
    reference
}

fn patch_body(mode: &str, due_at: serde_json::Value) -> Body {
    Body::from(
        serde_json::json!({"mode":mode,"patch":{
        "availableAt":{"kind":"inherit"},"dueAt":due_at,"closesAt":{"kind":"inherit"},
        "timeLimitSeconds":{"kind":"inherit"},"attemptLimit":{"kind":"inherit"}}})
        .to_string(),
    )
}

async fn fixture(audience_group: bool) -> Fixture {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(question_model::ActivityTimestamp::from_unix_millis(
            1_788_000_000_000,
        ))
        .expect("clock");
    let (tenant, course, assignment) = (
        TenantId::from_uuid(id(800)),
        CourseId::from_uuid(id(801)),
        AssignmentId::from_uuid(id(802)),
    );
    let (instructor, student_user, outsider) = (
        UserId::from_uuid(id(803)),
        UserId::from_uuid(id(804)),
        UserId::from_uuid(id(805)),
    );
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "BIOC 301".to_owned(),
                    term: CourseTerm::from_parts("2026-01-01", "2026-12-31", "America/Chicago")
                        .expect("term"),
                },
                initial_instructor: instructor,
            },
        )
        .await
        .expect("course");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student_user,
                display_name: "Student One".to_owned(),
                roster_contact: None,
            },
        )
        .await
        .expect("student");
    store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: outsider,
                display_name: "Student Two".to_owned(),
                roster_contact: None,
            },
        )
        .await
        .expect("second student");
    let instructor_cookie = cookie(&store, tenant, instructor, UserRole::Instructor).await;
    let student_cookie = cookie(&store, tenant, student_user, UserRole::Student).await;
    let outsider_cookie = cookie(&store, tenant, outsider, UserRole::Student).await;
    let student_id = store
        .get_current_course_membership(context, course, student_user)
        .await
        .expect("membership")
        .expect("membership exists")
        .id;
    let student = store
        .course_membership_reference(context, instructor, course, student_id)
        .await
        .expect("member ref")
        .expect("member ref exists");
    let app = crate::course::router(Arc::clone(&store));
    let mut create = request(
        "POST",
        format!("/api/courses/{course}/groups"),
        &instructor_cookie,
        Body::from(
            serde_json::json!({"title":"Lab A","purpose":"lab","members":[student]}).to_string(),
        ),
    );
    create
        .headers_mut()
        .insert("content-type", "application/json".parse().expect("JSON"));
    let created = app.clone().oneshot(create).await.expect("group response");
    assert_eq!(created.status(), StatusCode::CREATED);
    let group = json(created).await["reference"]
        .as_str()
        .expect("group reference")
        .to_owned();
    let mut create = request(
        "POST",
        format!("/api/courses/{course}/groups"),
        &instructor_cookie,
        Body::from(
            serde_json::json!({"title":"Access plan","purpose":"accommodation","members":[student]})
                .to_string(),
        ),
    );
    create
        .headers_mut()
        .insert("content-type", "application/json".parse().expect("JSON"));
    let accommodation = json(
        app.clone()
            .oneshot(create)
            .await
            .expect("accommodation response"),
    )
    .await["reference"]
        .as_str()
        .expect("accommodation reference")
        .to_owned();
    let outsider_id = store
        .get_current_course_membership(context, course, outsider)
        .await
        .expect("second membership")
        .expect("second membership exists")
        .id;
    let outsider_member = store
        .course_membership_reference(context, instructor, course, outsider_id)
        .await
        .expect("second member ref")
        .expect("second member ref exists");
    let mut create = request(
        "POST",
        format!("/api/courses/{course}/groups"),
        &instructor_cookie,
        Body::from(
            serde_json::json!({"title":"Lab B","purpose":"lab","members":[outsider_member]})
                .to_string(),
        ),
    );
    create
        .headers_mut()
        .insert("content-type", "application/json".parse().expect("JSON"));
    let other_group = json(
        app.clone()
            .oneshot(create)
            .await
            .expect("second group response"),
    )
    .await;
    let other_group = other_group["reference"]
        .as_str()
        .expect("second group reference");
    let other_group_id = store
        .get_course_group_by_reference(
            context,
            instructor,
            course,
            other_group.parse().expect("second group ref"),
        )
        .await
        .expect("second group read")
        .expect("second group exists")
        .group
        .record
        .id;
    let reference = publish_fixture(&store, context, tenant, instructor).await;
    let base_policy = question_model::BaseAssignmentPolicy {
        due_at: Some(question_model::ActivityTimestamp::from_unix_millis(
            1_790_000_000_000,
        )),
        ..question_model::BaseAssignmentPolicy::default()
    };
    store
        .create_assignment(
            context,
            learning_data_access::AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                audience: if audience_group {
                    AssignmentAudience::any_of_groups(vec![other_group_id]).expect("one group")
                } else {
                    AssignmentAudience::CourseWide
                },
                title: "Modifier fixture".to_owned(),
                lifecycle: AssignmentLifecycle::Draft,
                instructions: question_model::AssignmentInstructions::default(),
                items: vec![AssignmentItem {
                    id: AssignmentItemId::from_uuid(id(806)),
                    reference,
                    position: 0,
                    points_possible: PointValue::from_whole(1),
                    delivery_state: question_model::AssignmentDeliveryState::Active,
                    scoring_mode: AssignmentScoringMode::Normal,
                }],
                selection_groups: Vec::new(),
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                policies: policies(),
            },
            base_policy,
        )
        .await
        .expect("assignment");
    publish_assignment(
        &store,
        context,
        instructor,
        course,
        assignment,
        AssignmentTeachingSettings {
            lifecycle: AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::default(),
            base_policy,
        },
    )
    .await;
    Fixture {
        store,
        app,
        context,
        course,
        assignment,
        instructor_cookie,
        student_cookie,
        outsider_cookie,
        student,
        group,
        accommodation,
    }
}

async fn mutation(
    fixture: &Fixture,
    method: &str,
    path: &str,
    revision: u64,
    body: Body,
) -> axum::response::Response {
    let mut request = request(
        method,
        format!(
            "/api/courses/{}/assignments/{}/{}",
            fixture.course, fixture.assignment, path
        ),
        &fixture.instructor_cookie,
        body,
    );
    request
        .headers_mut()
        .insert("if-match", format!("\"{revision}\"").parse().expect("ETag"));
    if method == "PUT" {
        request
            .headers_mut()
            .insert("content-type", "application/json".parse().expect("JSON"));
    }
    fixture
        .app
        .clone()
        .oneshot(request)
        .await
        .expect("mutation response")
}

#[tokio::test]
async fn memory_modifier_http_corpus_mutates_m2_m3_m4_with_strong_revisions() {
    let fixture = fixture(false).await;
    let operations = [
        (
            "group-schedule-offsets",
            Body::from(r#"{"offsetSeconds":3600}"#),
        ),
        (
            "group-accommodations",
            patch_body("extendOnly", serde_json::json!({"kind":"unrestricted"})),
        ),
        (
            "individual-policy-exceptions",
            patch_body("override", serde_json::json!({"kind":"unrestricted"})),
        ),
    ];
    let mut revision = fixture
        .store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("exists")
        .revision
        .value();
    for (kind, body) in operations {
        let target = if kind == "individual-policy-exceptions" {
            fixture.student.to_string()
        } else if kind == "group-accommodations" {
            fixture.accommodation.clone()
        } else {
            fixture.group.clone()
        };
        let put = mutation(&fixture, "PUT", &format!("{kind}/{target}"), revision, body).await;
        if put.status() != StatusCode::OK {
            panic!("{kind} PUT failed: {}", json(put).await);
        }
        assert_eq!(put.headers()[CACHE_CONTROL], "no-store");
        revision += 1;
        assert_eq!(put.headers()[ETAG], format!("\"{revision}\""));
        assert_eq!(
            json(put).await,
            serde_json::json!({"revision": revision.to_string()})
        );
        assert_eq!(
            fixture
                .store
                .get_assignment_for_edit(fixture.context, fixture.assignment)
                .await
                .expect("assignment")
                .expect("exists")
                .revision
                .value(),
            revision,
            "{kind} reaches Store"
        );
        let delete = mutation(
            &fixture,
            "DELETE",
            &format!("{kind}/{target}"),
            revision,
            Body::empty(),
        )
        .await;
        assert_eq!(delete.status(), StatusCode::OK);
        revision += 1;
        assert_eq!(delete.headers()[ETAG], format!("\"{revision}\""));
        assert_eq!(
            json(delete).await,
            serde_json::json!({"revision": revision.to_string()})
        );
        assert_eq!(
            fixture
                .store
                .get_assignment_for_edit(fixture.context, fixture.assignment)
                .await
                .expect("assignment")
                .expect("exists")
                .revision
                .value(),
            revision
        );
    }
}

#[tokio::test]
async fn memory_modifier_http_cas_and_invalid_bodies_leave_store_unchanged() {
    let fixture = fixture(false).await;
    let revision = fixture
        .store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("exists")
        .revision
        .value();
    let uri = format!(
        "/api/courses/{}/assignments/{}/group-schedule-offsets/{}",
        fixture.course, fixture.assignment, fixture.group
    );
    for (header, body, status) in [
        (None, Body::from("{}"), StatusCode::PRECONDITION_REQUIRED),
        (
            Some("wrong"),
            Body::from("{}"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (
            Some("\"999\""),
            Body::from("{}"),
            StatusCode::PRECONDITION_FAILED,
        ),
        (
            Some("\"2\""),
            Body::from("not JSON"),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let mut request = request("PUT", uri.clone(), &fixture.instructor_cookie, body);
        request
            .headers_mut()
            .insert("content-type", "application/json".parse().expect("JSON"));
        if let Some(header) = header {
            request
                .headers_mut()
                .insert("if-match", header.parse().expect("header"));
        }
        let response = fixture
            .app
            .clone()
            .oneshot(request)
            .await
            .expect("response");
        assert_eq!(response.status(), status);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    }
    assert_eq!(
        fixture
            .store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment")
            .expect("exists")
            .revision
            .value(),
        revision
    );

    let mut revision_after_setup = revision;
    for (path, valid_body) in [
        (
            format!("group-accommodations/{}", fixture.accommodation),
            patch_body("extendOnly", serde_json::json!({"kind":"unrestricted"})),
        ),
        (
            format!("individual-policy-exceptions/{}", fixture.student),
            patch_body("override", serde_json::json!({"kind":"unrestricted"})),
        ),
    ] {
        let created = mutation(&fixture, "PUT", &path, revision_after_setup, valid_body).await;
        assert_eq!(created.status(), StatusCode::OK, "{path}");
        revision_after_setup += 1;
        let stale = mutation(
            &fixture,
            "PUT",
            &path,
            revision_after_setup + 1,
            Body::from(r#"{"mode":"override","patch":{"dueAt":{"kind":"unrestricted"}}}"#),
        )
        .await;
        assert_eq!(stale.status(), StatusCode::PRECONDITION_FAILED, "{path}");
        assert_eq!(stale.headers()[CACHE_CONTROL], "no-store", "{path}");
        let malformed = mutation(
            &fixture,
            "PUT",
            &path,
            revision_after_setup,
            Body::from("not JSON"),
        )
        .await;
        assert_eq!(
            malformed.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{path}"
        );
        assert_eq!(malformed.headers()[CACHE_CONTROL], "no-store", "{path}");
    }
    assert_eq!(
        fixture
            .store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment")
            .expect("exists")
            .revision
            .value(),
        revision_after_setup
    );
}

#[tokio::test]
async fn memory_preview_projects_base_group_and_individual_safe_provenance() {
    let fixture = fixture(false).await;
    let mut revision = fixture
        .store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("exists")
        .revision
        .value();
    let preview = |fixture: &Fixture| {
        request(
            "GET",
            format!(
                "/api/courses/{}/assignments/{}/policy-preview/{}",
                fixture.course, fixture.assignment, fixture.student
            ),
            &fixture.instructor_cookie,
            Body::empty(),
        )
    };
    let base = json(
        fixture
            .app
            .clone()
            .oneshot(preview(&fixture))
            .await
            .expect("base preview"),
    )
    .await;
    assert_eq!(base["entitlement"], "allowed");
    assert_eq!(base["dueAt"]["source"]["kind"], "base");
    let schedule = mutation(
        &fixture,
        "PUT",
        &format!("group-schedule-offsets/{}", fixture.group),
        revision,
        Body::from(r#"{"offsetSeconds":3600}"#),
    )
    .await;
    if schedule.status() != StatusCode::OK {
        panic!("schedule PUT failed: {}", json(schedule).await);
    }
    revision += 1;
    assert_eq!(schedule.headers()[ETAG], format!("\"{revision}\""));
    let scheduled = json(
        fixture
            .app
            .clone()
            .oneshot(preview(&fixture))
            .await
            .expect("schedule preview"),
    )
    .await;
    assert_eq!(scheduled["dueAt"]["source"]["kind"], "groupScheduleOffsets");
    assert_eq!(scheduled["dueAt"]["source"]["groups"][0]["label"], "Lab A");
    let accommodation = mutation(
        &fixture,
        "PUT",
        &format!("group-accommodations/{}", fixture.accommodation),
        revision,
        patch_body("extendOnly", serde_json::json!({"kind":"unrestricted"})),
    )
    .await;
    revision += 1;
    assert_eq!(accommodation.headers()[ETAG], format!("\"{revision}\""));
    let grouped = json(
        fixture
            .app
            .clone()
            .oneshot(preview(&fixture))
            .await
            .expect("group preview"),
    )
    .await;
    assert_eq!(grouped["dueAt"]["source"]["kind"], "groupAccommodations");
    assert_eq!(
        grouped["dueAt"]["source"]["groups"][0]["label"],
        "Access plan"
    );
    assert!(!grouped.to_string().contains("00000000"));
    let individual = mutation(
        &fixture,
        "PUT",
        &format!("individual-policy-exceptions/{}", fixture.student),
        revision,
        patch_body("override", serde_json::json!({"kind":"unrestricted"})),
    )
    .await;
    assert_eq!(individual.status(), StatusCode::OK);
    let individual = json(
        fixture
            .app
            .clone()
            .oneshot(preview(&fixture))
            .await
            .expect("individual preview"),
    )
    .await;
    assert_eq!(individual["dueAt"]["source"]["kind"], "membership");
    assert_eq!(individual["dueAt"]["source"]["label"], "Student One");
    assert!(
        individual["dueAt"]["source"]["membership"]
            .as_str()
            .expect("reference")
            .starts_with("M-")
    );
}

#[tokio::test]
async fn hypothetical_source_fails_closed_without_membership_projection() {
    let response = hypothetical_source_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = json(response).await;
    assert_eq!(
        body,
        serde_json::json!({"error":"preview provenance is invalid"})
    );
    assert!(!body.to_string().contains("membership"));
}

#[tokio::test]
async fn memory_preview_denial_has_no_s3_or_provenance_and_denials_precede_body() {
    let denied = fixture(true).await;
    let response = denied
        .app
        .clone()
        .oneshot(request(
            "GET",
            format!(
                "/api/courses/{}/assignments/{}/policy-preview/{}",
                denied.course, denied.assignment, denied.student
            ),
            &denied.instructor_cookie,
            Body::empty(),
        ))
        .await
        .expect("denied preview");
    assert_eq!(
        json(response).await,
        serde_json::json!({"entitlement":"denied", "reason":"notEntitled"})
    );
    for cookie in [&denied.student_cookie, &denied.outsider_cookie] {
        let response = denied
            .app
            .clone()
            .oneshot(request(
                "PUT",
                format!(
                    "/api/courses/{}/assignments/{}/group-schedule-offsets/{}",
                    denied.course, denied.assignment, denied.group
                ),
                cookie,
                Body::from("not JSON"),
            ))
            .await
            .expect("denied response");
        assert_ne!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
    }
}

#[tokio::test]
async fn memory_course_local_modifier_refusals_are_atomic_and_preview_is_local() {
    let fixture = fixture(false).await;
    let revision = fixture
        .store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("assignment")
        .expect("assignment exists")
        .revision
        .value();
    let path = format!("group-accommodations/{}", fixture.accommodation);
    for (field, reason, value) in [
        ("dueAt", "nonexistentLocalTime", "2026-03-08T02:30:00.000"),
        ("dueAt", "ambiguousLocalTime", "2026-11-01T01:30:00.000"),
        ("closesAt", "outsideCourseTerm", "2027-01-01T10:00:00.000"),
    ] {
        let mut patch = serde_json::json!({
            "availableAt":{"kind":"inherit"},
            "dueAt":{"kind":"inherit"},
            "closesAt":{"kind":"inherit"},
            "timeLimitSeconds":{"kind":"inherit"},
            "attemptLimit":{"kind":"inherit"}
        });
        patch[field] = serde_json::json!({"kind":"set","value":value});
        let response = mutation(
            &fixture,
            "PUT",
            &path,
            revision,
            Body::from(serde_json::json!({"mode":"override","patch":patch}).to_string()),
        )
        .await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
        let failure = json(response).await;
        assert_eq!(failure["field"], field);
        assert_eq!(failure["reason"], reason);
        assert_eq!(
            fixture
                .store
                .get_assignment_for_edit(fixture.context, fixture.assignment)
                .await
                .expect("assignment")
                .expect("assignment exists")
                .revision
                .value(),
            revision,
            "invalid local schedule must not reach the Store"
        );
    }
    let exact = mutation(
        &fixture,
        "PUT",
        &path,
        revision,
        patch_body(
            "override",
            serde_json::json!({"kind":"set","value":"2026-09-01T10:04:05.123"}),
        ),
    )
    .await;
    assert_eq!(exact.status(), StatusCode::OK);
    let preview = json(
        fixture
            .app
            .clone()
            .oneshot(request(
                "GET",
                format!(
                    "/api/courses/{}/assignments/{}/policy-preview/{}",
                    fixture.course, fixture.assignment, fixture.student
                ),
                &fixture.instructor_cookie,
                Body::empty(),
            ))
            .await
            .expect("preview"),
    )
    .await;
    assert_eq!(preview["entitlement"], "allowed");
    assert_eq!(preview["timeZone"], "America/Chicago");
    assert_eq!(preview["dueAt"]["value"], "2026-09-01T10:04:05.123");
    assert!(preview["dueAt"]["value"].is_string());
    assert!(!preview.to_string().contains("1790000000000"));
}
