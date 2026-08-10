use super::*;

#[tokio::test]
async fn memory_student_and_group_exceptions_are_most_permissive_and_immediate() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(20_000))
        .expect("fixture clock");
    let tenant = TenantId::from_uuid(uuid(96_000));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor = UserId::from_uuid(uuid(96_001));
    let student = UserId::from_uuid(uuid(96_002));
    let course = CourseId::from_uuid(uuid(96_003));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Accommodation course".to_string(),
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
            },
        )
        .await
        .expect("course");
    let reference = publish_assignment_version(
        &store,
        context,
        tenant,
        instructor,
        96_010,
        PublicationScope::Public,
    )
    .await;
    let assignment = AssignmentId::from_uuid(uuid(96_020));
    let student_record = StudentId::from_uuid(uuid(96_021));
    let created = store
        .create_assignment(
            context,
            AssignmentRecord {
                id: assignment,
                tenant,
                course_id: course,
                title: "Most permissive accommodations".to_string(),
                items: fixed_items(vec![reference]),
                selection_groups: Vec::new(),
                policies: policies(),
            },
        )
        .await
        .expect("assignment");
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: EnrollmentId::from_uuid(uuid(96_022)),
                tenant,
                assignment,
                user: student,
                student: student_record,
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .expect("enrollment");
    let base = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: created.revision,
                policy: AssignmentTimingPolicy {
                    available_at: Some(ActivityTimestamp::from_unix_millis(30_000)),
                    closes_at: Some(ActivityTimestamp::from_unix_millis(60_000)),
                    time_limit_seconds: Some(10),
                    attempt_limit: Some(1),
                    ..AssignmentTimingPolicy::default()
                },
            },
        )
        .await
        .expect("base policy");
    let group_id = CourseGroupId::from_uuid(uuid(96_023));
    store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: group_id,
                    tenant,
                    course,
                    title: "Extended testing".to_string(),
                    members: vec![student],
                },
            },
        )
        .await
        .expect("course group");
    let group_exception = AssignmentPolicyException {
        id: AssignmentPolicyExceptionId::from_uuid(uuid(96_024)),
        target: AssignmentPolicyExceptionTarget::CourseGroup(group_id),
        available_at: Some(AssignmentExceptionTimestamp::Unrestricted),
        closes_at: Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(80_000),
        )),
        time_limit_seconds: Some(AssignmentExceptionLimit::Value(20)),
        attempt_limit: Some(AssignmentExceptionLimit::Value(2)),
    };
    let group_exception = store
        .set_assignment_policy_exception(
            context,
            SetAssignmentPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: base.revision,
                exception: group_exception,
            },
        )
        .await
        .expect("group exception");
    let student_exception = AssignmentPolicyException {
        id: AssignmentPolicyExceptionId::from_uuid(uuid(96_025)),
        target: AssignmentPolicyExceptionTarget::Student(student_record),
        available_at: Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(25_000),
        )),
        closes_at: Some(AssignmentExceptionTimestamp::At(
            ActivityTimestamp::from_unix_millis(70_000),
        )),
        time_limit_seconds: Some(AssignmentExceptionLimit::Value(15)),
        attempt_limit: Some(AssignmentExceptionLimit::Value(3)),
    };
    let student_command = SetAssignmentPolicyExceptionCommand {
        actor: instructor,
        course,
        assignment,
        expected_revision: group_exception.assignment_revision,
        exception: student_exception.clone(),
    };
    assert_eq!(
        store
            .set_assignment_policy_exception(
                context,
                SetAssignmentPolicyExceptionCommand {
                    actor: student,
                    ..student_command.clone()
                },
            )
            .await,
        Err(StoreError::NotFound)
    );
    let stored_student = store
        .set_assignment_policy_exception(context, student_command.clone())
        .await
        .expect("student exception");
    assert_eq!(
        store
            .set_assignment_policy_exception(context, student_command)
            .await,
        Ok(stored_student.clone()),
        "an exact exception retry is revision-stable"
    );

    let resolved = store
        .resolve_assignment_timing(context, assignment, student_record)
        .await
        .expect("resolve policy")
        .expect("enrollment policy");
    assert_eq!(resolved.policy.available_at, None);
    assert_eq!(
        resolved.policy.closes_at,
        Some(ActivityTimestamp::from_unix_millis(80_000))
    );
    assert_eq!(resolved.policy.time_limit_seconds, Some(20));
    assert_eq!(resolved.policy.attempt_limit, Some(3));
    assert_eq!(
        resolved.contributors,
        vec![
            AssignmentPolicyExceptionTarget::Student(student_record),
            AssignmentPolicyExceptionTarget::CourseGroup(group_id),
        ]
    );
    let run = store
        .start_or_resume_run(context, student, assignment, RunId::from_uuid(uuid(96_026)))
        .await
        .expect("exception opens assignment early");
    let attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: QuestionAttemptId::from_uuid(uuid(96_027)),
                run: run.id,
                assignment_position: 0,
                problem: reference.problem,
                question_version: reference.version,
                seed: 5,
                presentation: presentation_binding(13),
                parameter_hash: "exception-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("timing-native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("timing-numeric"),
                    rendered_question_sha256: "exception-render".to_string(),
                },
                webwork_replay: None,
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .expect("exception-timed attempt");
    assert_eq!(
        attempt.timer.deadline,
        Some(ActivityTimestamp::from_unix_millis(40_000))
    );
    let recorded = store
        .get_attempt_resolved_timing(context, attempt.id)
        .await
        .expect("attempt policy")
        .expect("attempt resolution");
    assert_eq!(recorded.policy, resolved.policy);
    assert_eq!(recorded.contributors, resolved.contributors);

    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(35_000))
        .expect("advance beyond direct timer");
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Accommodation course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("remove student membership");
    let empty_group = store
        .get_course_group(context, group_id)
        .await
        .expect("group after course membership update")
        .expect("group remains");
    assert!(empty_group.record.members.is_empty());
    let empty_group_command = PutCourseGroupCommand {
        actor: instructor,
        expected_revision: Some(empty_group.revision),
        record: empty_group.record.clone(),
    };
    assert_eq!(
        store.put_course_group(context, empty_group_command).await,
        Ok(empty_group.clone())
    );
    assert_eq!(
        store
            .get_question_attempt(context, attempt.id)
            .await
            .expect("closed attempt read")
            .expect("attempt remains")
            .status,
        AttemptStatus::AutoSubmitted
    );
    let terminal_resolution = store
        .get_attempt_resolved_timing(context, attempt.id)
        .await
        .expect("terminal policy")
        .expect("terminal resolution remains");
    assert_eq!(terminal_resolution.policy.time_limit_seconds, Some(15));
    assert_eq!(
        terminal_resolution.contributors,
        vec![AssignmentPolicyExceptionTarget::Student(student_record)]
    );

    let other_course = CourseId::from_uuid(uuid(96_028));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: other_course,
                tenant,
                title: "Other accommodation course".to_string(),
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
            },
        )
        .await
        .expect("other course");
    assert_eq!(
        store
            .put_course_group(
                context,
                PutCourseGroupCommand {
                    actor: instructor,
                    expected_revision: Some(empty_group.revision),
                    record: CourseGroupRecord {
                        course: other_course,
                        ..empty_group.record.clone()
                    },
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a stable group identity cannot move between courses"
    );

    let after_student_delete = store
        .delete_assignment_policy_exception(
            context,
            DeleteAssignmentPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: stored_student.assignment_revision,
                exception: student_exception.id,
            },
        )
        .await
        .expect("delete student exception");
    let after_group_delete = store
        .delete_assignment_policy_exception(
            context,
            DeleteAssignmentPolicyExceptionCommand {
                actor: instructor,
                course,
                assignment,
                expected_revision: after_student_delete,
                exception: group_exception.exception.id,
            },
        )
        .await
        .expect("delete group exception");
    let base_again = store
        .resolve_assignment_timing(context, assignment, student_record)
        .await
        .expect("base resolution")
        .expect("enrollment remains");
    assert_eq!(base_again.revision, after_group_delete);
    assert!(base_again.contributors.is_empty());
    assert_eq!(base_again.policy.time_limit_seconds, Some(10));
}
