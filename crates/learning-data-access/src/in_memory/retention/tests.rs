//! Focused retention capability tests.

use super::*;

#[cfg(test)]
mod retention_tests {
    use super::*;
    use crate::{
        JobClaimFilter, JobLeaseDuration, JobPayload, JobStore, RetentionApiStore, RetentionDays,
        RetentionDispatchBatch, RetentionScheduleStore, RetentionWorkerCommand,
        RetentionWorkerStore, SessionLifetime, SessionStore,
    };
    use question_model::{CourseMembership, CourseMembershipRole};

    fn session(number: u8) -> SessionTokenHash {
        SessionTokenHash::compute(&[number; 32])
    }

    async fn establish_session(
        store: &MemoryStore,
        token: SessionTokenHash,
        tenant: TenantId,
        user: UserId,
        roles: Vec<UserRole>,
    ) {
        store
            .create_session(
                token,
                SessionSubject::new(tenant, user, "Retention fixture", roles)
                    .expect("valid session subject"),
                SessionLifetime::from_seconds(3_600).expect("valid lifetime"),
            )
            .await
            .expect("store session");
    }

    #[tokio::test]
    async fn retention_policy_and_course_end_are_session_authorized_and_idempotent() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(81_001));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(Uuid::from_u128(81_002));
        let student = UserId::from_uuid(Uuid::from_u128(81_003));
        let administrator = UserId::from_uuid(Uuid::from_u128(81_004));
        let course = CourseId::from_uuid(Uuid::from_u128(81_005));
        {
            let mut state = store.write_state().expect("retention state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(1_000_000);
            state.courses.insert(
                (tenant, course),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Retention course".to_string(),
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
            );
        }
        establish_session(
            &store,
            session(1),
            tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        establish_session(&store, session(2), tenant, student, vec![UserRole::Student]).await;
        establish_session(
            &store,
            session(3),
            tenant,
            administrator,
            vec![UserRole::Administrator],
        )
        .await;

        assert_eq!(
            store
                .configure_retention_policy(
                    context,
                    session(1),
                    InstitutionRetentionPolicy::default()
                )
                .await,
            Err(StoreError::Forbidden)
        );
        let custom = InstitutionRetentionPolicy::new(
            RetentionDays::new(31).unwrap(),
            RetentionDays::new(101).unwrap(),
            RetentionDays::new(366).unwrap(),
        )
        .unwrap();
        store
            .configure_retention_policy(context, session(3), custom)
            .await
            .expect("admin policy");
        let first = store
            .end_course_retention(context, session(1), course)
            .await
            .expect("instructor ends course");
        assert_eq!(
            first.snapshot.ended_at(),
            ActivityTimestamp::from_unix_millis(1_000_000)
        );
        assert_eq!(first.snapshot.policy(), custom);
        assert_eq!(first.snapshot.generation(), 1);
        assert_eq!(first.status.state, CourseRetentionState::Active);
        assert_eq!(
            store
                .end_course_retention(context, session(1), course)
                .await
                .expect("exact replay"),
            first
        );
        assert_eq!(
            store.course_retention(context, session(2), course).await,
            Ok(None)
        );
        assert_eq!(
            store
                .course_retention(context, session(3), course)
                .await
                .expect("admin view"),
            Some(first)
        );
    }

    #[tokio::test]
    async fn scheduler_dispatches_each_due_current_stage_once_and_binds_worker_execution() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(81_100));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(Uuid::from_u128(81_101));
        let administrator = UserId::from_uuid(Uuid::from_u128(81_102));
        let course = CourseId::from_uuid(Uuid::from_u128(81_103));
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(1_000_000);
            state.courses.insert(
                (tenant, course),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Dispatch course".to_string(),
                    members: vec![CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    }],
                },
            );
        }
        establish_session(
            &store,
            session(10),
            tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        establish_session(
            &store,
            session(11),
            tenant,
            administrator,
            vec![UserRole::Administrator],
        )
        .await;
        let policy = InstitutionRetentionPolicy::new(
            RetentionDays::new(2).unwrap(),
            RetentionDays::new(4).unwrap(),
            RetentionDays::new(6).unwrap(),
        )
        .unwrap();
        store
            .configure_retention_policy(context, session(11), policy)
            .await
            .expect("admin policy");
        let record = store
            .end_course_retention(context, session(10), course)
            .await
            .expect("course end");
        let batch = RetentionDispatchBatch::new(3).expect("batch");
        for stage in [
            crate::RetentionStage::Notify,
            crate::RetentionStage::ArchiveStudentRecords,
            crate::RetentionStage::DeleteStudentRecords,
        ] {
            let due = record
                .snapshot
                .policy()
                .due_at(record.snapshot.ended_at(), stage)
                .unwrap();
            {
                let mut state = store.write_state().expect("state");
                state.authoritative_time =
                    ActivityTimestamp::from_unix_millis(due.as_unix_millis() - 1);
            }
            assert_eq!(store.dispatch_due_retention_stages(batch).await, Ok(0));
            {
                let mut state = store.write_state().expect("state");
                state.authoritative_time = due;
            }
            assert_eq!(store.dispatch_due_retention_stages(batch).await, Ok(1));
            assert_eq!(store.dispatch_due_retention_stages(batch).await, Ok(0));
            let claimed = store
                .claim_next_job(
                    &JobClaimFilter::all(),
                    JobLeaseDuration::from_seconds(30).unwrap(),
                )
                .await
                .expect("claim")
                .expect("bound job");
            assert_eq!(
                claimed.payload,
                JobPayload::Retention {
                    course,
                    stage,
                    generation: 1,
                }
            );
            let command = RetentionWorkerCommand {
                tenant,
                course,
                stage,
                generation: 1,
                job: claimed.id,
                lease: claimed.lease_token,
            };
            store
                .prepare_retention_work(command)
                .await
                .expect("bound preparation");
            store
                .commit_retention_work(command)
                .await
                .expect("exact completion");
        }
        // A valid-looking but unbound job cannot execute under R3's worker API.
        let forged = store
            .enqueue_job(
                context,
                crate::EnqueueJob {
                    tenant,
                    payload: JobPayload::Retention {
                        course,
                        stage: crate::RetentionStage::Notify,
                        generation: 1,
                    },
                    max_attempts: 1,
                },
            )
            .await
            .expect("raw-looking job is queue-valid but not retention-bound");
        let claimed = store
            .claim_next_job(
                &JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).unwrap(),
            )
            .await
            .expect("claim forged")
            .expect("forged job");
        assert_eq!(claimed.id, forged);
        assert_eq!(
            store
                .prepare_retention_work(RetentionWorkerCommand {
                    tenant,
                    course,
                    stage: crate::RetentionStage::Notify,
                    generation: 1,
                    job: forged,
                    lease: claimed.lease_token,
                })
                .await,
            Err(StoreError::Conflict)
        );

        // The same trusted scheduler path honors an institution without an
        // explicit policy row: R1's 30-day default is still a real dispatch
        // deadline, not only a pure-policy calculation.
        let default_tenant = TenantId::from_uuid(Uuid::from_u128(81_104));
        let default_context = TenantContext::from_authenticated_session(default_tenant);
        let default_course = CourseId::from_uuid(Uuid::from_u128(81_105));
        {
            let mut state = store.write_state().expect("state");
            state.courses.insert(
                (default_tenant, default_course),
                CourseRecord {
                    id: default_course,
                    tenant: default_tenant,
                    title: "Default dispatch course".to_string(),
                    members: vec![CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    }],
                },
            );
        }
        establish_session(
            &store,
            session(12),
            default_tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        let default_record = store
            .end_course_retention(default_context, session(12), default_course)
            .await
            .expect("default-policy end");
        let default_due = default_record
            .snapshot
            .policy()
            .due_at(
                default_record.snapshot.ended_at(),
                crate::RetentionStage::Notify,
            )
            .expect("default due");
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = default_due;
        }
        assert_eq!(store.dispatch_due_retention_stages(batch).await, Ok(1));
        let default_job = store
            .claim_next_job(
                &JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).unwrap(),
            )
            .await
            .expect("claim default")
            .expect("default job");
        assert_eq!(
            default_job.payload,
            JobPayload::Retention {
                course: default_course,
                stage: crate::RetentionStage::Notify,
                generation: 1,
            }
        );
    }

    #[tokio::test]
    async fn extension_and_disposition_are_authorized_and_generation_fenced() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(81_200));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(Uuid::from_u128(81_201));
        let student = UserId::from_uuid(Uuid::from_u128(81_202));
        let administrator = UserId::from_uuid(Uuid::from_u128(81_203));
        let course = CourseId::from_uuid(Uuid::from_u128(81_204));
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(2_000_000);
            state.courses.insert(
                (tenant, course),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Extension course".to_string(),
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
            );
        }
        establish_session(
            &store,
            session(20),
            tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        establish_session(
            &store,
            session(21),
            tenant,
            student,
            vec![UserRole::Student],
        )
        .await;
        establish_session(
            &store,
            session(22),
            tenant,
            administrator,
            vec![UserRole::Administrator],
        )
        .await;
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(22),
                    course,
                    RetentionDays::new(1).unwrap(),
                )
                .await,
            Err(StoreError::Conflict),
            "an existing but unended course has no schedule to extend"
        );
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(22),
                    CourseId::from_uuid(Uuid::from_u128(81_299)),
                    RetentionDays::new(1).unwrap(),
                )
                .await,
            Err(StoreError::Forbidden),
            "missing courses remain nonenumerating"
        );
        let original = store
            .end_course_retention(context, session(20), course)
            .await
            .expect("end");
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(20),
                    course,
                    RetentionDays::new(1).unwrap()
                )
                .await,
            Err(StoreError::Forbidden)
        );
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(21),
                    course,
                    RetentionDays::new(1).unwrap()
                )
                .await,
            Err(StoreError::Forbidden)
        );
        let chosen = store
            .set_archive_disposition(
                context,
                session(20),
                course,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("instructor disposition");
        assert_eq!(
            chosen.status.assignment_definitions,
            AssignmentDefinitionDisposition::Delete
        );
        // A completed notification is historical: an extension copies it without
        // shifting or redelivering it, while future stages move to the new
        // generation.
        {
            let mut state = store.write_state().expect("state");
            let notify_key = (tenant, course, crate::RetentionStage::Notify, 1);
            let stored = state.retention_stages[&notify_key];
            state.retention_stages.insert(
                notify_key,
                StoredRetentionStage {
                    state: RetentionStageWorkState::Completed,
                    ..stored
                },
            );
            let notification_created_at = state.authoritative_time;
            state.retention_notifications.insert(
                (tenant, course, 1),
                crate::RetentionNotificationView {
                    intent: crate::RetentionNotificationIntent::Archive,
                    created_at: notification_created_at,
                },
            );
            // A scheduler may have handed a still-unstarted future stage to a
            // worker. Extension fences that lease by killing its exact dispatch
            // job before the generation changes.
            let leased_job = crate::JobId::from_uuid(Uuid::from_u128(81_205));
            let now = state.authoritative_time;
            state.jobs.insert(
                leased_job,
                StoredJob {
                    tenant,
                    payload: JobPayload::Retention {
                        course,
                        stage: crate::RetentionStage::ArchiveStudentRecords,
                        generation: 1,
                    },
                    state: JobState::Leased,
                    available_at: now,
                    lease_token: Some(JobLeaseToken::generate().expect("lease")),
                    lease_expires_at: Some(ActivityTimestamp::from_unix_millis(
                        now.as_unix_millis() + 10_000,
                    )),
                    attempt_count: 1,
                    max_attempts: RETENTION_JOB_MAX_ATTEMPTS,
                    failure: None,
                },
            );
            state.retention_dispatches.insert(
                (
                    tenant,
                    course,
                    crate::RetentionStage::ArchiveStudentRecords,
                    1,
                ),
                leased_job,
            );
        }
        let extended = store
            .extend_course_retention(context, session(22), course, RetentionDays::new(7).unwrap())
            .await
            .expect("admin extension");
        assert_eq!(extended.snapshot.generation(), 2);
        assert_eq!(
            extended.status.assignment_definitions,
            AssignmentDefinitionDisposition::Delete
        );
        let latest_notification = store
            .retention_notification(context, session(20), course)
            .await
            .expect("authorized notification read")
            .expect("completed notification remains readable after extension");
        assert_eq!(
            latest_notification.intent,
            crate::RetentionNotificationIntent::Archive
        );
        assert_eq!(
            latest_notification.created_at,
            ActivityTimestamp::from_unix_millis(2_000_000)
        );
        {
            let state = store.read_state().expect("state");
            for stage in [
                crate::RetentionStage::Notify,
                crate::RetentionStage::ArchiveStudentRecords,
                crate::RetentionStage::DeleteStudentRecords,
            ] {
                let old = state.retention_stages[&(tenant, course, stage, 1)];
                let new = state.retention_stages[&(tenant, course, stage, 2)];
                if stage == crate::RetentionStage::Notify {
                    assert_eq!(old.state, RetentionStageWorkState::Completed);
                    assert_eq!(new.state, RetentionStageWorkState::Completed);
                    assert_eq!(new.due_at, old.due_at);
                } else {
                    assert_eq!(old.state, RetentionStageWorkState::Superseded);
                    assert_eq!(
                        new.due_at.as_unix_millis(),
                        old.due_at.as_unix_millis() + 7 * 86_400_000
                    );
                }
            }
            assert_eq!(original.snapshot.ended_at(), extended.snapshot.ended_at());
            assert_eq!(
                state.jobs[&crate::JobId::from_uuid(Uuid::from_u128(81_205))].state,
                JobState::Dead,
                "extension must revoke a leased but unstarted dispatched stage"
            );
        }

        // The archive-time disposition freezes as soon as its own stage starts;
        // an administrator also cannot extend an in-progress generation.
        {
            let mut state = store.write_state().expect("state");
            let archive_key = (
                tenant,
                course,
                crate::RetentionStage::ArchiveStudentRecords,
                2,
            );
            let stored = state.retention_stages[&archive_key];
            state.retention_stages.insert(
                archive_key,
                StoredRetentionStage {
                    state: RetentionStageWorkState::Started,
                    ..stored
                },
            );
        }
        assert_eq!(
            store
                .set_archive_disposition(
                    context,
                    session(20),
                    course,
                    AssignmentDefinitionDisposition::Retain,
                )
                .await,
            Err(StoreError::Conflict)
        );
        assert_eq!(
            store
                .extend_course_retention(
                    context,
                    session(22),
                    course,
                    RetentionDays::new(1).unwrap(),
                )
                .await,
            Err(StoreError::Conflict)
        );
    }

    #[tokio::test]
    async fn retention_api_uses_safe_revision_cas_and_closed_manual_dispatch() {
        let store = MemoryStore::default();
        let tenant = TenantId::from_uuid(Uuid::from_u128(81_300));
        let context = TenantContext::from_authenticated_session(tenant);
        let instructor = UserId::from_uuid(Uuid::from_u128(81_301));
        let course = CourseId::from_uuid(Uuid::from_u128(81_302));
        let other_course = CourseId::from_uuid(Uuid::from_u128(81_303));
        {
            let mut state = store.write_state().expect("state");
            state.authoritative_time = ActivityTimestamp::from_unix_millis(3_000_000);
            state.courses.insert(
                (tenant, course),
                CourseRecord {
                    id: course,
                    tenant,
                    title: "Retention API course".to_string(),
                    members: vec![CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    }],
                },
            );
            state.courses.insert(
                (tenant, other_course),
                CourseRecord {
                    id: other_course,
                    tenant,
                    title: "Other retention API course".to_string(),
                    members: vec![CourseMembership {
                        user: instructor,
                        role: CourseMembershipRole::Instructor,
                    }],
                },
            );
        }
        establish_session(
            &store,
            session(30),
            tenant,
            instructor,
            vec![UserRole::Instructor],
        )
        .await;
        store
            .end_course_retention(context, session(30), course)
            .await
            .expect("end course");
        let view = store
            .retention_view(context, session(30), course)
            .await
            .expect("safe view")
            .expect("ended view");
        assert_eq!(view.revision.value(), 1);
        let queued = store
            .request_retention_archive_if_revision(
                context,
                session(30),
                course,
                view.revision,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("queue archive");
        assert_eq!(queued.outcome, crate::RetentionRequestOutcome::Scheduled);
        assert_eq!(queued.retention.revision.value(), 2);
        assert_eq!(
            queued.retention.assignment_definitions,
            AssignmentDefinitionDisposition::Delete
        );
        // The other course deliberately shares actor and resulting generation
        // but differs in disposition. Its receipt cannot authorize a replay
        // for this course or enqueue another job here.
        store
            .end_course_retention(context, session(30), other_course)
            .await
            .expect("end other course");
        let other_view = store
            .retention_view(context, session(30), other_course)
            .await
            .expect("other safe view")
            .expect("other ended view");
        store
            .request_retention_archive_if_revision(
                context,
                session(30),
                other_course,
                other_view.revision,
                AssignmentDefinitionDisposition::Retain,
            )
            .await
            .expect("queue other archive");
        let jobs_before_cross_course_replay = store.read_state().expect("state").jobs.len();
        assert_eq!(
            store
                .request_retention_archive_if_revision(
                    context,
                    session(30),
                    course,
                    queued.retention.revision,
                    AssignmentDefinitionDisposition::Retain,
                )
                .await,
            Err(StoreError::Conflict),
            "another course receipt must not authorize this course replay"
        );
        assert_eq!(
            store.read_state().expect("state").jobs.len(),
            jobs_before_cross_course_replay,
            "cross-course receipt confusion must not enqueue work"
        );
        // The rest of this established single-course worker test claims the
        // only ready job. Remove the independent adversarial fixture after
        // its assertion rather than making later worker assertions order-aware.
        {
            let mut state = store.write_state().expect("state");
            let other_job = state
                .retention_dispatches
                .remove(&(
                    tenant,
                    other_course,
                    crate::RetentionStage::ArchiveStudentRecords,
                    2,
                ))
                .expect("other archive dispatch");
            state.jobs.remove(&other_job);
        }
        let scheduled_replay = store
            .request_retention_archive_if_revision(
                context,
                session(30),
                course,
                view.revision,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("exact scheduled replay");
        assert_eq!(
            scheduled_replay.outcome,
            crate::RetentionRequestOutcome::Scheduled
        );
        assert_eq!(scheduled_replay.retention.revision.value(), 2);
        let current_revision_replay = store
            .request_retention_archive_if_revision(
                context,
                session(30),
                course,
                queued.retention.revision,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("current-revision scheduled replay");
        assert_eq!(
            current_revision_replay.outcome,
            crate::RetentionRequestOutcome::Scheduled
        );
        assert_eq!(current_revision_replay.retention.revision.value(), 2);
        assert_eq!(
            store
                .read_state()
                .expect("state")
                .jobs
                .values()
                .filter(|job| matches!(job.payload, JobPayload::Retention { course: candidate, stage: crate::RetentionStage::ArchiveStudentRecords, generation: 2 } if candidate == course))
                .count(),
            1,
            "exact scheduled replay creates no second job"
        );
        assert_eq!(
            store
                .request_retention_archive_if_revision(
                    context,
                    session(30),
                    course,
                    view.revision,
                    AssignmentDefinitionDisposition::Retain,
                )
                .await,
            Err(StoreError::Conflict),
            "a replay cannot silently change the requested disposition"
        );
        assert_eq!(
            store
                .request_retention_delete_if_revision(context, session(30), course, view.revision)
                .await,
            Err(StoreError::Conflict),
            "stale tabs cannot replace a queued request"
        );
        let job = store
            .claim_next_job(
                &JobClaimFilter::all(),
                JobLeaseDuration::from_seconds(30).unwrap(),
            )
            .await
            .expect("claim")
            .expect("bound archive work");
        assert_eq!(
            job.payload,
            JobPayload::Retention {
                course,
                stage: crate::RetentionStage::ArchiveStudentRecords,
                generation: 2,
            }
        );
        let command = RetentionWorkerCommand {
            tenant,
            course,
            stage: crate::RetentionStage::ArchiveStudentRecords,
            generation: 2,
            job: job.id,
            lease: job.lease_token,
        };
        store
            .prepare_retention_work(command)
            .await
            .expect("start archive");
        let in_progress = store
            .request_retention_archive_if_revision(
                context,
                session(30),
                course,
                view.revision,
                AssignmentDefinitionDisposition::Delete,
            )
            .await
            .expect("exact in-progress replay");
        assert_eq!(
            in_progress.outcome,
            crate::RetentionRequestOutcome::InProgress
        );
        assert_eq!(in_progress.retention.revision, queued.retention.revision);
        store
            .commit_retention_work(command)
            .await
            .expect("complete archive");
        assert_eq!(
            store
                .request_retention_archive_if_revision(
                    context,
                    session(30),
                    course,
                    view.revision,
                    AssignmentDefinitionDisposition::Delete,
                )
                .await
                .expect("exact completed replay")
                .outcome,
            crate::RetentionRequestOutcome::Completed
        );
    }
}
