//! Host-only E2E seed timing capability.

use super::*;

pub(super) async fn exercise_assignment_timing(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    instructor: UserId,
    student: UserId,
    ids: SeedIds,
) -> Result<()> {
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let timing_assignment = AssignmentRecord {
        id: ids.timing_assignment,
        tenant: context.tenant_id(),
        course_id: ids.course,
        title: "PLE mutable timing acceptance".to_string(),
        items: [
            ids.timing_assignment_item_one,
            ids.timing_assignment_item_two,
            ids.timing_assignment_item_three,
        ]
        .into_iter()
        .enumerate()
        .map(|(position, id)| AssignmentItem {
            id,
            reference,
            position: u32::try_from(position).expect("three positions fit"),
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        })
        .collect(),
        selection_groups: Vec::new(),
        policies: RunPolicies {
            completion: CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        },
    };
    let created = store
        .create_assignment(context, timing_assignment)
        .await
        .context("creating mutable timing acceptance assignment")?;
    store
        .create_enrollment(
            context,
            AssignmentEnrollment {
                id: ids.timing_enrollment,
                tenant: context.tenant_id(),
                assignment: ids.timing_assignment,
                user: student,
                student: StudentId::from_uuid(student.as_uuid()),
                first_completed_at: None,
                current_grade_run: None,
                best_grade_run: None,
            },
        )
        .await
        .context("creating mutable timing acceptance enrollment")?;
    let hidden = AssignmentTimingPolicy {
        visible: false,
        ..AssignmentTimingPolicy::default()
    };
    let initial_command = UpdateAssignmentTimingCommand {
        actor: instructor,
        course: ids.course,
        assignment: ids.timing_assignment,
        expected_revision: created.revision,
        policy: hidden,
    };
    if store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                actor: student,
                ..initial_command
            },
        )
        .await
        != Err(StoreError::NotFound)
    {
        bail!("student changed mutable assignment timing");
    }
    let hidden = store
        .update_assignment_timing(context, initial_command)
        .await
        .context("hiding timing acceptance assignment")?;
    if store
        .start_or_resume_run(context, student, ids.timing_assignment, ids.timing_run)
        .await
        != Err(StoreError::NotFound)
    {
        bail!("hidden assignment allowed a new student run");
    }
    let now = store
        .authoritative_time(context)
        .await
        .context("reading database clock for availability acceptance")?;
    let future = AssignmentTimingPolicy {
        available_at: Some(add_millis(now, 60_000)?),
        ..AssignmentTimingPolicy::default()
    };
    let future = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: hidden.revision,
                policy: future,
                ..initial_command
            },
        )
        .await
        .context("setting future assignment availability")?;
    if !matches!(
        store
            .start_or_resume_run(context, student, ids.timing_assignment, ids.timing_run)
            .await,
        Err(StoreError::InvalidRecord(_))
    ) {
        bail!("future assignment availability allowed an early run");
    }
    let open_now = store
        .authoritative_time(context)
        .await
        .context("reading database clock for timing acceptance")?;
    let open_policy = AssignmentTimingPolicy {
        closes_at: Some(add_millis(open_now, 60_000)?),
        ..AssignmentTimingPolicy::default()
    };
    let open = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: future.revision,
                policy: open_policy,
                ..initial_command
            },
        )
        .await
        .context("opening bounded timing acceptance assignment")?;
    if store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: future.revision,
                policy: open_policy,
                ..initial_command
            },
        )
        .await
        .context("replaying exact timing update")?
        != open
    {
        bail!("exact timing replay changed the revision or queue");
    }
    let run = store
        .start_or_resume_run(context, student, ids.timing_assignment, ids.timing_run)
        .await
        .context("starting mutable timing acceptance run")?;
    let implementation = |name: &str| ImplementationVersion {
        id: name.to_string(),
        version: "timing-acceptance-1".to_string(),
    };
    let issue = |attempt, position, seed| IssueQuestionAttemptCommand {
        actor: student,
        attempt,
        run: run.id,
        assignment_position: position,
        problem: ids.problem,
        question_version: ids.version,
        seed,
        presentation: presentation_binding(position as u8),
        parameter_hash: format!("database-timing-parameters-{position}"),
        provenance: AttemptProvenance {
            adapter: implementation("native"),
            renderer: None,
            generator: None,
            source_artifact: None,
            asset_objects: Vec::new(),
            grading: implementation("native"),
            rendered_question_sha256: format!("database-timing-render-{position}"),
        },
        prefetched: None,
        predecessor_submission: None,
    };
    let first = store
        .issue_or_resume_question_attempt(context, issue(ids.timing_attempt_one, 0, 31))
        .await
        .context("issuing shortened timing attempt")?;
    let shorten_at = store
        .authoritative_time(context)
        .await
        .context("reading shortening clock")?;
    let shortened = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: open.revision,
                policy: AssignmentTimingPolicy {
                    closes_at: Some(shorten_at),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("shortening active timing below elapsed time")?;
    assert_auto_submitted_without_work(store, context, first.id).await?;

    let second_now = store.authoritative_time(context).await?;
    let second_due = add_millis(second_now, 200)?;
    let second_policy = AssignmentTimingPolicy {
        closes_at: Some(second_due),
        ..AssignmentTimingPolicy::default()
    };
    let second_policy = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: shortened.revision,
                policy: second_policy,
                ..initial_command
            },
        )
        .await
        .context("scheduling natural auto-submit")?;
    let second = store
        .issue_or_resume_question_attempt(context, issue(ids.timing_attempt_two, 1, 32))
        .await
        .context("issuing natural auto-submit attempt")?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let due = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(30)?)
        .await
        .context("claiming natural auto-submit")?
        .ok_or_else(|| anyhow::anyhow!("natural auto-submit was not claimable"))?;
    let due_generation = auto_submit_generation(&due.payload, second.id)?;
    if store
        .commit_attempt_auto_submit(
            context,
            AttemptAutoSubmitWorkerCommand {
                job: due.id,
                lease: due.lease_token,
                attempt: second.id,
                timing_generation: due_generation,
            },
        )
        .await
        .context("committing natural auto-submit")?
        != AttemptAutoSubmitCommitOutcome::AutoSubmitted
    {
        bail!("natural deadline did not auto-submit the attempt");
    }
    assert_auto_submitted_without_work(store, context, second.id).await?;

    let third_now = store.authoritative_time(context).await?;
    let third_due = add_millis(third_now, 200)?;
    let third_policy = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: second_policy.revision,
                policy: AssignmentTimingPolicy {
                    closes_at: Some(third_due),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("scheduling leased extension acceptance")?;
    let third = store
        .issue_or_resume_question_attempt(context, issue(ids.timing_attempt_three, 2, 33))
        .await
        .context("issuing leased extension attempt")?;
    tokio::time::sleep(Duration::from_millis(250)).await;
    let stale = store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(30)?)
        .await
        .context("claiming old timing generation")?
        .ok_or_else(|| anyhow::anyhow!("old timing generation was not claimable"))?;
    let stale_generation = auto_submit_generation(&stale.payload, third.id)?;
    let extension_now = store.authoritative_time(context).await?;
    let extended = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: third_policy.revision,
                policy: AssignmentTimingPolicy {
                    closes_at: Some(add_millis(extension_now, 60_000)?),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("extending a leased timing generation")?;
    if store
        .commit_attempt_auto_submit(
            context,
            AttemptAutoSubmitWorkerCommand {
                job: stale.id,
                lease: stale.lease_token,
                attempt: third.id,
                timing_generation: stale_generation,
            },
        )
        .await
        .context("resolving leased old timing generation")?
        != AttemptAutoSubmitCommitOutcome::Rescheduled
    {
        bail!("leased old timing generation was not rescheduled");
    }
    if store
        .claim_next_job(&JobClaimFilter::all(), JobLeaseDuration::from_seconds(30)?)
        .await
        .context("checking extended deadline queue")?
        .is_some()
    {
        bail!("extended deadline remained immediately claimable");
    }
    let final_now = store.authoritative_time(context).await?;
    let closed = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: extended.revision,
                policy: AssignmentTimingPolicy {
                    closes_at: Some(final_now),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("shortening the rescheduled deadline")?;
    assert_auto_submitted_without_work(store, context, third.id).await?;

    let exception_now = store
        .authoritative_time(context)
        .await
        .context("reading exception acceptance clock")?;
    let base = store
        .update_assignment_timing(
            context,
            UpdateAssignmentTimingCommand {
                expected_revision: closed.revision,
                policy: AssignmentTimingPolicy {
                    available_at: Some(add_millis(exception_now, 60_000)?),
                    closes_at: Some(add_millis(exception_now, 60_000)?),
                    time_limit_seconds: Some(1),
                    attempt_limit: Some(1),
                    ..AssignmentTimingPolicy::default()
                },
                ..initial_command
            },
        )
        .await
        .context("setting restrictive base policy for exception acceptance")?;
    let group = store
        .put_course_group(
            context,
            PutCourseGroupCommand {
                actor: instructor,
                expected_revision: None,
                record: CourseGroupRecord {
                    id: ids.timing_group,
                    tenant: context.tenant_id(),
                    course: ids.course,
                    title: "Database extended testing".to_string(),
                    members: vec![student],
                },
            },
        )
        .await
        .context("creating timing acceptance group")?;
    let group_exception = store
        .set_assignment_policy_exception(
            context,
            SetAssignmentPolicyExceptionCommand {
                actor: instructor,
                course: ids.course,
                assignment: ids.timing_assignment,
                expected_revision: base.revision,
                exception: AssignmentPolicyException {
                    id: ids.timing_group_exception,
                    target: AssignmentPolicyExceptionTarget::CourseGroup(ids.timing_group),
                    available_at: Some(AssignmentExceptionTimestamp::Unrestricted),
                    closes_at: Some(AssignmentExceptionTimestamp::At(add_millis(
                        exception_now,
                        120_000,
                    )?)),
                    time_limit_seconds: Some(AssignmentExceptionLimit::Value(60)),
                    attempt_limit: Some(AssignmentExceptionLimit::Value(2)),
                },
            },
        )
        .await
        .context("creating group timing exception")?;
    let student_record = StudentId::from_uuid(student.as_uuid());
    let student_exception = store
        .set_assignment_policy_exception(
            context,
            SetAssignmentPolicyExceptionCommand {
                actor: instructor,
                course: ids.course,
                assignment: ids.timing_assignment,
                expected_revision: group_exception.assignment_revision,
                exception: AssignmentPolicyException {
                    id: ids.timing_student_exception,
                    target: AssignmentPolicyExceptionTarget::Student(student_record),
                    available_at: Some(AssignmentExceptionTimestamp::At(exception_now)),
                    closes_at: Some(AssignmentExceptionTimestamp::At(add_millis(
                        exception_now,
                        90_000,
                    )?)),
                    time_limit_seconds: Some(AssignmentExceptionLimit::Value(2)),
                    attempt_limit: Some(AssignmentExceptionLimit::Value(3)),
                },
            },
        )
        .await
        .context("creating direct student timing exception")?;
    let resolved = store
        .resolve_assignment_timing(context, ids.timing_assignment, student_record)
        .await
        .context("resolving combined timing exceptions")?
        .ok_or_else(|| anyhow::anyhow!("timing exception enrollment disappeared"))?;
    if resolved.policy.available_at.is_some()
        || resolved.policy.closes_at != Some(add_millis(exception_now, 120_000)?)
        || resolved.policy.time_limit_seconds != Some(60)
        || resolved.policy.attempt_limit != Some(3)
        || resolved.contributors
            != vec![
                AssignmentPolicyExceptionTarget::Student(student_record),
                AssignmentPolicyExceptionTarget::CourseGroup(ids.timing_group),
            ]
    {
        bail!("student and group timing exceptions did not resolve dimension-wise");
    }
    let exception_run = store
        .start_or_resume_run(
            context,
            student,
            ids.timing_assignment,
            ids.timing_exception_run,
        )
        .await
        .context("starting run opened by timing exception")?;
    let exception_attempt = store
        .issue_or_resume_question_attempt(
            context,
            IssueQuestionAttemptCommand {
                actor: student,
                attempt: ids.timing_exception_attempt,
                run: exception_run.id,
                assignment_position: 0,
                problem: ids.problem,
                question_version: ids.version,
                seed: 34,
                presentation: presentation_binding(34),
                parameter_hash: "database-timing-exception-parameters".to_string(),
                provenance: AttemptProvenance {
                    adapter: implementation("native"),
                    renderer: None,
                    generator: None,
                    source_artifact: None,
                    asset_objects: Vec::new(),
                    grading: implementation("native"),
                    rendered_question_sha256: "database-timing-exception-render".to_string(),
                },
                prefetched: None,
                predecessor_submission: None,
            },
        )
        .await
        .context("issuing timing exception attempt")?;
    let recorded = store
        .get_attempt_resolved_timing(context, exception_attempt.id)
        .await
        .context("reading recorded timing exception resolution")?
        .ok_or_else(|| anyhow::anyhow!("attempt timing resolution was not recorded"))?;
    if recorded.policy != resolved.policy || recorded.contributors != resolved.contributors {
        bail!("issued attempt did not record its effective timing exceptions");
    }
    tokio::time::sleep(Duration::from_millis(2_100)).await;
    store
        .upsert_course(
            context,
            CourseRecord {
                id: ids.course,
                tenant: context.tenant_id(),
                title: "PLE replica E2E course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .context("removing accommodated student from course group membership")?;
    assert_auto_submitted_without_work(store, context, exception_attempt.id).await?;
    let terminal = store
        .get_attempt_resolved_timing(context, exception_attempt.id)
        .await
        .context("reading terminal timing exception resolution")?
        .ok_or_else(|| anyhow::anyhow!("terminal attempt lost its timing resolution"))?;
    if terminal.policy.time_limit_seconds != Some(2)
        || terminal.contributors != vec![AssignmentPolicyExceptionTarget::Student(student_record)]
    {
        bail!("course membership removal did not immediately re-resolve active timing");
    }
    let after_student = store
        .delete_assignment_policy_exception(
            context,
            DeleteAssignmentPolicyExceptionCommand {
                actor: instructor,
                course: ids.course,
                assignment: ids.timing_assignment,
                expected_revision: student_exception.assignment_revision,
                exception: ids.timing_student_exception,
            },
        )
        .await
        .context("deleting direct student timing exception")?;
    store
        .delete_assignment_policy_exception(
            context,
            DeleteAssignmentPolicyExceptionCommand {
                actor: instructor,
                course: ids.course,
                assignment: ids.timing_assignment,
                expected_revision: after_student,
                exception: ids.timing_group_exception,
            },
        )
        .await
        .context("deleting group timing exception")?;
    if store
        .get_course_group(context, ids.timing_group)
        .await
        .context("reading timing group after course membership removal")?
        .is_none_or(|stored| stored.revision != group.revision || !stored.record.members.is_empty())
    {
        bail!("course membership removal did not preserve an empty revisioned group");
    }
    Ok(())
}

pub(super) fn add_millis(value: ActivityTimestamp, millis: i64) -> Result<ActivityTimestamp> {
    Ok(ActivityTimestamp::from_unix_millis(
        value
            .as_unix_millis()
            .checked_add(millis)
            .ok_or_else(|| anyhow::anyhow!("timing acceptance timestamp overflow"))?,
    ))
}

pub(super) fn auto_submit_generation(
    payload: &JobPayload,
    attempt: QuestionAttemptId,
) -> Result<u64> {
    match payload {
        JobPayload::AutoSubmitAttempt {
            attempt: queued,
            timing_generation,
        } if *queued == attempt => Ok(*timing_generation),
        _ => bail!("timing acceptance claimed another job family or attempt"),
    }
}

pub(super) async fn assert_auto_submitted_without_work(
    store: &learning_data_access::postgres::PostgresStore,
    context: TenantContext,
    attempt: QuestionAttemptId,
) -> Result<()> {
    let current = store
        .get_question_attempt(context, attempt)
        .await
        .context("reading auto-submitted attempt")?
        .ok_or_else(|| anyhow::anyhow!("auto-submitted attempt disappeared"))?;
    if current.status != AttemptStatus::AutoSubmitted
        || current.response.is_some()
        || current.result.is_some()
        || current.timer.submitted_at.is_none()
    {
        bail!("auto-submit fabricated a response or grade, or did not close the attempt");
    }
    Ok(())
}
