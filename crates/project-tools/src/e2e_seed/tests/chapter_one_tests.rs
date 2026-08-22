use super::*;
use question_model::Capability;

fn resume_manifest(tenant: TenantId) -> ChapterOnePilotManifest {
    let mut question_identity = 1_u128;
    ChapterOnePilotManifest {
        chapters: pilot_chapters()
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(chapter_index, chapter)| ChapterManifest {
                course_id: CourseId::from_uuid(pilot_uuid(tenant, &chapter.slug, "course")),
                assignment_id: AssignmentId::from_uuid(pilot_uuid(
                    tenant,
                    &chapter.slug,
                    "assignment",
                )),
                enrollment_id: EnrollmentId::from_uuid(Uuid::from_u128(
                    900 + chapter_index as u128,
                )),
                slug: chapter.slug,
                questions: chapter
                    .questions
                    .into_iter()
                    .map(|question| {
                        let identity = question_identity;
                        question_identity += 1;
                        QuestionManifest {
                            slug: question.slug,
                            display_id: format!("CAT-{identity:04}"),
                            problem_id: ProblemId::from_uuid(Uuid::from_u128(100 + identity)),
                            version_id: VersionId::from_uuid(Uuid::from_u128(200 + identity)),
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn synthetic_resume_specs() -> Vec<PilotChapterSpec> {
    vec![
        PilotChapterSpec {
            slug: "first".to_string(),
            course_title: "First".to_string(),
            assignment_title: "First assignment".to_string(),
            questions: vec![PilotQuestionSpec {
                slug: "first-question".to_string(),
                title: "First question".to_string(),
                source_path: "synthetic".to_string(),
                source: b"synthetic",
                kind: PilotQuestionKind::FlatMultipleChoice,
                points: 1,
            }],
        },
        PilotChapterSpec {
            slug: "second".to_string(),
            course_title: "Second".to_string(),
            assignment_title: "Second assignment".to_string(),
            questions: vec![PilotQuestionSpec {
                slug: "second-question".to_string(),
                title: "Second question".to_string(),
                source_path: "synthetic".to_string(),
                source: b"synthetic",
                kind: PilotQuestionKind::FlatMultipleChoice,
                points: 1,
            }],
        },
    ]
}

fn synthetic_resume_manifest(
    tenant: TenantId,
    tracked: &[PilotChapterSpec],
) -> ChapterOnePilotManifest {
    let mut identity = 1_u128;
    ChapterOnePilotManifest {
        chapters: tracked
            .iter()
            .enumerate()
            .map(|(chapter_index, chapter)| ChapterManifest {
                slug: chapter.slug.clone(),
                course_id: CourseId::from_uuid(pilot_uuid(tenant, &chapter.slug, "course")),
                assignment_id: AssignmentId::from_uuid(pilot_uuid(
                    tenant,
                    &chapter.slug,
                    "assignment",
                )),
                enrollment_id: EnrollmentId::from_uuid(Uuid::from_u128(
                    900 + chapter_index as u128,
                )),
                questions: chapter
                    .questions
                    .iter()
                    .map(|question| {
                        let current = identity;
                        identity += 1;
                        QuestionManifest {
                            slug: question.slug.clone(),
                            display_id: format!("CAT-{current:04}"),
                            problem_id: ProblemId::from_uuid(Uuid::from_u128(100 + current)),
                            version_id: VersionId::from_uuid(Uuid::from_u128(200 + current)),
                        }
                    })
                    .collect(),
            })
            .collect(),
    }
}

#[test]
fn resume_manifest_validation_rejects_identity_drift() {
    let tenant = TenantId::from_uuid(Uuid::from_u128(88));
    let tracked = pilot_chapters().unwrap();
    let valid = resume_manifest(tenant);
    assert!(validate_resume_manifest(&valid, tenant, &tracked).is_ok());
    let tracked = synthetic_resume_specs();
    let valid = synthetic_resume_manifest(tenant, &tracked);
    assert!(validate_resume_manifest(&valid, tenant, &tracked).is_ok());

    let mut wrong = valid.clone();
    wrong
        .chapters
        .first_mut()
        .expect("synthetic manifest has a chapter")
        .slug = "unexpected-order".to_string();
    assert!(validate_resume_manifest(&wrong, tenant, &tracked).is_err());

    let mut wrong = valid.clone();
    wrong
        .chapters
        .first_mut()
        .expect("synthetic manifest has a chapter")
        .course_id = CourseId::from_uuid(Uuid::from_u128(999));
    assert!(validate_resume_manifest(&wrong, tenant, &tracked).is_err());

    let mut wrong = valid.clone();
    wrong
        .chapters
        .first_mut()
        .expect("synthetic manifest has a chapter")
        .assignment_id = AssignmentId::from_uuid(Uuid::from_u128(999));
    assert!(validate_resume_manifest(&wrong, tenant, &tracked).is_err());

    let mut duplicate = valid.clone();
    let enrollment = duplicate
        .chapters
        .first()
        .expect("synthetic manifest has a chapter")
        .enrollment_id;
    let another_chapter = duplicate
        .chapters
        .get_mut(1)
        .expect("synthetic manifest has another chapter");
    another_chapter.enrollment_id = enrollment;
    assert!(validate_resume_manifest(&duplicate, tenant, &tracked).is_err());

    let mut duplicate = valid.clone();
    let display_id = duplicate
        .chapters
        .iter()
        .flat_map(|chapter| &chapter.questions)
        .next()
        .expect("synthetic manifest has a question")
        .display_id
        .clone();
    duplicate
        .chapters
        .iter_mut()
        .flat_map(|chapter| &mut chapter.questions)
        .nth(1)
        .expect("synthetic manifest has another question")
        .display_id = display_id;
    assert!(validate_resume_manifest(&duplicate, tenant, &tracked).is_err());

    let mut duplicate = valid.clone();
    let (problem_id, version_id) = duplicate
        .chapters
        .iter()
        .flat_map(|chapter| &chapter.questions)
        .next()
        .map(|question| (question.problem_id, question.version_id))
        .expect("synthetic manifest has a question");
    let target = duplicate
        .chapters
        .iter_mut()
        .flat_map(|chapter| &mut chapter.questions)
        .nth(1)
        .expect("synthetic manifest has another question");
    target.problem_id = problem_id;
    target.version_id = version_id;
    assert!(validate_resume_manifest(&duplicate, tenant, &tracked).is_err());

    let mut noncanonical = valid.clone();
    noncanonical
        .chapters
        .iter_mut()
        .flat_map(|chapter| &mut chapter.questions)
        .next()
        .expect("synthetic manifest has a question")
        .display_id = "cat-0001".to_string();
    assert!(validate_resume_manifest(&noncanonical, tenant, &tracked).is_err());

    let unknown =
        serde_json::from_str::<ChapterOnePilotManifest>(r#"{"chapters":[],"extra":true}"#);
    assert!(unknown.is_err());
}

#[test]
fn chapter_one_course_markers_select_fresh_or_protected_resume() {
    let fresh = chapter_one_corpus_state([false, false]).expect("empty markers select fresh");
    assert_eq!(fresh, ChapterOneCorpusState::Fresh);
    assert_eq!(
        chapter_one_resume_manifest_path(fresh, Some("obsolete-host-manifest.json"))
            .expect("fresh publication ignores a host manifest candidate"),
        None
    );

    let published = chapter_one_corpus_state([true, true]).expect("full markers select resume");
    assert_eq!(published, ChapterOneCorpusState::Published);
    assert!(chapter_one_resume_manifest_path(published, None).is_err());
    assert_eq!(
        chapter_one_resume_manifest_path(published, Some("resume-manifest.json"))
            .expect("published markers require their manifest"),
        Some("resume-manifest.json")
    );

    assert!(chapter_one_corpus_state([true, false]).is_err());
}

#[test]
fn outer_seed_marker_decision_stops_interrupted_publication_before_a_fresh_retry() {
    assert_eq!(
        seed_replay_state(false, false, "test seed").expect("empty scaffold starts fresh"),
        SeedReplayState::Fresh
    );
    assert_eq!(
        seed_replay_state(true, true, "test seed").expect("complete scaffold replays"),
        SeedReplayState::Replay
    );
    assert!(seed_replay_state(true, false, "test seed").is_err());
    assert!(seed_replay_state(false, true, "test seed").is_err());
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
    let question_ids = QuestionIds::generate();
    let published = publish_webwork_question(
        &store,
        &objects::memory::MemoryObjectStore::default(),
        context,
        instructor,
        &question,
        &question_ids,
        None,
    )
    .await
    .expect("the fixture publishes an assignment item through the catalog contract");
    let expected_course = CourseRecord {
        id: course,
        tenant,
        title: "Disposable Chapter 1 Genetics".to_string(),
        term: question_model::CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
            .expect("explicit fixture course term"),
    };
    ensure_webwork_pilot_course(&store, context, instructor, expected_course.clone())
        .await
        .expect("the seed creates an instructor-owned course before roster activation");
    ensure_webwork_pilot_assignment(
        &store,
        context,
        instructor,
        AssignmentRecord {
            id: assignment,
            tenant,
            course_id: course,
            title: "Disposable Chapter 1 assignment".to_string(),
            lifecycle: question_model::AssignmentLifecycle::Published,
            instructions: question_model::AssignmentInstructions::try_new(
                "Use the Chapter 1 evidence to explain each answer.".to_string(),
            )
            .expect("fixture instructions are valid"),
            audience: question_model::AssignmentAudience::CourseWide,
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
            disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
            policies: RunPolicies {
                completion: CompletionRequirement::AnswerAll,
                grade: GradePolicy::Highest,
                continued_practice: ContinuedPractice::Unlimited,
                variation: VariationPolicy::NewSeeds,
            },
        },
    )
    .await
    .expect("the seed creates Draft then publishes before roster activation");

    let first =
        upsert_chapter_one_student(&store, context, instructor, student, course, assignment)
            .await
            .expect("the entitlement seam materializes the first enrollment");
    let second =
        upsert_chapter_one_student(&store, context, instructor, student, course, assignment)
            .await
            .expect("the entitlement seam is idempotent on rerun");
    ensure_webwork_pilot_course(&store, context, instructor, expected_course)
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
            .get_current_course_membership(context, course, student)
            .await
            .expect("membership read succeeds")
            .expect("student membership remains")
            .role,
        question_model::CourseMembershipRole::Student
    );
}

#[test]
fn course_seed_identity_allows_only_roster_owned_additional_students() {
    let tenant = TenantId::from_uuid(Uuid::from_u128(311));
    let expected = CourseRecord {
        id: CourseId::from_uuid(Uuid::from_u128(313)),
        tenant,
        title: "Genetics Chapter 1".to_string(),
        term: question_model::CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
            .expect("explicit fixture course term"),
    };
    let different_title = CourseRecord {
        title: "Different course".to_string(),
        ..expected.clone()
    };
    assert!(webwork_pilot_course_seed_matches(&expected, &expected));
    assert!(!webwork_pilot_course_seed_matches(
        &different_title,
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
                        adapter_webwork::reviewed_webwork_source_profile_capabilities(
                            &source,
                            &objects::Sha256Digest::compute(question.source).to_string(),
                        )
                        .expect("tracked PGML pilot source is registered");
                    assert!(capabilities.supports(Capability::AlgorithmicGeneration));
                    assert!(capabilities.supports(Capability::ServerGrading));
                    assert!(capabilities.supports(Capability::Hints));
                    assert_eq!(
                        capabilities.supports(Capability::PartialCredit),
                        matches!(question.kind, PilotQuestionKind::WebworkMatching)
                    );
                    let draft =
                        webwork_draft(WorkspaceId::from_uuid(Uuid::from_u128(78)), &question);
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
async fn chapter_one_webwork_publishers_retain_one_exact_immutable_publication() {
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
    let ids = QuestionIds::generate();
    let store = learning_data_access::in_memory::MemoryStore::default();
    let objects = objects::memory::MemoryObjectStore::default();

    let (first, second) = tokio::join!(
        publish_webwork_question(&store, &objects, context, publisher, &question, &ids, None),
        publish_webwork_question(&store, &objects, context, publisher, &question, &ids, None),
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

    assert!(published.capabilities.supports(Capability::Hints));
    assert_eq!(stored.bytes, question.source);
    assert_eq!(artifact.object.version, Some(published.version));

    let projected = serde_json::to_string(&published).expect("public record serializes");
    let source_text = std::str::from_utf8(question.source).expect("tracked PGML is UTF-8");
    assert!(!projected.contains(source_text));
    assert!(!projected.contains("correctResponse"));

    assert_eq!(published.question_id, first.question_id);

    let host_manifest = Manifest {
        course_id: CourseId::from_uuid(Uuid::from_u128(700)),
        assignment_id: AssignmentId::from_uuid(Uuid::from_u128(701)),
        enrollment_id: EnrollmentId::from_uuid(Uuid::from_u128(702)),
        question_id: first.question_id.clone(),
        problem_id: first.problem,
        version_id: first.version,
    };
    let encoded = serde_json::to_value(host_manifest).expect("host manifest serializes");
    assert_eq!(encoded["courseId"], Uuid::from_u128(700).to_string());
    assert_eq!(encoded["questionId"], first.question_id.to_string());

    let rerun = publish_webwork_question(
        &store,
        &objects,
        context,
        publisher,
        &question,
        &QuestionIds::from_published(&published),
        Some(&published),
    )
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
        "--private-content-bucket",
        "private-content",
    ]
    .map(str::to_string);
    let parsed = parse_arguments(&common).expect("complete Chapter 1 storage settings parse");
    assert!(parsed.chapter_one_pilot.is_some());
    assert!(parsed.webwork_pilot.is_none());

    let mut conflicting = common.to_vec();
    conflicting.push("--webwork-pilot".to_string());
    assert!(parse_arguments(&conflicting).is_err());
}
