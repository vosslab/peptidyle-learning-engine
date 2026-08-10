use super::*;

#[test]
fn help_has_no_secret_or_browser_seed_endpoint() {
    assert!(USAGE.contains("e2e-seed"));
    assert!(!USAGE.contains("token"));
    assert!(!USAGE.contains("answer"));
    assert!(!USAGE.contains("SECRET_ACCESS_KEY"));
    assert!(!USAGE.contains("secret-access-key"));
}

#[test]
fn parsing_requires_distinct_course_members() {
    let id = "00000000-0000-0000-0000-000000000001".to_string();
    let result = parse_arguments(&[
        "--database-url".to_string(),
        "postgres://example".to_string(),
        "--tenant".to_string(),
        id.clone(),
        "--instructor".to_string(),
        id.clone(),
        "--student".to_string(),
        id,
    ]);
    assert!(result.is_err());
}

#[test]
fn ids_are_stable_and_separated_by_purpose() {
    let tenant = TenantId::from_uuid(Uuid::from_u128(9));
    let first = SeedIds::for_tenant(tenant);
    let second = SeedIds::for_tenant(tenant);
    assert_eq!(first.assignment, second.assignment);
    assert_ne!(first.assignment.as_uuid(), first.enrollment.as_uuid());
    assert_ne!(first.problem.as_uuid(), first.version.as_uuid());
}

#[test]
fn webwork_pilot_enrollment_rerun_preserves_server_owned_progress() {
    let tenant = TenantId::from_uuid(Uuid::from_u128(20));
    let ids = SeedIds::for_tenant(tenant);
    let user = UserId::from_uuid(Uuid::from_u128(21));
    let expected = AssignmentEnrollment {
        id: ids.enrollment,
        tenant,
        assignment: ids.assignment,
        user,
        student: StudentId::from_uuid(user.as_uuid()),
        first_completed_at: None,
        current_grade_run: None,
        best_grade_run: None,
    };
    let mut progressed = expected.clone();
    progressed.first_completed_at = Some(ActivityTimestamp::from_unix_millis(1));
    progressed.current_grade_run = Some(RunId::from_uuid(Uuid::from_u128(22)));
    progressed.best_grade_run = Some(RunId::from_uuid(Uuid::from_u128(23)));

    assert!(webwork_pilot_enrollment_identity_matches(
        &progressed,
        &expected
    ));

    progressed.user = UserId::from_uuid(Uuid::from_u128(24));
    assert!(!webwork_pilot_enrollment_identity_matches(
        &progressed,
        &expected
    ));
}

#[test]
fn native_seed_matches_catalog_publication_capability_admission() {
    let draft = native_draft(WorkspaceId::from_uuid(Uuid::from_u128(12)));
    let violations = domain::policy::validate_draft_for_publication(
        &draft,
        &native_capabilities().expect("registered native family has capabilities"),
    );

    assert!(
        violations.is_empty(),
        "the host seed must pass the same capability check as catalog publication: {violations:?}"
    );
}

#[test]
fn webwork_pilot_requires_all_host_storage_coordinates_without_secret_arguments() {
    let tenant = "00000000-0000-0000-0000-000000000001".to_string();
    let instructor = "00000000-0000-0000-0000-000000000002".to_string();
    let student = "00000000-0000-0000-0000-000000000003".to_string();
    let result = parse_arguments(&[
        "--database-url".to_string(),
        "postgres://example".to_string(),
        "--tenant".to_string(),
        tenant,
        "--instructor".to_string(),
        instructor,
        "--student".to_string(),
        student,
        "--apply-migrations".to_string(),
        "--webwork-pilot".to_string(),
        "--s3-endpoint".to_string(),
        "http://127.0.0.1:9000".to_string(),
    ]);
    let error = result.expect_err("partial WebWork storage settings must refuse");
    assert!(error.to_string().contains("--content-bucket"));
    assert!(!error.to_string().contains("AWS_SECRET_ACCESS_KEY"));
}

#[test]
fn webwork_pilot_storage_settings_are_opt_in_and_deterministic() {
    let parsed = parse_arguments(&[
        "--database-url".to_string(),
        "postgres://example".to_string(),
        "--tenant".to_string(),
        "00000000-0000-0000-0000-000000000001".to_string(),
        "--instructor".to_string(),
        "00000000-0000-0000-0000-000000000002".to_string(),
        "--student".to_string(),
        "00000000-0000-0000-0000-000000000003".to_string(),
        "--apply-migrations".to_string(),
        "--webwork-pilot".to_string(),
        "--s3-endpoint".to_string(),
        "http://127.0.0.1:9000".to_string(),
        "--s3-region".to_string(),
        "us-east-1".to_string(),
        "--content-bucket".to_string(),
        "content".to_string(),
    ])
    .expect("complete opt-in settings parse");
    let storage = parsed
        .webwork_pilot
        .expect("WebWork pilot settings retained");
    assert_eq!(storage.endpoint_url, "http://127.0.0.1:9000/");
    assert_eq!(storage.region, "us-east-1");
    assert_eq!(storage.content_bucket, "content");
}

#[test]
fn tracked_webwork_fixture_matches_declared_digest_and_provenance() {
    assert_eq!(
        objects::Sha256Digest::compute(WEBWORK_PILOT_SOURCE).to_string(),
        WEBWORK_PILOT_SOURCE_SHA256
    );
    let provenance: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/pilot/webwork/which_hydrophobic-simple.provenance.json"
    )))
    .expect("tracked provenance is JSON");
    assert_eq!(provenance["sha256"], WEBWORK_PILOT_SOURCE_SHA256);
    assert_eq!(
        provenance["copiedFrom"],
        "OTHER_REPOS/biology-problems-website/site_docs/biochemistry/topic01/downloads/which_hydrophobic-simple.pgml"
    );
    assert_eq!(
        provenance["title"],
        "Biochemistry: Identify hydrophobic compounds from formulas"
    );
    assert_eq!(provenance["license"], "CC-BY-4.0");
    assert_eq!(
        provenance["licenseUrl"],
        "https://creativecommons.org/licenses/by/4.0/"
    );
    assert_eq!(provenance["author"], "Dr. Neil R. Voss");
    assert_eq!(provenance["institution"], "Roosevelt University");
    assert_eq!(provenance["date"], "2026-01-23");
    assert!(
        String::from_utf8_lossy(WEBWORK_PILOT_SOURCE)
            .contains("# Source code portions are licensed under LGPLv3.")
    );
}

#[test]
fn webwork_pilot_draft_uses_immutable_source_and_declared_capabilities() {
    let draft = webwork_pilot_draft(WorkspaceId::from_uuid(Uuid::from_u128(12)));
    assert_eq!(
        draft.source,
        DraftQuestionSource::Webwork {
            pg_path: WEBWORK_PILOT_SOURCE_PATH.to_string(),
        }
    );
    let capabilities = webwork_capabilities();
    assert!(capabilities.supports(Capability::AlgorithmicGeneration));
    assert!(capabilities.supports(Capability::ServerGrading));
    assert!(!capabilities.supports(Capability::PartialCredit));
    assert_eq!(draft.attempt_policy.feedback, FeedbackDisclosure::Deferred);
    assert!(domain::policy::validate_draft_for_publication(&draft, &capabilities).is_empty());
}

#[test]
fn webwork_pilot_published_source_binds_one_deterministic_problem_object_key() {
    let tenant = TenantId::from_uuid(Uuid::from_u128(9));
    let ids = WebworkPilotSeedIds::for_tenant(tenant);
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    assert_eq!(
        webwork_pilot_published_source(),
        QuestionSource::Webwork {
            pg_path: WEBWORK_PILOT_SOURCE_PATH.to_string(),
        }
    );
    assert_eq!(
        webwork_pilot_source_key(reference, ids.source_object),
        ObjectKey::ProblemSource {
            problem: ids.problem,
            version: ids.version,
            object: ids.source_object,
        }
    );
}

#[test]
fn webwork_pilot_ids_are_stable_and_disjoint_from_native_seed() {
    let tenant = TenantId::from_uuid(Uuid::from_u128(9));
    let first = WebworkPilotSeedIds::for_tenant(tenant);
    let second = WebworkPilotSeedIds::for_tenant(tenant);
    let native = SeedIds::for_tenant(tenant);
    assert_eq!(first.assignment, second.assignment);
    assert_ne!(first.problem.as_uuid(), first.source_object.as_uuid());
    assert_ne!(first.problem.as_uuid(), native.problem.as_uuid());
    assert_ne!(first.assignment.as_uuid(), native.assignment.as_uuid());
}

#[test]
fn webwork_pilot_refuses_s3_endpoint_credentials_without_echoing_them() {
    let error = validate_s3_endpoint("http://public-value:private-value@127.0.0.1:9000")
        .expect_err("credential-bearing endpoint must refuse");
    assert!(error.to_string().contains("must not include credentials"));
    assert!(!error.to_string().contains("public-value"));
    assert!(!error.to_string().contains("private-value"));
}

#[tokio::test]
async fn injected_draft_create_conflict_rereads_and_accepts_only_exact_seed_content() {
    let store = learning_data_access::in_memory::MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(81));
    let actor = UserId::from_uuid(Uuid::from_u128(82));
    let context = TenantContext::from_authenticated_session(tenant);
    let draft = DraftRecord {
        tenant,
        question: webwork_pilot_draft(WorkspaceId::from_uuid(Uuid::from_u128(83))),
        revises: None,
        derived_from: None,
    };
    let raced = store
        .upsert_draft(context, actor, None, draft.clone())
        .await
        .expect("the competing seeder wrote the draft first");
    let resumed = reconcile_webwork_pilot_draft(Some(raced), &draft)
        .expect("typed conflict reread accepts the exact competing draft")
        .expect("competing draft remains available");
    assert_eq!(resumed.record, draft);
    assert!(
        reconcile_webwork_pilot_draft(None, &draft)
            .expect("a competing publisher may consume the draft before reread")
            .is_none()
    );
    let mut different = draft.clone();
    different.question.metadata.title = "different seeded content".to_string();
    assert!(reconcile_webwork_pilot_draft(Some(resumed), &different).is_err());
}

#[tokio::test]
async fn webwork_pilot_converges_after_every_persisted_prefix_and_on_rerun() {
    let store = learning_data_access::in_memory::MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(91));
    let instructor = UserId::from_uuid(Uuid::from_u128(92));
    let student = UserId::from_uuid(Uuid::from_u128(93));
    let context = TenantContext::from_authenticated_session(tenant);
    let ids = WebworkPilotSeedIds::for_tenant(tenant);
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let source_key = webwork_pilot_source_key(reference, ids.source_object);
    let source_record = objects::ObjectRecord {
        id: ids.source_object,
        bucket: objects::Bucket::Content,
        key: source_key,
        sha256: objects::Sha256Digest::compute(WEBWORK_PILOT_SOURCE),
        size_bytes: u64::try_from(WEBWORK_PILOT_SOURCE.len()).expect("fixture fits u64"),
        media_type: "text/x-wework-pg".to_string(),
        category: ObjectCategory::Source,
        version: Some(ids.version),
        license: "CC-BY-4.0".to_string(),
        provenance: WEBWORK_PILOT_SOURCE_PROVENANCE.to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
    };
    let draft = DraftRecord {
        tenant,
        question: webwork_pilot_draft(ids.workspace),
        revises: None,
        derived_from: None,
    };
    let capabilities = webwork_capabilities();
    ensure_webwork_pilot_publication(
        &store,
        context,
        instructor,
        draft.clone(),
        reference,
        source_record.clone(),
        capabilities.clone(),
    )
    .await
    .expect("publication prefix converges");

    let course = CourseRecord {
        id: ids.course,
        tenant,
        title: "PLE WebWork pilot E2E course".to_string(),
        members: vec![
            CourseMembership {
                user: instructor,
                role: CourseMembershipRole::Instructor,
            },
            CourseMembership {
                user: student,
                role: CourseMembershipRole::Student,
            },
        ],
    };
    ensure_webwork_pilot_course(&store, context, course.clone())
        .await
        .expect("course prefix converges");
    let assignment = AssignmentRecord {
        id: ids.assignment,
        tenant,
        course_id: ids.course,
        title: "PLE WebWork pilot E2E assignment".to_string(),
        items: vec![AssignmentItem {
            id: ids.assignment_item,
            reference,
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: Vec::new(),
        policies: RunPolicies {
            completion: CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        },
    };
    ensure_webwork_pilot_assignment(&store, context, assignment.clone())
        .await
        .expect("assignment prefix converges");
    let enrollment = AssignmentEnrollment {
        id: ids.enrollment,
        tenant,
        assignment: ids.assignment,
        user: student,
        student: StudentId::from_uuid(student.as_uuid()),
        first_completed_at: None,
        current_grade_run: None,
        best_grade_run: None,
    };
    ensure_webwork_pilot_enrollment(&store, context, enrollment.clone())
        .await
        .expect("enrollment prefix converges");

    ensure_webwork_pilot_publication(
        &store,
        context,
        instructor,
        draft,
        reference,
        source_record.clone(),
        capabilities,
    )
    .await
    .expect("published rerun verifies rather than republishes");
    ensure_webwork_pilot_course(&store, context, course)
        .await
        .expect("course rerun verifies rather than mutates");
    ensure_webwork_pilot_assignment(&store, context, assignment)
        .await
        .expect("assignment rerun verifies rather than mutates");
    ensure_webwork_pilot_enrollment(&store, context, enrollment)
        .await
        .expect("enrollment rerun verifies rather than mutates");
    let error = ensure_webwork_pilot_publication(
        &store,
        context,
        instructor,
        DraftRecord {
            tenant,
            question: webwork_pilot_draft(ids.workspace),
            revises: None,
            derived_from: None,
        },
        reference,
        source_record,
        BackendCapabilities::from_iter([Capability::ServerGrading]),
    )
    .await
    .expect_err("capability mismatch must refuse instead of mutating a published source");
    assert!(
        error
            .to_string()
            .contains("publication differs from the deterministic seed")
    );
}
