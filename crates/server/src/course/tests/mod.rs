use super::*;
use axum::body::{Body, to_bytes};
use axum::http::Request;
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CatalogStore, DraftRecord, JobLeaseDuration, JobPayload, JobStore, PublishDraftCommand,
    RetentionWorkerCommand, RetentionWorkerStore, SessionLifetime, SessionSubject, TenantContext,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::RandomizationDefinition;
use question_model::response::ResponseDefinition;
use question_model::run_policy::{
    AttemptPolicy, CompletionRequirement, ContinuedPractice, FeedbackDisclosure, GradePolicy,
    TimingPolicy, VariationPolicy,
};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, BackendCapabilities, Capability, DraftQuestionDefinition,
    DraftQuestionSource, GradingDefinition, ObjectId, ProblemId, PublicationScope,
    QuestionMetadata, QuestionSource, StudentId, TenantId, UserId, VersionId, WorkspaceId,
};
use tower::ServiceExt;
use uuid::Uuid;

mod roster;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

async fn issued_cookie(store: &MemoryStore, roles: Vec<UserRole>, user: UserId) -> String {
    issued_cookie_for_tenant(store, TenantId::from_uuid(id(1)), roles, user).await
}

async fn issued_cookie_for_tenant(
    store: &MemoryStore,
    tenant: TenantId,
    roles: Vec<UserRole>,
    user: UserId,
) -> String {
    let subject =
        SessionSubject::new(tenant, user, "Course Fixture", roles).expect("fixture identity");
    let issued = crate::auth::issue_session(
        store,
        subject,
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("positive lifetime"),
            crate::auth::CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("fixture session");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

async fn response_json(response: Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 128 * 1_024)
        .await
        .expect("response body");
    serde_json::from_slice(&body).expect("JSON response")
}

async fn publish_fixture(
    store: &MemoryStore,
    context: TenantContext,
    tenant: TenantId,
    publisher: UserId,
) -> ProblemVersionRef {
    let problem = ProblemId::from_uuid(id(20));
    let version = VersionId::from_uuid(id(21));
    let workspace = WorkspaceId::from_uuid(id(22));
    let draft = DraftRecord {
        tenant,
        question: DraftQuestionDefinition {
            workspace,
            source: DraftQuestionSource::Native {
                family: "course-fixture".to_string(),
            },
            prompt: vec![ContentBlock::Text {
                markdown: "What is a peptide bond?".to_string(),
            }],
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.0 },
                unit: None,
            },
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateFull,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Peptide bond fixture".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBySa,
                language: "en-US".to_string(),
            },
        },
        revises: None,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, publisher, None, draft.clone())
        .await
        .expect("draft save");
    store
        .publish_draft(
            context,
            publisher,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: question_model::ProblemVersionRef { problem, version },
                published_source: QuestionSource::Native {
                    family: "course-fixture".to_string(),
                },
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher,
                scope: PublicationScope::Public,
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("fixture publication");
    ProblemVersionRef { problem, version }
}

fn policies() -> RunPolicies {
    RunPolicies {
        completion: CompletionRequirement::AllCorrect,
        grade: GradePolicy::Highest,
        continued_practice: ContinuedPractice::Unlimited,
        variation: VariationPolicy::NewSeeds,
    }
}

#[tokio::test]
async fn membership_scopes_courses_and_exact_assignment_references_survive() {
    let store = Arc::new(MemoryStore::default());
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(id(1));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(id(2));
    let student = UserId::from_uuid(id(3));
    let outsider = UserId::from_uuid(id(4));
    let administrator = UserId::from_uuid(id(5));
    let foreign_tenant = TenantId::from_uuid(id(6));
    let foreign_user = UserId::from_uuid(id(7));
    let instructor_cookie = issued_cookie(&store, vec![UserRole::Instructor], instructor).await;
    let student_cookie = issued_cookie(&store, vec![UserRole::Student], student).await;
    let outsider_cookie = issued_cookie(&store, vec![UserRole::Instructor], outsider).await;
    let administrator_cookie =
        issued_cookie(&store, vec![UserRole::Administrator], administrator).await;
    let foreign_cookie = issued_cookie_for_tenant(
        &store,
        foreign_tenant,
        vec![UserRole::Instructor],
        foreign_user,
    )
    .await;
    let app = router(Arc::clone(&store));

    let created_course = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"BIOC 301: Biochemistry"}"#))
                .expect("course request"),
        )
        .await
        .expect("course response");
    assert_eq!(created_course.status(), StatusCode::CREATED);
    let created_course = response_json(created_course).await;
    let course: CourseId =
        serde_json::from_value(created_course["id"].clone()).expect("course ID response");
    assert_eq!(created_course["role"], "instructor");

    let mut course_record = store
        .get_course(context, course)
        .await
        .expect("course lookup")
        .expect("course exists");
    course_record.members.push(CourseMembership {
        user: student,
        role: CourseMembershipRole::Student,
    });
    store
        .upsert_course(context, course_record)
        .await
        .expect("student membership save");
    let reference = publish_fixture(&store, context, tenant, instructor).await;

    let assignment_request = CreateAssignmentRequest {
        title: "Peptide bond mastery".to_string(),
        problems: vec![reference],
        policies: policies(),
    };
    let created_assignment = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/courses/{course}/assignments"))
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&assignment_request)
                        .expect("assignment request serialization"),
                ))
                .expect("assignment request"),
        )
        .await
        .expect("assignment response");
    assert_eq!(created_assignment.status(), StatusCode::CREATED);
    let assignment_etag = created_assignment
        .headers()
        .get(ETAG)
        .expect("created assignment ETag")
        .to_str()
        .expect("ASCII ETag")
        .to_string();
    let created_assignment = response_json(created_assignment).await;
    let assignment: AssignmentId =
        serde_json::from_value(created_assignment["id"].clone()).expect("assignment ID response");
    assert_eq!(created_assignment["courseId"], serde_json::json!(course));
    assert_eq!(
        created_assignment["items"][0]["reference"],
        serde_json::json!(reference),
        "the stable assignment item must retain exact IDs rather than copy a question"
    );
    assert!(created_assignment["items"][0]["id"].is_string());

    for request in [
        Request::builder()
            .uri(format!("/api/assignments/{assignment}"))
            .header("cookie", &foreign_cookie)
            .body(Body::empty())
            .expect("foreign exact request"),
        Request::builder()
            .method("PUT")
            .uri(format!("/api/courses/{course}/assignments/{assignment}"))
            .header("cookie", &foreign_cookie)
            .header(IF_MATCH, &assignment_etag)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "foreign", "problems": [reference], "policies": policies(),
                })
                .to_string(),
            ))
            .expect("foreign update request"),
        Request::builder()
            .method("PUT")
            .uri(format!("/api/courses/{course}/assignments/{assignment}"))
            .header("cookie", &foreign_cookie)
            .header(IF_MATCH, "W/\"1\"")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "title": "foreign malformed", "problems": [reference], "policies": policies(),
                })
                .to_string(),
            ))
            .expect("foreign malformed update request"),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request)
                .await
                .expect("foreign response")
                .status(),
            StatusCode::NOT_FOUND,
            "foreign tenant must not enumerate an assignment"
        );
    }

    let nested_unknown = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &instructor_cookie)
                    .header(IF_MATCH, &assignment_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "Peptide bond mastery",
                        "problems": [{"problem": reference.problem, "version": reference.version, "capabilities": ["serverGrading"]}],
                        "policies": policies(),
                    }).to_string()))
                    .expect("nested unknown request"),
            )
            .await
            .expect("nested unknown response");
    assert_eq!(nested_unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &instructor_cookie)
                .header(IF_MATCH, &assignment_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "Peptide bond mastery revised",
                        "problems": [reference],
                        "policies": policies(),
                    })
                    .to_string(),
                ))
                .expect("assignment update request"),
        )
        .await
        .expect("assignment update response");
    assert_eq!(updated.status(), StatusCode::OK);
    let updated_etag = updated.headers().get(ETAG).expect("updated ETag");
    assert_ne!(updated_etag.to_str().expect("ASCII ETag"), assignment_etag);

    let stale = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &instructor_cookie)
                .header(IF_MATCH, &assignment_etag)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "title": "stale overwrite", "problems": [reference], "policies": policies(),
                    })
                    .to_string(),
                ))
                .expect("stale update request"),
        )
        .await
        .expect("stale update response");
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(
        store
            .get_assignment(context, assignment)
            .await
            .expect("stored assignment")
            .expect("assignment")
            .title,
        "Peptide bond mastery revised"
    );

    let administrator_get = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/assignments/{assignment}"))
                .header("cookie", &administrator_cookie)
                .body(Body::empty())
                .expect("administrator assignment request"),
        )
        .await
        .expect("administrator assignment response");
    assert_eq!(administrator_get.status(), StatusCode::OK);
    let administrator_etag = administrator_get
        .headers()
        .get(ETAG)
        .expect("administrator ETag");
    let administrator_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &administrator_cookie)
                    .header(IF_MATCH, administrator_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "Administrator revised", "problems": [reference], "policies": policies(),
                    }).to_string()))
                    .expect("administrator update request"),
            )
            .await
            .expect("administrator update response");
    assert_eq!(administrator_update.status(), StatusCode::OK);

    let wrong_course = CourseId::from_uuid(id(99));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: wrong_course,
                tenant,
                title: "BIOC 399: Wrong course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("wrong-course fixture");
    let wrong_course_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{wrong_course}/assignments/{assignment}"))
                    .header("cookie", &instructor_cookie)
                    .header(IF_MATCH, updated_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "must not move course", "problems": [reference], "policies": policies(),
                    }).to_string()))
                    .expect("wrong-course update request"),
            )
            .await
            .expect("wrong-course update response");
    assert_eq!(wrong_course_update.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        store
            .get_assignment(context, assignment)
            .await
            .expect("stored assignment")
            .expect("assignment")
            .course_id,
        course
    );

    store
        .create_enrollment(
            context,
            question_model::AssignmentEnrollment {
                id: question_model::EnrollmentId::from_uuid(id(40)),
                tenant,
                assignment,
                user: student,
                student: StudentId::from_uuid(id(41)),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("gradebook fixture enrollment");

    let gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("gradebook request"),
        )
        .await
        .expect("gradebook response");
    assert_eq!(gradebook.status(), StatusCode::OK);
    assert_eq!(
        gradebook
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok()),
        Some("no-store")
    );
    let gradebook = response_json(gradebook).await;
    let rows = gradebook["items"].as_array().expect("gradebook rows");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    let row_fields: std::collections::BTreeSet<_> = row
        .as_object()
        .expect("gradebook row object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        row_fields,
        std::collections::BTreeSet::from([
            "tenant",
            "courseId",
            "enrollmentId",
            "studentId",
            "assignmentId",
            "assignmentTitle",
            "summary",
        ])
    );
    assert_eq!(row["summary"]["tenant"], row["tenant"]);

    let administrator_gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &administrator_cookie)
                .body(Body::empty())
                .expect("administrator gradebook request"),
        )
        .await
        .expect("administrator gradebook response");
    assert_eq!(administrator_gradebook.status(), StatusCode::OK);

    let student_gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student gradebook request"),
        )
        .await
        .expect("student gradebook response");
    assert_eq!(student_gradebook.status(), StatusCode::FORBIDDEN);

    let outsider_gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("outsider gradebook request"),
        )
        .await
        .expect("outsider gradebook response");
    assert_eq!(outsider_gradebook.status(), StatusCode::NOT_FOUND);

    let second_assignment = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/courses/{course}/assignments"))
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&assignment_request)
                        .expect("second assignment request serialization"),
                ))
                .expect("second assignment request"),
        )
        .await
        .expect("second assignment response");
    assert_eq!(second_assignment.status(), StatusCode::CREATED);

    let second_course = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title":"BIOC 302: Enzymes"}"#))
                .expect("second course request"),
        )
        .await
        .expect("second course response");
    assert_eq!(second_course.status(), StatusCode::CREATED);
    let second_course = response_json(second_course).await;
    let second_course: CourseId =
        serde_json::from_value(second_course["id"].clone()).expect("second course ID response");
    let mut second_course_record = store
        .get_course(context, second_course)
        .await
        .expect("second course lookup")
        .expect("second course exists");
    second_course_record.members.push(CourseMembership {
        user: student,
        role: CourseMembershipRole::Student,
    });
    store
        .upsert_course(context, second_course_record)
        .await
        .expect("second student membership save");

    let student_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses?pageSize=1")
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student courses request"),
        )
        .await
        .expect("student courses response");
    let student_courses = response_json(student_courses).await;
    assert_eq!(student_courses["items"][0]["role"], "student");
    assert_eq!(student_courses["items"].as_array().map(Vec::len), Some(1));
    let course_cursor = student_courses["nextCursor"]
        .as_str()
        .expect("course continuation cursor");
    let continued_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses?pageSize=1&cursor={course_cursor}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("course continuation request"),
        )
        .await
        .expect("course continuation response");
    assert_eq!(continued_courses.status(), StatusCode::OK);
    let continued_courses = response_json(continued_courses).await;
    assert_eq!(continued_courses["items"].as_array().map(Vec::len), Some(1));
    assert_ne!(student_courses["items"][0], continued_courses["items"][0]);
    assert_eq!(continued_courses["nextCursor"], serde_json::Value::Null);

    let exact_course = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("exact course request"),
        )
        .await
        .expect("exact course response");
    assert_eq!(exact_course.status(), StatusCode::OK);
    assert_eq!(response_json(exact_course).await["role"], "student");

    let student_assignments = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/assignments?pageSize=1"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("student assignments request"),
        )
        .await
        .expect("student assignments response");
    assert_eq!(student_assignments.status(), StatusCode::OK);
    let student_assignments = response_json(student_assignments).await;
    assert_eq!(
        student_assignments["items"].as_array().map(Vec::len),
        Some(1)
    );
    let assignment_cursor = student_assignments["nextCursor"]
        .as_str()
        .expect("assignment continuation cursor");
    let continued_assignments = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/courses/{course}/assignments?pageSize=1&cursor={assignment_cursor}"
                ))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("assignment continuation request"),
        )
        .await
        .expect("assignment continuation response");
    assert_eq!(continued_assignments.status(), StatusCode::OK);
    let continued_assignments = response_json(continued_assignments).await;
    assert_eq!(
        continued_assignments["items"].as_array().map(Vec::len),
        Some(1)
    );
    assert_ne!(
        student_assignments["items"][0],
        continued_assignments["items"][0]
    );
    assert_eq!(continued_assignments["nextCursor"], serde_json::Value::Null);

    for path in [
        "/api/courses".to_string(),
        format!("/api/courses/{course}/assignments"),
    ] {
        for query in ["pageSize=0", "pageSize=101", "cursor=", "offset=1"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("{path}?{query}"))
                        .header("cookie", &student_cookie)
                        .body(Body::empty())
                        .expect("invalid pagination request"),
                )
                .await
                .expect("invalid pagination response");
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "{path}?{query} must be rejected"
            );
        }
    }

    let exact = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/assignments/{assignment}"))
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("exact assignment request"),
        )
        .await
        .expect("exact assignment response");
    assert_eq!(exact.status(), StatusCode::OK);

    let outsider_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses")
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("outsider courses request"),
        )
        .await
        .expect("outsider courses response");
    assert!(
        response_json(outsider_courses).await["items"]
            .as_array()
            .expect("course items")
            .is_empty()
    );

    let hidden_course = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}"))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("hidden course request"),
        )
        .await
        .expect("hidden course response");
    assert_eq!(hidden_course.status(), StatusCode::NOT_FOUND);

    let hidden = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/assignments"))
                .header("cookie", &outsider_cookie)
                .body(Body::empty())
                .expect("hidden assignments request"),
        )
        .await
        .expect("hidden assignments response");
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);

    let student_update = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &student_cookie)
                    .header(IF_MATCH, &assignment_etag)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({
                        "title": "student overwrite", "problems": [reference], "policies": policies(),
                    }).to_string()))
                    .expect("student update request"),
            )
            .await
            .expect("student update response");
    assert_eq!(student_update.status(), StatusCode::FORBIDDEN);

    let student_missing_revision = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                    .header("cookie", &student_cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "title": "student missing revision", "problems": [reference], "policies": policies(),
                        })
                        .to_string(),
                    ))
                    .expect("student missing revision request"),
            )
            .await
            .expect("student missing revision response");
    assert_eq!(student_missing_revision.status(), StatusCode::FORBIDDEN);

    let student_write = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/courses/{course}/assignments"))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&assignment_request)
                        .expect("assignment request serialization"),
                ))
                .expect("student write request"),
        )
        .await
        .expect("student write response");
    assert_eq!(student_write.status(), StatusCode::FORBIDDEN);

    store
        .seed_retention_cleanup_for_test(
            tenant,
            course,
            (0..4)
                .map(|offset| ObjectId::from_uuid(id(100 + offset)))
                .collect(),
        )
        .expect("archive cleanup fixture");
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease duration"),
        )
        .await
        .expect("archive claim")
        .expect("archive job");
    let (stage, generation) = match claim.payload {
        JobPayload::Retention {
            course: claimed_course,
            stage,
            generation,
        } => {
            assert_eq!(claimed_course, course);
            (stage, generation)
        }
        _ => panic!("fixture must claim retention work"),
    };
    store
        .prepare_retention_work(RetentionWorkerCommand {
            tenant,
            course,
            stage,
            generation,
            job: claim.id,
            lease: claim.lease_token,
        })
        .await
        .expect("archive prepare fence");

    for uri in [
        format!("/api/courses/{course}"),
        format!("/api/courses/{course}/assignments"),
        format!("/api/assignments/{assignment}"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("cookie", &student_cookie)
                    .body(Body::empty())
                    .expect("archived learner request"),
            )
            .await
            .expect("archived learner response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("cache-control"),
            Some(&HeaderValue::from_static("no-store"))
        );
    }

    let student_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses")
                .header("cookie", &student_cookie)
                .body(Body::empty())
                .expect("archived learner course list"),
        )
        .await
        .expect("archived learner course response");
    let student_courses = response_json(student_courses).await;
    assert!(
        student_courses["items"]
            .as_array()
            .expect("course items")
            .iter()
            .all(|item| item["id"] != serde_json::json!(course)),
        "archived course leaked into learner list: {student_courses}"
    );

    let instructor_courses = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/courses")
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("retained manager course list"),
        )
        .await
        .expect("retained manager course response");
    let instructor_courses = response_json(instructor_courses).await;
    assert!(
        instructor_courses["items"]
            .as_array()
            .expect("course items")
            .iter()
            .any(|item| item["id"] == serde_json::json!(course)),
        "retained course missing from manager list: {instructor_courses}"
    );

    for (cookie, uri) in [
        (&instructor_cookie, format!("/api/courses/{course}")),
        (
            &instructor_cookie,
            format!("/api/courses/{course}/assignments"),
        ),
        (&instructor_cookie, format!("/api/assignments/{assignment}")),
        (&administrator_cookie, format!("/api/courses/{course}")),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header("cookie", cookie)
                    .body(Body::empty())
                    .expect("retained manager definition request"),
            )
            .await
            .expect("retained manager definition response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let archived_gradebook = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/courses/{course}/gradebook"))
                .header("cookie", &instructor_cookie)
                .body(Body::empty())
                .expect("archived gradebook request"),
        )
        .await
        .expect("archived gradebook response");
    assert_eq!(archived_gradebook.status(), StatusCode::NOT_FOUND);

    let archived_student_update = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/courses/{course}/assignments/{assignment}"))
                .header("cookie", &student_cookie)
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("archived learner update request"),
        )
        .await
        .expect("archived learner update response");
    assert_eq!(archived_student_update.status(), StatusCode::NOT_FOUND);
}

#[test]
fn assignment_revision_requires_one_positive_strong_etag() {
    let accepted = HeaderMap::from_iter([(IF_MATCH, HeaderValue::from_static("\"7\""))]);
    assert_eq!(
        required_assignment_revision(&accepted).expect("strong revision"),
        serde_json::from_str("7").expect("revision")
    );
    for value in ["7", "W/\"7\"", "\"0\"", "\"-1\"", "\"9223372036854775808\""] {
        let headers =
            HeaderMap::from_iter([(IF_MATCH, HeaderValue::from_str(value).expect("test header"))]);
        assert_eq!(
            required_assignment_revision(&headers),
            Err(AssignmentRevisionHeaderError::Malformed)
        );
    }
    assert_eq!(
        required_assignment_revision(&HeaderMap::new()),
        Err(AssignmentRevisionHeaderError::Missing)
    );
}
