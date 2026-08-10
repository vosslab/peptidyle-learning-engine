use super::*;

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
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(seed)),
        version: VersionId::from_uuid(uuid(seed + 1)),
    };
    let draft = DraftRecord {
        tenant,
        question: draft_question(WorkspaceId::from_uuid(uuid(seed + 2))),
        revises: None,
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
    S: Store + CatalogStore,
{
    let tenant = TenantId::from_uuid(uuid(70_000));
    let foreign_tenant = TenantId::from_uuid(uuid(70_001));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor = UserId::from_uuid(uuid(70_002));
    let course = CourseId::from_uuid(uuid(70_003));
    let wrong_course = CourseId::from_uuid(uuid(70_004));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Assignment CAS course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("assignment CAS course");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: wrong_course,
                tenant,
                title: "Other course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("wrong-course fixture");
    let foreign_course = CourseId::from_uuid(uuid(70_005));
    store
        .upsert_course(
            foreign_context,
            CourseRecord {
                id: foreign_course,
                tenant: foreign_tenant,
                title: "Foreign course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
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

    let assignment = AssignmentId::from_uuid(uuid(70_200));
    let initial = AssignmentRecord {
        id: assignment,
        tenant,
        course_id: course,
        title: "Ordered source selection".to_string(),
        items: fixed_items(vec![published, deprecated]),
        selection_groups: Vec::new(),
        policies: policies(),
    };
    let created = store
        .create_assignment(context, initial.clone())
        .await
        .expect("published and deprecated versions are assignable");
    assert_eq!(created.revision.value(), 1);
    assert_eq!(created.record, initial);

    let updated_policies = RunPolicies {
        completion: CompletionRequirement::AnswerAll,
        grade: GradePolicy::Latest,
        continued_practice: ContinuedPractice::Closed,
        variation: VariationPolicy::SelectedProblemVariants,
    };
    let update = AssignmentUpdate {
        title: "Reordered source selection".to_string(),
        items: fixed_items(vec![deprecated, published]),
        selection_groups: Vec::new(),
        policies: updated_policies,
    };
    let updated = store
        .replace_assignment(
            context,
            course,
            assignment,
            created.revision,
            update.clone(),
        )
        .await
        .expect("fresh assignment revision updates");
    assert_eq!(updated.revision.value(), 2);
    assert_eq!(updated.record.items, update.items);
    assert_eq!(updated.record.policies, update.policies);
    assert_eq!(updated.record.title, update.title);
    assert_eq!(
        store
            .replace_assignment(
                context,
                course,
                assignment,
                created.revision,
                update.clone()
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
    assert_eq!(
        store
            .replace_assignment(
                context,
                wrong_course,
                assignment,
                updated.revision,
                update.clone()
            )
            .await,
        Err(StoreError::NotFound),
        "a course path cannot move an assignment"
    );
    assert_eq!(
        store
            .replace_assignment(
                foreign_context,
                course,
                assignment,
                updated.revision,
                update.clone()
            )
            .await,
        Err(StoreError::NotFound),
        "foreign tenant must not enumerate assignment identity"
    );
    assert_eq!(
        store
            .get_assignment_for_edit(context, assignment)
            .await
            .expect("failed writes leave assignment unchanged"),
        Some(updated.clone())
    );

    assert!(matches!(
        store
            .create_assignment(
                context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_201)),
                    tenant,
                    course_id: course,
                    title: "archived reference".to_string(),
                    items: fixed_items(vec![archived]),
                    selection_groups: Vec::new(),
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    assert!(matches!(
        store
            .create_assignment(
                foreign_context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_202)),
                    tenant: foreign_tenant,
                    course_id: foreign_course,
                    title: "hidden reference".to_string(),
                    items: fixed_items(vec![hidden]),
                    selection_groups: Vec::new(),
                    policies: policies(),
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
    let repeated = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: AssignmentId::from_uuid(uuid(70_203)),
                tenant,
                course_id: course,
                title: "Repeated immutable version positions".to_string(),
                items: fixed_items(vec![published, published]),
                selection_groups: Vec::new(),
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
            .create_assignment(
                context,
                AssignmentRecord {
                    id: AssignmentId::from_uuid(uuid(70_204)),
                    tenant,
                    course_id: course,
                    title: "Invalid completion threshold".to_string(),
                    items: fixed_items(vec![published]),
                    selection_groups: Vec::new(),
                    policies: invalid_threshold,
                },
            )
            .await,
        Err(StoreError::InvalidRecord(_))
    ));
}
