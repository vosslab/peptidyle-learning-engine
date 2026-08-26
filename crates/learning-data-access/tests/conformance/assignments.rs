use super::*;

pub(super) use super::assignment_workspace::exercise_assignment_workspace_slices;

async fn issued_native_snapshot<S>(
    store: &S,
    context: TenantContext,
    reference: ProblemVersionRef,
) -> learning_data_access::IssuedQuestionSnapshotV1
where
    S: CatalogStore + ?Sized,
{
    let question = store
        .get_catalog_problem(context, reference)
        .await
        .expect("assignment fixture catalog lookup")
        .expect("assignment fixture publication")
        .question;
    learning_data_access::IssuedQuestionSnapshotV1::new(
        question,
        learning_data_access::IssuedQuestionFamilyWitnessV1::Native {
            physical_asset_bindings: Vec::new(),
        },
    )
    .expect("assignment fixture native issued snapshot")
}

pub(super) async fn publish_assignment_version<S>(
    store: &S,
    context: TenantContext,
    tenant: TenantId,
    author: UserId,
    seed: u128,
    scope: PublicationScope,
) -> ProblemVersionRef
where
    S: Store + CatalogStore,
{
    publish_assignment_version_with_timing(
        store,
        context,
        tenant,
        author,
        seed,
        scope,
        question_model::run_policy::TimingPolicy::Untimed,
    )
    .await
}

/// Publishes ordinary native material with an authored timing policy.  Timing
/// conformance uses this normal draft/publication path rather than mutating issued learner work.
pub(super) async fn publish_assignment_version_with_timing<S>(
    store: &S,
    context: TenantContext,
    tenant: TenantId,
    author: UserId,
    seed: u128,
    scope: PublicationScope,
    timing_policy: question_model::run_policy::TimingPolicy,
) -> ProblemVersionRef
where
    S: Store + CatalogStore,
{
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(seed)),
        version: VersionId::from_uuid(uuid(seed + 1)),
    };
    let mut question = draft_question(WorkspaceId::from_uuid(uuid(seed + 2)));
    question.timing_policy = timing_policy;
    let draft = DraftRecord {
        tenant,
        question,
        derived_from: None,
    };
    let saved = store
        .upsert_draft(context, author, None, draft.clone())
        .await
        .expect("assignment fixture draft");
    store
        .publish_draft(
            context,
            author,
            PublishDraftCommand {
                expected_draft: draft,
                expected_revision: saved.revision,
                publication: reference,
                published_source: published_source(),
                source_artifact: None,
                qti_promotion: None,
                flat_question_promotion: None,
                publisher: author,
                scope,
                byline: reviewed_byline(),
                capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
            },
        )
        .await
        .expect("assignment fixture publication");
    reference
}

/// Exercises the revisioned assignment edit contract independently of HTTP.
/// Every Store backend must retain exact ordering/policies, refuse stale or
/// cross-course writes without mutation, and apply catalog visibility/lifecycle
/// rules before accepting a new course artifact.
pub(super) async fn exercise_assignment_cas<S>(store: &S)
where
    S: Store + CatalogStore + CourseRosterStore + SessionStore,
{
    let tenant = TenantId::from_uuid(uuid(70_000));
    let foreign_tenant = TenantId::from_uuid(uuid(70_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(uuid(70_002));
    let student = UserId::from_uuid(uuid(70_006));
    let future_student = UserId::from_uuid(uuid(70_007));
    let course = CourseId::from_uuid(uuid(70_003));
    let wrong_course = CourseId::from_uuid(uuid(70_004));
    let course_creation_authority =
        sysadmin_course_creation_authority(store, tenant, course, instructor).await;
    store
        .create_course(
            context,
            learning_data_access::CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Assignment CAS course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: course_creation_authority,
            },
        )
        .await
        .expect("assignment CAS course");
    for (user, display_name) in [
        (student, "Assignment learner"),
        (future_student, "Future assignment learner"),
    ] {
        store
            .upsert_course_member(
                context,
                instructor,
                learning_data_access::UpsertCourseMember {
                    course,
                    user,
                    display_name: display_name.to_string(),
                    roster_contact: None,
                },
            )
            .await
            .expect("assignment learner membership");
    }
    store
        .create_course(
            context,
            learning_data_access::CreateCourseCommand {
                course: CourseRecord {
                    id: wrong_course,
                    tenant,
                    title: "Other course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: sysadmin_course_creation_authority(
                    store,
                    tenant,
                    wrong_course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("wrong-course fixture");
    let foreign_course = CourseId::from_uuid(uuid(70_005));
    store
        .create_course(
            foreign_context,
            learning_data_access::CreateCourseCommand {
                course: CourseRecord {
                    id: foreign_course,
                    tenant: foreign_tenant,
                    title: "Foreign course".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("explicit fixture course term"),
                },
                authority: sysadmin_course_creation_authority(
                    store,
                    foreign_tenant,
                    foreign_course,
                    instructor,
                )
                .await,
            },
        )
        .await
        .expect("foreign course fixture");

    let published = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_100,
        PublicationScope::Public,
    )
    .await;
    let deprecated = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_110,
        PublicationScope::Public,
    )
    .await;
    store
        .transition_catalog_problem(
            context,
            instructor,
            deprecated,
            CatalogTransition::Deprecate {
                reason: "Revised but usable".to_string(),
            },
        )
        .await
        .expect("deprecated fixture");
    let archived = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_120,
        PublicationScope::Public,
    )
    .await;
    store
        .transition_catalog_problem(
            context,
            instructor,
            archived,
            CatalogTransition::Deprecate {
                reason: "Archive fixture".to_string(),
            },
        )
        .await
        .expect("archive deprecation");
    store
        .transition_catalog_problem(context, instructor, archived, CatalogTransition::Archive)
        .await
        .expect("archive fixture");
    let hidden = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_130,
        PublicationScope::Institution,
    )
    .await;
    let replacement_reference = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_140,
        PublicationScope::Public,
    )
    .await;
    let post_run_replacement_reference = publish_assignment_version(
        store,
        context,
        tenant,
        instructor,
        70_150,
        PublicationScope::Public,
    )
    .await;

    let assignment = AssignmentId::from_uuid(uuid(70_200));
    let mut initial_items = fixed_items(vec![published, deprecated]);
    initial_items[1].position = 2;
    let initial = AssignmentRecord {
        id: assignment,
        tenant,
        course_id: course,
        title: "Ordered source selection".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Published,
        instructions: question_model::AssignmentInstructions::default(),
        audience: question_model::AssignmentAudience::CourseWide,
        items: initial_items,
        selection_groups: vec![AssignmentSelectionGroup {
            id: AssignmentSelectionGroupId::from_uuid(uuid(70_208)),
            position: 1,
            draw_count: 1,
            points_per_item: PointValue::from_whole(1),
            ordering: SelectionOrdering::CandidateOrder,
            algorithm: question_model::PoolDrawAlgorithm::V1,
            candidates: vec![AssignmentSelectionCandidate {
                id: AssignmentItemId::from_uuid(uuid(70_209)),
                position: 0,
                reference: published,
                delivery_state: AssignmentDeliveryState::Active,
            }],
        }],
        disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
        policies: policies(),
    };
    let created = store
        .create_assignment_with_default_policy(context, instructor, initial.clone())
        .await
        .expect("published and deprecated versions are assignable");
    assert_eq!(created.revision.value(), 2);
    assert_eq!(created.record, initial);

    let updated_policies = RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Latest,
        continued_practice: ContinuedPractice::Closed,
        variation: VariationPolicy::FullRegeneration,
    };
    let mut reordered_items = initial.items.clone();
    reordered_items.reverse();
    reordered_items[0].position = 0;
    reordered_items[1].position = 2;
    let update = AssignmentUpdate {
        title: "Reordered source selection".to_string(),
        audience: initial.audience.clone(),
        items: reordered_items,
        selection_groups: initial.selection_groups.clone(),
        disclosure_policy: initial.disclosure_policy,
        policies: updated_policies,
    };
    let updated = store
        .replace_assignment(
            context,
            ReplaceAssignmentCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: created.revision,
                update: update.clone(),
            },
        )
        .await
        .expect("fresh assignment revision updates");
    assert_eq!(updated.revision.value(), 3);
    assert_eq!(updated.record.items, update.items);
    assert_eq!(updated.record.policies, update.policies);
    assert_eq!(updated.record.title, update.title);
    assert_eq!(
        store
            .replace_assignment(
                context,
                ReplaceAssignmentCommand {
                    actor: instructor,
                    course,
                    assignment,
                    expected_revision: created.revision,
                    update: update.clone(),
                }
            )
            .await,
        Err(StoreError::Conflict),
        "stale revision must not overwrite"
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("read updated assignment"),
        Some(updated.clone())
    );

    let replaced_item = updated.record.items[0].id;
    let replacement = store
        .replace_assignment_fixed_item(
            context,
            ReplaceAssignmentFixedItemCommand {
                actor: instructor,
                course,
                assignment,
                current_item: updated.record.items[0].id,
                expected_revision: updated.revision,
                replacement: replacement_reference,
            },
        )
        .await
        .expect("focused replacement updates future assignment definition");
    assert_eq!(replacement.record.items[0].id, replaced_item);
    assert_eq!(replacement.record.items[0].reference, replacement_reference);
    assert_eq!(
        replacement.record.items[0].points_possible, updated.record.items[0].points_possible,
        "replacement retains the assignment-authored slot settings"
    );
    assert_eq!(
        store
            .replace_assignment_fixed_item(
                context,
                ReplaceAssignmentFixedItemCommand {
                    actor: instructor,
                    course,
                    assignment,
                    current_item: updated.record.items[0].id,
                    expected_revision: updated.revision,
                    replacement: replacement_reference,
                },
            )
            .await,
        Err(StoreError::Conflict),
        "focused replacement uses the assignment revision as a strong CAS"
    );
    let inserted_item = AssignmentItem {
        id: AssignmentItemId::from_uuid(uuid(70_207)),
        reference: published,
        position: 1,
        points_possible: PointValue::from_whole(2),
        delivery_state: AssignmentDeliveryState::Active,
        scoring_mode: AssignmentScoringMode::Normal,
    };
    let added = store
        .add_assignment_fixed_item(
            context,
            AddAssignmentFixedItemCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: replacement.revision,
                item: inserted_item.clone(),
            },
        )
        .await
        .expect("pre-evidence add inserts at the requested future-run position");
    assert_eq!(added.record.items[1].id, inserted_item.id);
    assert_eq!(added.record.selection_groups[0].position, 2);
    let removed = store
        .remove_assignment_fixed_item(
            context,
            RemoveAssignmentFixedItemCommand {
                actor: instructor,
                course,
                assignment,
                item: inserted_item.id,
                expected_revision: added.revision,
            },
        )
        .await
        .expect("pre-evidence removal changes the future definition");
    assert!(
        removed
            .record
            .items
            .iter()
            .all(|item| item.id != inserted_item.id)
    );
    assert_eq!(removed.record.selection_groups[0].position, 1);
    assert_eq!(removed.record.items[1].position, 2);

    let run = store
        .start_or_resume_run(
            context,
            student,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(uuid(70_211)),
        )
        .await
        .expect("run start atomically materializes the learner receipt");
    let old_run_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                binding: LearnerWorkRoutingBinding::new(course, assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(70_214)),
                run: run.id,
                assignment_position: 0,
                problem: replacement_reference.problem,
                question_version: replacement_reference.version,
                issued_question_snapshot: issued_native_snapshot(
                    store,
                    context,
                    replacement_reference,
                )
                .await,
                seed: 42,
                parameter_hash: "assignment-snapshot".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: Some(generator("molar-mass")),
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("numeric"),
                    rendered_question_sha256: "assignment-snapshot".to_string(),
                },
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_replay: None,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("issued attempt freezes the pre-replacement exact immutable reference");
    assert_eq!(
        (old_run_attempt.problem, old_run_attempt.question_version),
        (replacement_reference.problem, replacement_reference.version)
    );
    let issued_policy_before_replacement = store
        .get_issued_effective_policy_receipt(context, old_run_attempt.id)
        .await
        .expect("read issued policy before content replacement")
        .expect("issued attempt has an immutable policy receipt");
    let post_run_replacement = store
        .replace_assignment_fixed_item(
            context,
            ReplaceAssignmentFixedItemCommand {
                actor: instructor,
                course,
                assignment,
                current_item: replaced_item,
                expected_revision: removed.revision,
                replacement: post_run_replacement_reference,
            },
        )
        .await
        .expect("post-run replacement changes only the future assignment definition");
    assert_eq!(post_run_replacement.record.items[0].id, replaced_item);
    assert_eq!(
        post_run_replacement.record.items[0].reference,
        post_run_replacement_reference
    );
    let issued_policy_after_replacement = store
        .get_issued_effective_policy_receipt(context, old_run_attempt.id)
        .await
        .expect("read issued policy after content replacement")
        .expect("content replacement preserves the issued policy receipt");
    assert_eq!(
        issued_policy_after_replacement, issued_policy_before_replacement,
        "future-content replacement must not create a new timing-policy generation"
    );
    let receipt = store
        .submit_question_attempt(
            context,
            SubmitQuestionAttemptCommand {
                actor: student,
                binding: LearnerWorkRoutingBinding::new(course, assignment),
                attempt: old_run_attempt.id,
                response: StudentResponse::Numeric { value: 18.0 },
                result: AttemptResult {
                    correct: true,
                    points_earned: 1.0,
                    points_possible: 1.0,
                },
                feedback: FeedbackContent::default(),
                idempotency_key: SubmissionIdempotencyKey::parse("stable-slot-replacement")
                    .expect("valid submission key"),
            },
        )
        .await
        .expect("old issued item grades through its stable assignment slot");
    assert_eq!(receipt.attempt.id, old_run_attempt.id);
    let future_run = store
        .start_or_resume_run(
            context,
            future_student,
            LearnerWorkRoutingBinding::new(course, assignment),
            RunId::from_uuid(uuid(70_217)),
        )
        .await
        .expect("future run snapshots the replacement definition");
    let future_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: future_student,
                binding: LearnerWorkRoutingBinding::new(course, assignment),
                attempt: QuestionAttemptId::from_uuid(uuid(70_219)),
                run: future_run.id,
                assignment_position: 0,
                problem: post_run_replacement_reference.problem,
                question_version: post_run_replacement_reference.version,
                issued_question_snapshot: issued_native_snapshot(
                    store,
                    context,
                    post_run_replacement_reference,
                )
                .await,
                seed: 43,
                parameter_hash: "future-assignment-snapshot".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: Some(generator("molar-mass")),
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("numeric"),
                    rendered_question_sha256: "future-assignment-snapshot".to_string(),
                },
                presentation_capability: PresentationCapability::NotApplicable,
                presentation: None,
                presentation_snapshot: None,
                grading_envelope: None,
                native_execution_envelope_capability: NativeExecutionEnvelopeCapability::Required,
                flat_grading: None,
                flat_grading_capability: FlatGradingCapability::NotApplicable,
                webwork_replay: None,
                webwork_grading: None,
                webwork_grading_capability: WebworkGradingCapability::NotApplicable,
                qti_grading: None,
                qti_grading_capability: QtiGradingCapability::NotApplicable,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("future run issues the replacement publication");
    assert_eq!(
        (future_attempt.problem, future_attempt.question_version),
        (
            post_run_replacement_reference.problem,
            post_run_replacement_reference.version
        )
    );
    assert_eq!(
        store
            .add_assignment_fixed_item(
                context,
                AddAssignmentFixedItemCommand {
                    actor: instructor,
                    course,
                    assignment,
                    expected_revision: post_run_replacement.revision,
                    item: inserted_item.clone(),
                },
            )
            .await,
        Err(StoreError::Conflict)
    );
    assert_eq!(
        store
            .remove_assignment_fixed_item(
                context,
                RemoveAssignmentFixedItemCommand {
                    actor: instructor,
                    course,
                    assignment,
                    item: replaced_item,
                    expected_revision: post_run_replacement.revision,
                },
            )
            .await,
        Err(StoreError::Conflict)
    );
    let current_definition = AssignmentUpdate {
        title: post_run_replacement.record.title.clone(),
        audience: post_run_replacement.record.audience.clone(),
        items: post_run_replacement.record.items.clone(),
        selection_groups: post_run_replacement.record.selection_groups.clone(),
        disclosure_policy: post_run_replacement.record.disclosure_policy,
        policies: post_run_replacement.record.policies,
    };
    assert_eq!(
        store
            .replace_assignment(
                context,
                ReplaceAssignmentCommand {
                    actor: instructor,
                    course: wrong_course,
                    assignment,
                    expected_revision: post_run_replacement.revision,
                    update: current_definition.clone(),
                }
            )
            .await,
        Err(StoreError::NotFound),
        "a course path cannot move an assignment"
    );
    assert_eq!(
        store
            .replace_assignment(
                foreign_context,
                ReplaceAssignmentCommand {
                    actor: instructor,
                    course,
                    assignment,
                    expected_revision: post_run_replacement.revision,
                    update: current_definition.clone(),
                }
            )
            .await,
        Err(StoreError::NotFound),
        "foreign tenant must not enumerate assignment identity"
    );
    assert_eq!(
        store
            .replace_assignment_fixed_item(
                foreign_context,
                ReplaceAssignmentFixedItemCommand {
                    actor: instructor,
                    course,
                    assignment,
                    current_item: replaced_item,
                    expected_revision: post_run_replacement.revision,
                    replacement: post_run_replacement_reference,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "foreign tenant cannot invoke the focused replacement broker"
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("failed writes leave assignment unchanged"),
        Some(post_run_replacement.clone())
    );

    assert!(matches!(
        store
            .create_assignment_with_default_policy(
                context,
                instructor,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_201)),
                    tenant,
                    course_id: course,
                    title: "archived reference".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    audience: question_model::AssignmentAudience::CourseWide,
                    items: fixed_items(vec![archived]),
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(matches!(
        store
            .create_assignment_with_default_policy(
                foreign_context,
                instructor,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_202)),
                    tenant: foreign_tenant,
                    course_id: foreign_course,
                    title: "hidden reference".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    audience: question_model::AssignmentAudience::CourseWide,
                    items: fixed_items(vec![hidden]),
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let repeated = store
        .create_assignment_with_default_policy(
            context,
            instructor,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(70_203)),
                tenant,
                course_id: course,
                title: "Repeated immutable version positions".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![published, published]),
                selection_groups: Vec::new(),
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                policies: policies(),
            },
        )
        .await
        .expect("one immutable version may occupy distinct ordered positions");
    assert_eq!(
        repeated.record.references().collect::<Vec<_>>(),
        vec![published, published]
    );
    let invalid_threshold = RunPolicies {
        completion: CompletionRequirement::ScoreAtLeast { fraction: 1.1 },
        ..policies()
    };
    assert!(matches!(
        store
            .create_assignment_with_default_policy(
                context,
                instructor,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_204)),
                    tenant,
                    course_id: course,
                    title: "Invalid completion threshold".to_string(),
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    audience: question_model::AssignmentAudience::CourseWide,
                    items: fixed_items(vec![published]),
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: invalid_threshold,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
