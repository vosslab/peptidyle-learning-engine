use super::*;
use question_model::Capability;

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
fn chapter_one_seed_keeps_the_exact_two_by_four_teaching_matrix() {
    let chapters = pilot_chapters().expect("the tracked pilot inventory is valid");
    assert_eq!(chapters.len(), 2);
    for chapter in chapters {
        assert_eq!(chapter.questions.len(), 4);
        assert!(matches!(
            chapter.questions[0].kind,
            PilotQuestionKind::WebworkMultipleChoice
        ));
        assert!(matches!(
            chapter.questions[1].kind,
            PilotQuestionKind::WebworkMatching
        ));
        assert!(matches!(
            chapter.questions[2].kind,
            PilotQuestionKind::FlatMultipleChoice
        ));
        assert!(matches!(
            chapter.questions[3].kind,
            PilotQuestionKind::FlatMatching
        ));
        for question in chapter.questions {
            assert!(!question.slug.is_empty());
            assert!(!question.title.is_empty());
            assert!(!question.source.is_empty());
        }
    }
}

#[tokio::test]
async fn chapter_one_seed_upserts_the_fake_learner_through_the_canonical_roster() {
    let store = learning_data_access::in_memory::MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(301));
    let instructor = UserId::from_uuid(Uuid::from_u128(302));
    let student = UserId::from_uuid(Uuid::from_u128(303));
    let course = CourseId::from_uuid(Uuid::from_u128(304));
    let assignment = AssignmentId::from_uuid(Uuid::from_u128(305));
    let context = TenantContext::from_authenticated_session(tenant);
    let question = pilot_chapters()
        .expect("the tracked pilot inventory is valid")
        .into_iter()
        .flat_map(|chapter| chapter.questions)
        .find(|question| matches!(question.kind, PilotQuestionKind::WebworkMultipleChoice))
        .expect("the Chapter 1 matrix includes a WeBWorK multiple-choice question");
    let question_ids = question_ids(tenant, &question.slug);
    let published = publish_webwork_question(
        &store,
        &objects::memory::MemoryObjectStore::default(),
        context,
        instructor,
        &question,
        &question_ids,
    )
    .await
    .expect("the fixture publishes an assignment item through the catalog contract");
    let expected_course = CourseRecord {
        id: course,
        tenant,
        title: "Disposable Chapter 1 Genetics".to_string(),
        members: vec![CourseMembership {
            user: instructor,
            role: CourseMembershipRole::Instructor,
        }],
    };
    ensure_webwork_pilot_course(&store, context, expected_course.clone())
        .await
        .expect("the seed creates an instructor-owned course before roster activation");
    store
        .create_untimed_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Disposable Chapter 1 assignment".to_string(),
                items: vec![AssignmentItem {
                    id: AssignmentItemId::from_uuid(Uuid::from_u128(306)),
                    reference: ProblemVersionRef {
                        problem: published.problem,
                        version: published.version,
                    },
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
            },
        )
        .await
        .expect("the seed creates the assignment before roster activation");

    let first = upsert_chapter_one_student(&store, context, student, course, assignment)
        .await
        .expect("the canonical roster derives the first enrollment");
    let second = upsert_chapter_one_student(&store, context, student, course, assignment)
        .await
        .expect("the canonical roster upsert is idempotent on rerun");
    ensure_webwork_pilot_course(&store, context, expected_course)
        .await
        .expect("the course seed accepts student membership owned by the canonical roster");
    assert_eq!(first, second);
    assert_eq!(first.user, student);
    let claimed = store
        .upsert_course_member(
            context,
            UpsertCourseMember {
                course,
                user: student,
                display_name: CHAPTER_ONE_FAKE_STUDENT_DISPLAY_NAME.to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("the canonical roster returns the previously created learner");
    let member = &claimed.member;
    assert_eq!(member.display_name, CHAPTER_ONE_FAKE_STUDENT_DISPLAY_NAME);
    assert_eq!(
        member.status,
        learning_data_access::CourseMemberStatus::Active
    );
    assert_eq!(member.roster_email, None);
    assert_eq!(member.roster_id, None);
    assert_eq!(
        store
            .get_course(context, course)
            .await
            .expect("course read succeeds")
            .expect("course remains")
            .role_for(student),
        Some(question_model::CourseRole::Student)
    );
}

#[test]
fn course_seed_identity_allows_only_roster_owned_additional_students() {
    let tenant = TenantId::from_uuid(Uuid::from_u128(311));
    let instructor = UserId::from_uuid(Uuid::from_u128(312));
    let expected = CourseRecord {
        id: CourseId::from_uuid(Uuid::from_u128(313)),
        tenant,
        title: "Genetics Chapter 1".to_string(),
        members: vec![CourseMembership {
            user: instructor,
            role: CourseMembershipRole::Instructor,
        }],
    };
    let mut roster_extended = expected.clone();
    roster_extended.members.push(CourseMembership {
        user: UserId::from_uuid(Uuid::from_u128(314)),
        role: CourseMembershipRole::Student,
    });
    assert!(webwork_pilot_course_seed_matches(
        &roster_extended,
        &expected
    ));

    let mut unexpected_instructor = roster_extended;
    unexpected_instructor.members.push(CourseMembership {
        user: UserId::from_uuid(Uuid::from_u128(315)),
        role: CourseMembershipRole::Instructor,
    });
    assert!(!webwork_pilot_course_seed_matches(
        &unexpected_instructor,
        &expected
    ));
}

#[test]
fn chapter_one_seed_sources_compile_and_use_evidence_bounded_capabilities() {
    for chapter in pilot_chapters().expect("the tracked pilot inventory is valid") {
        for question in chapter.questions {
            match question.kind {
                PilotQuestionKind::FlatMultipleChoice | PilotQuestionKind::FlatMatching => {
                    let (draft, _) =
                        adapter_native::flat_question::FlatQuestionDocument::parse(question.source)
                            .expect("tracked flat pilot source parses")
                            .compile(WorkspaceId::from_uuid(Uuid::from_u128(77)))
                            .expect("tracked flat pilot source compiles")
                            .into_parts();
                    assert_eq!(
                        matches!(draft.response, ResponseDefinition::Matching { .. }),
                        matches!(question.kind, PilotQuestionKind::FlatMatching),
                        "the flat source family must match its assigned teaching-matrix slot"
                    );
                }
                PilotQuestionKind::WebworkMultipleChoice | PilotQuestionKind::WebworkMatching => {
                    let source = QuestionSource::Webwork {
                        pg_path: question.source_path.to_string(),
                    };
                    let capabilities =
                        adapter_webwork::reviewed_webwork_source_capabilities_for_feedback(
                            &source,
                            &objects::Sha256Digest::compute(question.source).to_string(),
                            FeedbackDisclosure::ImmediateCorrectness,
                        )
                        .expect("tracked PGML pilot source is registered");
                    assert!(capabilities.supports(Capability::AlgorithmicGeneration));
                    assert!(capabilities.supports(Capability::ServerGrading));
                    assert!(capabilities.supports(Capability::Hints));
                    assert_eq!(
                        capabilities.supports(Capability::PartialCredit),
                        matches!(question.kind, PilotQuestionKind::WebworkMatching)
                    );
                    let draft = webwork_draft(
                        WorkspaceId::from_uuid(Uuid::from_u128(78)),
                        &question,
                        FeedbackDisclosure::ImmediateCorrectness,
                    );
                    assert!(
                        domain::policy::validate_draft_for_publication(&draft, &capabilities)
                            .is_empty()
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn chapter_one_webwork_publishers_converge_on_one_current_immutable_version() {
    let tenant = TenantId::from_uuid(Uuid::from_u128(91));
    let context = TenantContext::from_authenticated_session(tenant);
    let publisher = UserId::from_uuid(Uuid::from_u128(92));
    let question = pilot_chapters()
        .expect("the tracked pilot inventory is valid")
        .into_iter()
        .flat_map(|chapter| chapter.questions)
        .find(|question| {
            matches!(
                question.kind,
                PilotQuestionKind::WebworkMultipleChoice | PilotQuestionKind::WebworkMatching
            )
        })
        .expect("the teaching matrix includes a WebWork question");
    let ids = question_ids(tenant, &question.slug);
    let store = learning_data_access::in_memory::MemoryStore::default();
    let objects = objects::memory::MemoryObjectStore::default();

    let (first, second) = tokio::join!(
        publish_webwork_question(&store, &objects, context, publisher, &question, &ids),
        publish_webwork_question(&store, &objects, context, publisher, &question, &ids),
    );
    let first = first.expect("first concurrent publisher converges");
    let second = second.expect("second concurrent publisher converges");
    assert_eq!(first.problem, ids.problem);
    assert_eq!(first.version, ids.version);
    assert_eq!(first, second);

    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let published = store
        .get_catalog_problem(context, reference)
        .await
        .expect("published read succeeds")
        .expect("current version is published");
    let artifact = store
        .catalog_source_artifact(context, reference)
        .await
        .expect("source artifact read succeeds")
        .expect("source artifact is bound");
    let stored = objects
        .get(&artifact.object.key)
        .await
        .expect("source bytes are stored");

    assert_eq!(published.version_number.value(), 1);
    assert_eq!(published.previous_version, None);
    assert!(published.capabilities.supports(Capability::Hints));
    assert_eq!(
        published.question.attempt_policy.feedback,
        FeedbackDisclosure::ImmediateCorrectness
    );
    assert_eq!(stored.bytes, question.source);
    assert_eq!(artifact.object.version, Some(published.version));

    let projected = serde_json::to_string(&published).expect("public record serializes");
    let source_text = std::str::from_utf8(question.source).expect("tracked PGML is UTF-8");
    assert!(!projected.contains(source_text));
    assert!(!projected.contains("correctResponse"));

    let mut versions = store
        .list_catalog(
            context,
            PageRequest::first(PageSize::new(10).expect("page size is valid")),
        )
        .await
        .expect("catalog listing succeeds")
        .items
        .into_iter()
        .filter(|item| item.public_id == published.public_id)
        .map(|item| item.version_number.value())
        .collect::<Vec<_>>();
    versions.sort_unstable();
    assert_eq!(
        versions,
        vec![1],
        "concurrent publication does not create a synthetic successor"
    );

    let rerun = publish_webwork_question(&store, &objects, context, publisher, &question, &ids)
        .await
        .expect("rerun converges");
    assert_eq!(rerun, published);
}

#[test]
fn chapter_one_seed_storage_flag_is_explicit_and_mutually_exclusive() {
    let common = [
        "--database-url",
        "postgres://example",
        "--tenant",
        "00000000-0000-0000-0000-000000000001",
        "--instructor",
        "00000000-0000-0000-0000-000000000002",
        "--student",
        "00000000-0000-0000-0000-000000000003",
        "--apply-migrations",
        "--chapter-one-pilot",
        "--s3-endpoint",
        "http://127.0.0.1:9000",
        "--s3-region",
        "us-east-1",
        "--content-bucket",
        "content",
    ]
    .map(str::to_string);
    let parsed = parse_arguments(&common).expect("complete Chapter 1 storage settings parse");
    assert!(parsed.chapter_one_pilot.is_some());
    assert!(parsed.webwork_pilot.is_none());

    let mut conflicting = common.to_vec();
    conflicting.push("--webwork-pilot".to_string());
    assert!(parse_arguments(&conflicting).is_err());
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
