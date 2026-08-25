use super::*;
use std::collections::BTreeMap;

use crate::{
    CourseMemberStatus, SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store,
};
use question_model::answer::NumericTolerance;
use question_model::envelope::ContentBlock;
use question_model::generation::{GeneratorReference, ParameterSpec, RandomizationDefinition};
use question_model::run_policy::AttemptPolicy;
use question_model::taxonomy::{License, Tag};
use question_model::{
    AttemptProvenance, AttemptResult, AttemptTimerRecord, BackendCapabilities, Capability,
    CourseMembershipId, CourseMembershipRole, DraftQuestionDefinition, DraftQuestionSource,
    GradingDefinition, ImplementationVersion, QuestionDefinition, QuestionMetadata,
    ResponseDefinition,
};

pub(super) fn seed_catalog(
    store: &MemoryStore,
    records: impl IntoIterator<Item = PublishedProblemRecord>,
) {
    let mut state = store.write_state().expect("catalog fixture state");
    for record in records {
        let sequence = state.next_catalog_publication_sequence;
        state.next_catalog_publication_sequence += 1;
        state
            .catalog_publication_sequences
            .insert((record.problem, record.version), sequence);
        state
            .published
            .insert((record.problem, record.version), record);
    }
}

pub(super) fn record(number: u128) -> PublishedProblemRecord {
    let problem = ProblemId::from_uuid(Uuid::from_u128(number));
    let version = VersionId::from_uuid(Uuid::from_u128(20_000 + number));
    let question = QuestionDefinition::from_draft(
        DraftQuestionDefinition {
            workspace: WorkspaceId::from_uuid(Uuid::from_u128(30_000 + number)),
            source: DraftQuestionSource::Native {
                family: "catalog_fixture".to_string(),
            },
            prompt: Vec::new(),
            response: ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Absolute { epsilon: 0.1 },
                unit: None,
            },
            attempt_policy: AttemptPolicy { max_attempts: None },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: format!("Peptide catalog item {number}"),
                tags: vec![Tag::new("peptide")],
                taxonomy: vec![TaxonomyTerm {
                    scheme: "discipline".to_string(),
                    code: "biochemistry".to_string(),
                    label: "Biochemistry".to_string(),
                }],
                license: License::CcBy,
                language: "en".to_string(),
            },
        },
        problem,
        version,
        question_model::QuestionSource::Native {
            family: "catalog_fixture".to_string(),
        },
    );
    PublishedProblemRecord {
        problem,
        question_id: {
            let mut value = u32::try_from(number).expect("fixture Question ID fits 30 bits");
            let mut bytes = [b'0'; 6];
            for output in bytes.iter_mut().rev() {
                *output = question_model::QUESTION_ID_ALPHABET[(value & 0x1f) as usize];
                value >>= 5;
            }
            crate::QuestionIdCodec::from_server_secret([0x42; 32])
                .issue_for_identifier(std::str::from_utf8(&bytes).expect("alphabet is ASCII"))
                .expect("fixture Question ID issues")
        },
        version,
        question,
        capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
        scope: PublicationScope::Public,
        lifecycle: CatalogLifecycle::Published,
        author_ids: vec![UserId::from_uuid(Uuid::from_u128(40_000))],
        byline: question_model::PublicByline::new(vec![
            question_model::PublicAuthorName::new("Catalog test author".to_string())
                .expect("valid test byline"),
        ])
        .expect("valid test byline"),
        derived_from: None,
        published_at: ActivityTimestamp::from_unix_millis(0),
    }
}

fn seeded_record(number: u128) -> PublishedProblemRecord {
    let mut published = record(number);
    published.question.prompt = vec![ContentBlock::Text {
        markdown: "A {{residue}} example.".to_string(),
    }];
    published.question.randomization = RandomizationDefinition::Seeded {
        generator: GeneratorReference {
            id: "catalog-projection-fixture".to_string(),
            version: "1".to_string(),
        },
        parameters: BTreeMap::from([(
            "residue".to_string(),
            ParameterSpec::Choice {
                options: vec!["glycine".to_string()],
            },
        )]),
    };
    published
}

pub(super) fn statistics_attempt(
    number: u128,
    tenant: TenantId,
    run: RunId,
    reference: ProblemVersionRef,
    position: u32,
    issued_at: i64,
) -> QuestionAttempt {
    QuestionAttempt {
        id: QuestionAttemptId::from_uuid(Uuid::from_u128(number)),
        tenant,
        run,
        problem: reference.problem,
        question_version: reference.version,
        assignment_position: position,
        seed: number as u64,
        parameter_hash: format!("statistics-parameters-{number}"),
        response: None,
        status: AttemptStatus::InProgress,
        result: None,
        timer: AttemptTimerRecord {
            issued_at: ActivityTimestamp::from_unix_millis(issued_at),
            deadline: None,
            submitted_at: None,
        },
        provenance: AttemptProvenance {
            adapter: ImplementationVersion {
                id: "native".to_string(),
                version: "1".to_string(),
            },
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: ImplementationVersion {
                id: "numeric".to_string(),
                version: "1".to_string(),
            },
            rendered_question_sha256: format!("statistics-render-{number}"),
        },
        issued_capability: question_model::IssuedAttemptCapabilityV1::NotApplicable,
    }
}

pub(super) struct StatisticsSubmissionFixture {
    pub(super) context: TenantContext,
    pub(super) actor: UserId,
    pub(super) binding: LearnerWorkRoutingBinding,
    pub(super) attempt: QuestionAttempt,
}

pub(super) fn submit_statistics_attempt(
    store: &MemoryStore,
    fixture: StatisticsSubmissionFixture,
    submitted_at: i64,
    earned: f64,
    possible: f64,
) -> (SubmissionRecord, SubmitQuestionAttemptCommand) {
    let StatisticsSubmissionFixture {
        context,
        actor,
        binding,
        attempt,
    } = fixture;
    let command = SubmitQuestionAttemptCommand {
        actor,
        binding,
        attempt: attempt.id,
        response: StudentResponse::Numeric { value: earned },
        result: AttemptResult {
            correct: earned == possible,
            points_earned: earned,
            points_possible: possible,
        },
        feedback: question_model::FeedbackContent::default(),
        idempotency_key: SubmissionIdempotencyKey::parse(format!(
            "statistics-submission-{}",
            attempt.id
        ))
        .expect("valid fixture idempotency key"),
    };
    let mut state = store.write_state().expect("statistics fixture state");
    state.authoritative_time = ActivityTimestamp::from_unix_millis(submitted_at);
    insert_statistics_issued_authority(&mut state, &attempt);
    state.attempts.insert((attempt.tenant, attempt.id), attempt);
    let record = submit_question_attempt_locked(&mut state, context, command.clone())
        .expect("statistics fixture submission");
    (record, command)
}

/// Statistics fixtures install only the authority a real issue transaction
/// persists. Submission must therefore succeed even after the catalog record
/// is unavailable; it has no permission to recover timing from current policy.
pub(super) fn insert_statistics_issued_authority(state: &mut State, attempt: &QuestionAttempt) {
    let run = state
        .runs
        .get(&(attempt.tenant, attempt.run))
        .expect("statistics attempt has a run");
    let enrollment = state
        .enrollments
        .get(&(attempt.tenant, run.enrollment))
        .expect("statistics run has an enrollment");
    state.attempt_presentation_capabilities.insert(
        (attempt.tenant, attempt.id),
        crate::PresentationCapability::NotApplicable,
    );
    state.attempt_flat_grading_capabilities.insert(
        (attempt.tenant, attempt.id),
        crate::FlatGradingCapability::NotApplicable,
    );
    state.attempt_webwork_grading_capabilities.insert(
        (attempt.tenant, attempt.id),
        crate::WebworkGradingCapability::NotApplicable,
    );
    state.attempt_qti_grading_capabilities.insert(
        (attempt.tenant, attempt.id),
        crate::QtiGradingCapability::NotApplicable,
    );
    state.attempt_timing.insert(
        (attempt.tenant, attempt.id),
        MemoryAttemptTiming {
            assignment: enrollment.assignment,
            authored_deadline: None,
            authored_grace_seconds: 0,
            effective_deadline: None,
            effective_grace_seconds: 0,
            auto_submit_at: None,
            generation: 1,
            job: None,
        },
    );
    super::course_policy::store_issued_effective_policy_receipt(
        state,
        attempt.tenant,
        attempt.id,
        domain::effective_assignment_policy::EffectiveAssignmentPolicy {
            available_at: domain::effective_assignment_policy::ResolvedField {
                value: None,
                source: domain::effective_assignment_policy::PolicySource::Base,
            },
            due_at: domain::effective_assignment_policy::ResolvedField {
                value: None,
                source: domain::effective_assignment_policy::PolicySource::Base,
            },
            closes_at: domain::effective_assignment_policy::ResolvedField {
                value: None,
                source: domain::effective_assignment_policy::PolicySource::Base,
            },
            time_limit_seconds: domain::effective_assignment_policy::ResolvedField {
                value: None,
                source: domain::effective_assignment_policy::PolicySource::Base,
            },
            attempt_limit: domain::effective_assignment_policy::ResolvedField {
                value: None,
                source: domain::effective_assignment_policy::PolicySource::Base,
            },
            late_submission: domain::effective_assignment_policy::ResolvedField {
                value: question_model::LateSubmissionPolicy::Accept,
                source: domain::effective_assignment_policy::PolicySource::Base,
            },
            deadline_behavior: domain::effective_assignment_policy::ResolvedField {
                value: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                source: domain::effective_assignment_policy::PolicySource::Base,
            },
        },
    )
    .expect("statistics fixture effective-policy receipt");
}

#[tokio::test]
async fn statistics_receipts_are_exactly_once_and_disclose_only_at_k_five() {
    let store = MemoryStore::default();
    let mut record = record(71_000);
    record.scope = PublicationScope::Institution;
    let reference = ProblemVersionRef {
        problem: record.problem,
        version: record.version,
    };
    let tenant = TenantId::from_uuid(Uuid::from_u128(71_001));
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .write_state()
        .expect("test state")
        .published
        .insert((reference.problem, reference.version), record);
    store
        .write_state()
        .expect("test state")
        .catalog_grants
        .insert((tenant, reference.problem, reference.version));

    let first = CollapsedQuestionObservation::new(0.5, 2, 30, Some(0.4))
        .expect("valid collapsed observation");
    assert!(
        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
                RunId::from_uuid(Uuid::from_u128(71_020)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                reference,
                first,
            )
            .expect("first receipt records")
    );
    assert!(
        !store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
                RunId::from_uuid(Uuid::from_u128(71_020)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                reference,
                first,
            )
            .expect("exact replay is harmless")
    );
    let before_conflict = store.read_state().expect("test state").question_statistics
        [&(reference.problem, reference.version)]
        .snapshot();
    assert_eq!(
        store.record_question_statistics_contribution(
            tenant,
            EnrollmentId::from_uuid(Uuid::from_u128(71_010)),
            RunId::from_uuid(Uuid::from_u128(71_020)),
            QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
            reference,
            CollapsedQuestionObservation::new(0.6, 2, 30, Some(0.4))
                .expect("valid conflicting observation"),
        ),
        Err(StoreError::Conflict)
    );
    assert_eq!(
        store.read_state().expect("test state").question_statistics
            [&(reference.problem, reference.version)]
            .snapshot(),
        before_conflict
    );
    for number in 1..4_u128 {
        assert!(
            store
                .record_question_statistics_contribution(
                    tenant,
                    EnrollmentId::from_uuid(Uuid::from_u128(71_010 + number)),
                    RunId::from_uuid(Uuid::from_u128(71_020 + number)),
                    QuestionAttemptId::from_uuid(Uuid::from_u128(71_030 + number)),
                    reference,
                    CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.4))
                        .expect("valid contribution"),
                )
                .expect("distinct receipt records")
        );
    }
    assert_eq!(
        store
            .question_statistics(context, reference)
            .await
            .expect("safe statistics read at four"),
        QuestionStatisticsDisclosure::Suppressed
    );
    assert!(
        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_014)),
                RunId::from_uuid(Uuid::from_u128(71_024)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_034)),
                reference,
                CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.4))
                    .expect("valid fifth contribution"),
            )
            .expect("fifth receipt records")
    );
    let disclosure = store
        .question_statistics(context, reference)
        .await
        .expect("safe statistics read");
    assert!(matches!(
        disclosure,
        QuestionStatisticsDisclosure::Available(view) if view.cohort_size == 5
    ));
    {
        let state = store.read_state().expect("test state");
        assert_eq!(state.question_statistics_receipts.len(), 5);
        assert_eq!(
            state.question_statistics[&(reference.problem, reference.version)].cohort_size(),
            5
        );
    }
    let second_reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(Uuid::from_u128(71_100)),
        version: VersionId::from_uuid(Uuid::from_u128(71_101)),
    };
    assert!(
        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(71_100)),
                RunId::from_uuid(Uuid::from_u128(71_020)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(71_030)),
                second_reference,
                first,
            )
            .expect("one completion trigger can contribute another version")
    );
    let foreign_context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(71_999)));
    assert_eq!(
        store
            .question_statistics(foreign_context, reference)
            .await
            .expect("foreign safe statistics read"),
        QuestionStatisticsDisclosure::Suppressed
    );
}

#[tokio::test]
async fn catalog_search_finds_an_exact_question_id_beyond_the_first_page() {
    let store = MemoryStore::default();
    let context =
        TenantContext::from_authenticated_session(TenantId::from_uuid(Uuid::from_u128(73_000)));
    let mut decoy = record(70);
    let exact = record(71);
    let exact_reference = exact.question_id.to_string();
    {
        let mut state = store.write_state().expect("catalog search state");
        decoy.question.metadata.title = exact_reference.clone();
        state
            .published
            .insert((decoy.problem, decoy.version), decoy);
        state
            .published
            .insert((exact.problem, exact.version), exact);
    }

    let page = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                text: Some(exact_reference.clone()),
                page_size: Some(1),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("exact display reference search");

    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].summary.question_id.to_string(),
        exact_reference
    );
    assert_eq!(page.next_cursor, None);
}

#[tokio::test]
async fn catalog_search_authorship_uses_the_authenticated_actor_and_includes_coauthors() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(73_090));
    let context = TenantContext::from_authenticated_session(tenant);
    let mut authored = record(73_091);
    authored.author_ids = vec![
        UserId::from_uuid(Uuid::from_u128(73_092)),
        UserId::from_uuid(tenant.as_uuid()),
    ];
    let mut foreign = record(73_093);
    foreign.author_ids = vec![UserId::from_uuid(Uuid::from_u128(73_094))];
    let mut coauthored = record(73_095);
    coauthored.author_ids = vec![
        UserId::from_uuid(Uuid::from_u128(73_096)),
        UserId::from_uuid(tenant.as_uuid()),
    ];
    seed_catalog(&store, [authored.clone(), coauthored, foreign.clone()]);

    let page = store
        .search_catalog_as_instructor(
            context,
            CatalogSearchQuery {
                authorship: question_model::CatalogAuthorship::AuthoredByCurrentActor,
                page_size: Some(1),
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("authenticated authored scope search");

    assert_eq!(page.items.len(), 1);
    assert!(
        page.items
            .iter()
            .all(|item| item.summary.question_id != foreign.question_id)
    );
    let cursor = page.next_cursor.expect("authored page has a continuation");
    assert!(matches!(
        store
            .search_catalog_as_instructor(
                context,
                CatalogSearchQuery {
                    cursor: Some(cursor),
                    ..CatalogSearchQuery::default()
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn catalog_search_applies_metadata_filters_facets_and_actor_course_usage() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(73_100));
    let actor = UserId::from_uuid(Uuid::from_u128(73_101));
    let context = TenantContext::from_authenticated_session(tenant);
    let session = SessionTokenHash::compute(b"catalog-metadata-parity");
    let publication = record(73_102);
    let mut later_publication = record(73_108);
    later_publication.question.metadata.tags = vec![Tag::new("other")];
    let reference = ProblemVersionRef {
        problem: publication.problem,
        version: publication.version,
    };
    let later_reference = ProblemVersionRef {
        problem: later_publication.problem,
        version: later_publication.version,
    };
    seed_catalog(&store, [publication.clone(), later_publication]);
    store
        .create_session(
            session,
            SessionSubject::new(
                tenant,
                actor,
                "Catalog metadata parity",
                vec![question_model::UserRole::Instructor],
            )
            .expect("valid instructor session subject"),
            SessionLifetime::from_seconds(60).expect("positive session lifetime"),
        )
        .await
        .expect("metadata parity session");
    {
        let mut state = store.write_state().expect("metadata parity state");
        state.instructor_approvals.insert(
            actor,
            crate::StoredInstructorApproval {
                approval: question_model::InstructorApproval {
                    user: actor,
                    approved_by: actor,
                    approved_at: ActivityTimestamp::from_unix_millis(0),
                    revoked_at: None,
                },
                revision: crate::InstructorApprovalRevision::INITIAL,
            },
        );
        let course = CourseId::from_uuid(Uuid::from_u128(73_103));
        state.courses.insert(
            (tenant, course),
            CourseRecord {
                id: course,
                tenant,
                title: "Metadata parity course".to_string(),
                term: question_model::CourseTerm::from_parts(
                    "2026-08-24",
                    "2026-12-18",
                    "America/Chicago",
                )
                .expect("valid metadata parity term"),
            },
        );
        state.course_references.insert(
            (tenant, course),
            question_model::CourseReference::new(73_104).unwrap(),
        );
        let membership = CourseMembershipId::from_uuid(Uuid::from_u128(73_105));
        state.course_memberships.insert(
            (tenant, membership),
            CourseMembershipRecord {
                id: membership,
                tenant,
                course,
                user: actor,
                student: None,
                role: CourseMembershipRole::Instructor,
                roster_id: None,
                status: CourseMemberStatus::Active,
                joined_at: ActivityTimestamp::from_unix_millis(0),
                revoked_at: None,
            },
        );
        let assignment_id = AssignmentId::from_uuid(Uuid::from_u128(73_106));
        state.assignments.insert(
            (tenant, assignment_id),
            AssignmentRecord {
                id: assignment_id,
                tenant,
                course_id: course,
                title: "Metadata parity assignment".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Draft,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: [reference, later_reference]
                    .into_iter()
                    .enumerate()
                    .map(|(position, reference)| question_model::AssignmentItem {
                        id: question_model::AssignmentItemId::from_uuid(Uuid::from_u128(
                            73_107 + position as u128,
                        )),
                        reference,
                        position: u32::try_from(position).expect("fixture position fits"),
                        points_possible: question_model::PointValue::from_whole(1),
                        delivery_state: question_model::AssignmentDeliveryState::Active,
                        scoring_mode: question_model::AssignmentScoringMode::Normal,
                    })
                    .collect(),
                selection_groups: Vec::new(),
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                policies: question_model::RunPolicies {
                    completion: question_model::CompletionRequirement::AnswerAll,
                    grade: question_model::GradePolicy::First,
                    continued_practice: question_model::ContinuedPractice::Unlimited,
                    variation: question_model::VariationPolicy::NewSeeds,
                },
            },
        );
    }

    let page = store
        .search_catalog(
            context,
            session,
            CatalogSearchQuery {
                bylines: vec!["CATALOG TEST AUTHOR".to_string()],
                backends: vec![question_model::QuestionBackend::Native],
                tags: vec!["PEPTIDE".to_string()],
                response_families: vec![question_model::CatalogResponseFamily::Numeric],
                taxonomy: vec![question_model::CatalogTaxonomyFilter {
                    scheme: "discipline".to_string(),
                    code: "biochemistry".to_string(),
                }],
                capabilities: vec![Capability::ServerGrading],
                licenses: vec![CatalogLicenseValue::CcBy],
                used_in_my_courses: question_model::CatalogUsedInMyCourses::Used,
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("metadata parity search");

    assert_eq!(page.items.len(), 1);
    assert_eq!(page.facets.bylines[0].count, 1);
    assert_eq!(
        page.facets.backends[0].backend,
        question_model::QuestionBackend::Native
    );
    assert_eq!(page.facets.tags[0].tag, "peptide");
    assert_eq!(
        page.facets.response_families[0].response_family,
        question_model::CatalogResponseFamily::Numeric
    );
    assert_eq!(page.facets.used_in_my_courses.used, 1);

    let first_page = store
        .search_catalog(
            context,
            session,
            CatalogSearchQuery {
                page_size: Some(1),
                used_in_my_courses: question_model::CatalogUsedInMyCourses::Used,
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("first actor-usage snapshot page");
    let cursor = first_page
        .next_cursor
        .clone()
        .expect("actor-usage continuation cursor");
    store
        .write_state()
        .expect("mutate actor usage")
        .assignments
        .values_mut()
        .next()
        .expect("actor usage assignment")
        .items
        .clear();
    let second_page = store
        .search_catalog(
            context,
            session,
            CatalogSearchQuery {
                page_size: Some(1),
                cursor: Some(cursor.clone()),
                used_in_my_courses: question_model::CatalogUsedInMyCourses::Used,
                ..CatalogSearchQuery::default()
            },
        )
        .await
        .expect("stable actor-usage continuation");
    assert_eq!(second_page.items.len(), 1);
    assert_eq!(second_page.facets.used_in_my_courses.used, 2);

    for membership in store
        .write_state()
        .expect("revoke actor membership")
        .course_memberships
        .values_mut()
        .filter(|membership| membership.user == actor)
    {
        membership.status = CourseMemberStatus::Revoked;
        membership.revoked_at = Some(ActivityTimestamp::from_unix_millis(1));
    }
    assert!(matches!(
        store
            .search_catalog(
                context,
                session,
                CatalogSearchQuery {
                    page_size: Some(1),
                    cursor: Some(cursor),
                    used_in_my_courses: question_model::CatalogUsedInMyCourses::Used,
                    ..CatalogSearchQuery::default()
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn catalog_search_requires_approved_instructor_or_sysadmin_session() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(73_110));
    let context = TenantContext::from_authenticated_session(tenant);
    seed_catalog(&store, [record(73_111)]);
    let mut sessions = Vec::new();
    for (number, role) in [
        (73_112_u128, question_model::UserRole::Student),
        (73_113_u128, question_model::UserRole::Instructor),
        (73_114_u128, question_model::UserRole::Sysadmin),
    ] {
        let token = SessionTokenHash::compute(&number.to_be_bytes());
        store
            .create_session(
                token,
                SessionSubject::new(
                    tenant,
                    UserId::from_uuid(Uuid::from_u128(number)),
                    "Catalog authority parity",
                    vec![role],
                )
                .expect("valid authority session subject"),
                SessionLifetime::from_seconds(60).expect("positive session lifetime"),
            )
            .await
            .expect("authority parity session");
        sessions.push(token);
    }
    let [student, unapproved_instructor, sysadmin] =
        sessions.try_into().expect("three authority sessions");

    assert!(matches!(
        store
            .search_catalog(context, student, CatalogSearchQuery::default())
            .await,
        Err(StoreError::Forbidden)
    ));
    assert!(matches!(
        store
            .search_catalog(
                context,
                unapproved_instructor,
                CatalogSearchQuery::default()
            )
            .await,
        Err(StoreError::Forbidden)
    ));
    let page = store
        .search_catalog(context, sysadmin, CatalogSearchQuery::default())
        .await
        .expect("sysadmin catalog search");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.facets.used_in_my_courses.used, 0);
}

#[tokio::test]
async fn catalog_statistics_filter_facets_and_detail_use_only_k_gated_aggregates() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(73_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let mut published = record(73_002);
    published.question.prompt = vec![ContentBlock::Text {
        markdown: "Fixed catalog prompt.".to_string(),
    }];
    let reference = ProblemVersionRef {
        problem: published.problem,
        version: published.version,
    };
    store
        .write_state()
        .expect("catalog statistics state")
        .published
        .insert((reference.problem, reference.version), published);
    let detail_user = UserId::from_uuid(Uuid::from_u128(73_999));
    let detail_session = SessionTokenHash::compute(b"catalog-detail-evidence-test");
    store
        .write_state()
        .expect("catalog detail authority state")
        .instructor_approvals
        .insert(
            detail_user,
            crate::StoredInstructorApproval {
                approval: question_model::InstructorApproval {
                    user: detail_user,
                    approved_by: detail_user,
                    approved_at: ActivityTimestamp::from_unix_millis(0),
                    revoked_at: None,
                },
                revision: crate::InstructorApprovalRevision::INITIAL,
            },
        );
    store
        .create_session(
            detail_session,
            SessionSubject::new(
                tenant,
                detail_user,
                "Catalog detail",
                vec![question_model::UserRole::Instructor],
            )
            .expect("valid detail session subject"),
            SessionLifetime::from_seconds(60).expect("positive session lifetime"),
        )
        .await
        .expect("detail session");

    for number in 0..4_u128 {
        store
            .record_question_statistics_contribution(
                tenant,
                EnrollmentId::from_uuid(Uuid::from_u128(73_100 + number)),
                RunId::from_uuid(Uuid::from_u128(73_200 + number)),
                QuestionAttemptId::from_uuid(Uuid::from_u128(73_300 + number)),
                reference,
                CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5))
                    .expect("valid observation"),
            )
            .expect("statistics receipt");
    }
    let below_k = store
        .search_catalog_as_instructor(context, CatalogSearchQuery::default())
        .await
        .expect("below-k catalog search");
    assert_eq!(below_k.facets.evidence.available, 0);
    assert_eq!(below_k.facets.evidence.unavailable, 1);
    assert!(
        store
            .search_catalog_as_instructor(
                context,
                CatalogSearchQuery {
                    evidence: CatalogEvidenceAvailability::Available,
                    ..CatalogSearchQuery::default()
                },
            )
            .await
            .expect("below-k available filter")
            .items
            .is_empty()
    );
    assert!(matches!(
        store
            .get_catalog_detail(context, detail_session, reference)
            .await
            .expect("below-k detail")
            .expect("visible detail")
            .evidence,
        question_model::CatalogDiscoveryEvidence::InsufficientEvidence
    ));
    assert_eq!(
        store
            .get_catalog_detail(context, detail_session, reference)
            .await
            .expect("below-k static detail")
            .expect("visible static detail")
            .prompt,
        question_model::CatalogPromptProjection::Static {
            blocks: vec![ContentBlock::Text {
                markdown: "Fixed catalog prompt.".to_string(),
            }],
        }
    );

    store
        .record_question_statistics_contribution(
            tenant,
            EnrollmentId::from_uuid(Uuid::from_u128(73_104)),
            RunId::from_uuid(Uuid::from_u128(73_204)),
            QuestionAttemptId::from_uuid(Uuid::from_u128(73_304)),
            reference,
            CollapsedQuestionObservation::new(0.5, 1, 30, Some(0.5))
                .expect("valid fifth observation"),
        )
        .expect("fifth statistics receipt");
    let at_k = store
        .search_catalog_as_instructor(context, CatalogSearchQuery::default())
        .await
        .expect("at-k catalog search");
    assert_eq!(at_k.facets.evidence.available, 1);
    assert_eq!(at_k.facets.evidence.unavailable, 0);
    assert_eq!(
        store
            .search_catalog_as_instructor(
                context,
                CatalogSearchQuery {
                    evidence: CatalogEvidenceAvailability::Available,
                    ..CatalogSearchQuery::default()
                },
            )
            .await
            .expect("at-k available filter")
            .items
            .len(),
        1
    );
    assert!(matches!(
        store
            .get_catalog_detail(context, detail_session, reference)
            .await
            .expect("at-k detail")
            .expect("visible detail")
            .evidence,
        question_model::CatalogDiscoveryEvidence::Available {
            independent_learner_observation_count: 5,
            ..
        }
    ));

    let seeded = seeded_record(73_003);
    let seeded_reference = ProblemVersionRef {
        problem: seeded.problem,
        version: seeded.version,
    };
    store
        .write_state()
        .expect("seeded catalog state")
        .published
        .insert((seeded_reference.problem, seeded_reference.version), seeded);
    assert_eq!(
        store
            .get_catalog_detail(context, detail_session, seeded_reference)
            .await
            .expect("seeded detail")
            .expect("visible seeded detail")
            .prompt,
        question_model::CatalogPromptProjection::GeneratedExample {
            blocks: vec![ContentBlock::Text {
                markdown: "A glycine example.".to_string(),
            }],
        }
    );
}
