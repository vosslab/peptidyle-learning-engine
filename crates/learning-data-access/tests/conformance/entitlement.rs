use super::*;
use domain::effective_assignment_policy::BaseAssignmentPolicy;
use learning_data_access::PutAssignmentTeachingSettingsCommand;

#[tokio::test]
async fn memory_entitlement_pages_only_over_visible_assignments() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(uuid(98_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(98_001));
    let learner = UserId::from_uuid(uuid(98_002));
    let course = CourseId::from_uuid(uuid(98_003));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Entitlement pagination".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid fixture term"),
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
                user: learner,
                display_name: "Visible learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("learner membership");
    let hidden_group = CourseGroupId::from_uuid(uuid(98_004));
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: hidden_group,
                    tenant,
                    course,
                    purpose: question_model::CourseGroupPurpose::Section,
                    title: "Another section".to_string(),
                    members: Vec::new(),
                },
            },
        )
        .await
        .expect("hidden audience group");
    let reference = publish_assignment_version(
        &store,
        context,
        tenant,
        instructor,
        98_010,
        PublicationScope::Public,
    )
    .await;
    let first_visible = AssignmentId::from_uuid(uuid(98_101));
    let policy_denied = AssignmentId::from_uuid(uuid(98_102));
    let second_visible = AssignmentId::from_uuid(uuid(98_103));
    for (id, audience) in [
        (
            AssignmentId::from_uuid(uuid(98_100)),
            question_model::AssignmentAudience::any_of_groups(vec![hidden_group])
                .expect("nonempty hidden audience"),
        ),
        (
            first_visible,
            question_model::AssignmentAudience::CourseWide,
        ),
        (
            policy_denied,
            question_model::AssignmentAudience::CourseWide,
        ),
        (
            second_visible,
            question_model::AssignmentAudience::CourseWide,
        ),
    ] {
        store
            .create_assignment_with_default_policy(
                context,
                instructor,
                AssignmentRecord {
                    id,
                    tenant,
                    course_id: course,
                    title: format!("Assignment {id}"),
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    audience,
                    items: fixed_items(vec![reference]),
                    selection_groups: Vec::new(),
                    disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                    policies: policies(),
                },
            )
            .await
            .expect("assignment");
    }
    let default_policy = store
        .get_base_assignment_policy(context, policy_denied)
        .await
        .expect("read default policy")
        .expect("assignment owns an S3 base policy");
    store
        .put_assignment_teaching_settings(
            context,
            PutAssignmentTeachingSettingsCommand {
                actor: instructor,
                course,
                assignment: policy_denied,
                expected_revision: default_policy.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: question_model::AssignmentInstructions::default(),
                    base_policy: BaseAssignmentPolicy {
                        available_at: Some(ActivityTimestamp::from_unix_millis(1_787_590_800_000)),
                        due_at: None,
                        closes_at: None,
                        time_limit_seconds: None,
                        attempt_limit: None,
                        late_submission: question_model::LateSubmissionPolicy::Accept,
                        deadline_behavior: question_model::AssignmentDeadlineBehavior::AutoSubmit,
                    },
                },
            },
        )
        .await
        .expect("create S3-denied candidate between visible assignments");

    let first = store
        .list_learner_entitled_assignments(
            context,
            learner,
            course,
            PageRequest::first(PageSize::new(1).expect("valid page size")),
        )
        .await
        .expect("first S5-and-S3-visible page");
    assert_eq!(
        first.items.iter().map(|item| item.id).collect::<Vec<_>>(),
        vec![first_visible]
    );
    let second = store
        .list_learner_entitled_assignments(
            context,
            learner,
            course,
            PageRequest::after(
                first
                    .next_cursor
                    .expect("cursor follows only the visible assignment"),
                PageSize::new(1).expect("valid page size"),
            ),
        )
        .await
        .expect("second S5-and-S3-visible page");
    assert_eq!(
        second.items.iter().map(|item| item.id).collect::<Vec<_>>(),
        vec![second_visible]
    );
    assert!(second.next_cursor.is_none());
}

#[tokio::test]
async fn memory_entitlement_materialization_enforces_closed_authority_matrix() {
    let store = MemoryStore::default();
    let tenant = TenantId::from_uuid(uuid(98_200));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(98_201));
    let learner = UserId::from_uuid(uuid(98_202));
    let course = CourseId::from_uuid(uuid(98_203));
    store
        .create_course(
            context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: course,
                    tenant,
                    title: "Entitlement authority".to_string(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("valid fixture term"),
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
                user: learner,
                display_name: "Authority learner".to_string(),
                roster_contact: None,
            },
        )
        .await
        .expect("learner membership");
    let reference = publish_assignment_version(
        &store,
        context,
        tenant,
        instructor,
        98_210,
        PublicationScope::Public,
    )
    .await;
    let assignment = AssignmentId::from_uuid(uuid(98_211));
    store
        .create_assignment_with_default_policy(
            context,
            instructor,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Authority matrix".to_string(),
                lifecycle: question_model::AssignmentLifecycle::Published,
                instructions: question_model::AssignmentInstructions::default(),
                audience: question_model::AssignmentAudience::CourseWide,
                items: fixed_items(vec![reference]),
                selection_groups: Vec::new(),
                disclosure_policy: question_model::LearnerDisclosurePolicy::default(),
                policies: policies(),
            },
        )
        .await
        .expect("assignment");

    let outsider = UserId::from_uuid(uuid(98_212));
    assert_eq!(
        store
            .issue_assignment_entitlement(
                context,
                MaterializeAssignmentEntitlementCommand::for_instructor_action(
                    learner,
                    course,
                    assignment,
                    outsider,
                    EntitlementPurpose::InstructorIssue,
                )
                .expect("typed outsider issue command"),
            )
            .await,
        Err(StoreError::Forbidden)
    );
    let issued = store
        .issue_assignment_entitlement(
            context,
            MaterializeAssignmentEntitlementCommand::for_rule_grade(
                learner,
                course,
                assignment,
                question_model::MaterializationRule::AutomatedGrader,
            ),
        )
        .await
        .expect("valid rule-backed grade materialization");
    let learning_data_access::AssignmentEntitlementMaterialization::Granted(issued) = issued else {
        panic!("current course-wide learner must be entitled")
    };
    assert_eq!(
        issued.provenance.authority,
        question_model::MaterializationAuthority::Rule(
            question_model::MaterializationRule::AutomatedGrader
        )
    );
    assert_eq!(
        issued.disposition,
        question_model::MaterializationDisposition::Created,
        "the refused outsider issue must not have created a receipt"
    );
}
