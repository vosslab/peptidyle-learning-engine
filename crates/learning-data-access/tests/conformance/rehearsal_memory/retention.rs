//! Retention integration evidence for the isolated rehearsal aggregate.

use super::*;
use learning_data_access::{
    AssignmentDefinitionDisposition, JobKind, RetentionApiStore, RetentionDispatchBatch,
    RetentionScheduleStore, RetentionStage, RetentionStore, RetentionWorkerCommand,
    RetentionWorkerStore,
};

fn retention_session() -> SessionTokenHash {
    SessionTokenHash::compute(&[0xD4; 32])
}

async fn establish_retention_session(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
) {
    store
        .create_session(
            retention_session(),
            SessionSubject::new(
                fixture.context.tenant_id(),
                fixture.instructor,
                "Retention rehearsal instructor",
                vec![UserRole::Instructor],
            )
            .expect("valid instructor session"),
            SessionLifetime::from_seconds(3_600).expect("valid session lifetime"),
        )
        .await
        .expect("store instructor session");
}

async fn commit_retention_stage(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    stage: RetentionStage,
    generation: u64,
) {
    let claimed = store
        .claim_next_job(
            &JobClaimFilter::new([JobKind::Retention]).expect("retention job filter"),
            JobLeaseDuration::from_seconds(30).expect("valid lease"),
        )
        .await
        .expect("claim retention job")
        .expect("retention job");
    assert_eq!(
        claimed.payload,
        JobPayload::Retention {
            course: fixture.course,
            stage,
            generation,
        }
    );
    let command = RetentionWorkerCommand {
        tenant: fixture.context.tenant_id(),
        course: fixture.course,
        stage,
        generation,
        job: claimed.id,
        lease: claimed.lease_token,
    };
    store
        .prepare_retention_work(command)
        .await
        .expect("prepare retention work");
    store
        .commit_retention_work(command)
        .await
        .expect("commit retention work");
}

async fn commit_due_retention_stage(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    expected_stage: RetentionStage,
    generation: u64,
) {
    store
        .dispatch_due_retention_stages(RetentionDispatchBatch::new(3).expect("valid batch"))
        .await
        .expect("dispatch due retention stages");
    for _ in 0..3 {
        let claimed = store
            .claim_next_job(
                &JobClaimFilter::new([JobKind::Retention]).expect("retention job filter"),
                JobLeaseDuration::from_seconds(30).expect("valid lease"),
            )
            .await
            .expect("claim due retention job")
            .expect("due retention job");
        let JobPayload::Retention {
            course,
            stage,
            generation: claimed_generation,
        } = claimed.payload
        else {
            panic!("retention fixture must dispatch only retention jobs");
        };
        assert_eq!(course, fixture.course);
        assert_eq!(claimed_generation, generation);
        let command = RetentionWorkerCommand {
            tenant: fixture.context.tenant_id(),
            course,
            stage,
            generation: claimed_generation,
            job: claimed.id,
            lease: claimed.lease_token,
        };
        store
            .prepare_retention_work(command)
            .await
            .expect("prepare due retention work");
        store
            .commit_retention_work(command)
            .await
            .expect("commit due retention work");
        if stage == expected_stage {
            return;
        }
    }
    panic!("expected retention stage was not dispatched");
}

#[cfg(feature = "test-support")]
async fn prepare_due_retention_stage(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    expected_stage: RetentionStage,
    generation: u64,
) -> RetentionWorkerCommand {
    store
        .dispatch_due_retention_stages(RetentionDispatchBatch::new(3).expect("valid batch"))
        .await
        .expect("dispatch due retention stages");
    for _ in 0..3 {
        let claimed = store
            .claim_next_job(
                &JobClaimFilter::new([JobKind::Retention]).expect("retention job filter"),
                JobLeaseDuration::from_seconds(30).expect("valid lease"),
            )
            .await
            .expect("claim due retention job")
            .expect("due retention job");
        let JobPayload::Retention {
            course,
            stage,
            generation: claimed_generation,
        } = claimed.payload
        else {
            panic!("retention fixture must dispatch only retention jobs");
        };
        assert_eq!(course, fixture.course);
        assert_eq!(claimed_generation, generation);
        let command = RetentionWorkerCommand {
            tenant: fixture.context.tenant_id(),
            course,
            stage,
            generation: claimed_generation,
            job: claimed.id,
            lease: claimed.lease_token,
        };
        store
            .prepare_retention_work(command)
            .await
            .expect("prepare due retention work");
        if stage == expected_stage {
            return command;
        }
        store
            .commit_retention_work(command)
            .await
            .expect("commit preceding due retention work");
    }
    panic!("expected retention stage was not dispatched");
}

async fn delete_student_records(
    store: &MemoryStore,
    fixture: &effective_policy::EffectivePolicyFixture,
    disposition: AssignmentDefinitionDisposition,
) {
    store
        .end_course_retention(fixture.context, retention_session(), fixture.course)
        .await
        .expect("end course retention");
    let retention = store
        .retention_view(fixture.context, retention_session(), fixture.course)
        .await
        .expect("retention view")
        .expect("course retention");
    let archive = store
        .request_retention_archive_if_revision(
            fixture.context,
            retention_session(),
            fixture.course,
            retention.revision,
            disposition,
        )
        .await
        .expect("request archive");
    commit_retention_stage(
        store,
        fixture,
        RetentionStage::ArchiveStudentRecords,
        archive.retention.revision.value(),
    )
    .await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(9_999_999_999_999))
        .expect("advance retention clock");
    commit_due_retention_stage(
        store,
        fixture,
        RetentionStage::DeleteStudentRecords,
        archive.retention.revision.value(),
    )
    .await;
}

/// Student-record deletion with `Retain` must preserve the active rehearsal's
/// live direct-Instructor authorization.  This runs the actual retention job
/// rather than calling the private source-fence helper.
#[tokio::test]
async fn retain_student_record_deletion_keeps_active_rehearsal_resumable() {
    let store = MemoryStore::default();
    let (fixture, locator, _) = start_and_freeze(&store).await;
    establish_retention_session(&store, &fixture).await;
    let rehearsal_before = store
        .read_rehearsal(fixture.context, locator)
        .await
        .expect("active rehearsal before retention");

    delete_student_records(&store, &fixture, AssignmentDefinitionDisposition::Retain).await;

    assert_eq!(
        store.read_rehearsal(fixture.context, locator).await,
        Ok(rehearsal_before),
        "student-only retention must leave the active identity-free rehearsal resumable"
    );
}

#[cfg(feature = "test-support")]
async fn claim_and_complete(
    store: &MemoryStore,
    context: TenantContext,
    locator: RehearsalLocator,
    frozen: &RehearsalFrozenItemEvidence,
    key: &str,
) {
    let claimed = store
        .claim_rehearsal_submission(
            context,
            ClaimRehearsalSubmissionCommand {
                locator,
                attempt: frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: RehearsalSubmissionIdempotencyKey::new(key.into())
                    .expect("valid idempotency key"),
            },
        )
        .await
        .expect("claim completion fixture");
    let RehearsalSubmissionClaimResult::Claimed(claimed) = claimed else {
        panic!("completion fixture must create a claim");
    };
    let dispatched = store
        .mark_rehearsal_submission_dispatched(
            context,
            MarkRehearsalSubmissionDispatchedCommand {
                locator,
                handle: claimed.handle,
            },
        )
        .await
        .expect("dispatch completion fixture");
    store
        .complete_rehearsal_submission(
            context,
            CompleteRehearsalSubmissionCommand {
                locator,
                handle: dispatched,
                grading: deterministic_grade(),
            },
        )
        .await
        .expect("complete fixture");
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn deleting_assignment_sources_fences_matching_active_runs_and_preserves_completed_evidence()
{
    let store = MemoryStore::default();
    let (fixture, first_locator, first_frozen) = start_and_freeze(&store).await;
    establish_retention_session(&store, &fixture).await;
    claim_and_complete(
        &store,
        fixture.context,
        first_locator,
        &first_frozen,
        "retention-completed",
    )
    .await;

    let second_assignment = AssignmentId::from_uuid(uuid(99_012));
    let second_stored = store
        .get_assignment_for_edit(fixture.context, second_assignment)
        .await
        .expect("second assignment read")
        .expect("second assignment");
    let second_policy = store
        .get_base_assignment_policy(fixture.context, second_assignment)
        .await
        .expect("second assignment policy read")
        .expect("second assignment policy");
    store
        .put_assignment_teaching_settings(
            fixture.context,
            PutAssignmentTeachingSettingsCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: second_assignment,
                expected_revision: second_policy.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: second_stored.record.instructions.clone(),
                    base_policy: second_policy.policy,
                },
            },
        )
        .await
        .expect("publish second rehearsal assignment");
    let second_reference = store
        .assignment_reference(fixture.context, fixture.instructor, second_assignment)
        .await
        .expect("second assignment lookup")
        .expect("second assignment reference");
    let second_revision = TeachingOperationRevision::new(
        store
            .get_assignment_for_edit(fixture.context, second_assignment)
            .await
            .expect("second assignment read")
            .expect("second assignment")
            .revision
            .value(),
    )
    .expect("second teaching revision");
    let second_receipt = store
        .start_rehearsal(
            fixture.context,
            StartRehearsalCommand {
                actor: fixture.instructor,
                course: fixture.course,
                assignment: second_reference,
                revision: second_revision,
                subject: synthetic_start(),
                start_new_after_completion: false,
            },
        )
        .await
        .expect("second active rehearsal");
    let second_locator = RehearsalLocator {
        actor: fixture.instructor,
        course: fixture.course,
        assignment: second_reference,
        revision: second_revision,
        rehearsal: second_receipt.rehearsal,
    };
    let prepared_frozen = RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(uuid(990_100)),
        ..first_frozen.clone()
    };
    let dispatched_frozen = RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(uuid(990_101)),
        ..first_frozen.clone()
    };
    for frozen in [&prepared_frozen, &dispatched_frozen] {
        store
            .append_rehearsal_frozen_item(
                fixture.context,
                AppendRehearsalFrozenItemCommand {
                    locator: second_locator,
                    frozen: frozen.clone(),
                },
            )
            .await
            .expect("freeze second rehearsal item");
    }
    let prepared = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator: second_locator,
                attempt: prepared_frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: RehearsalSubmissionIdempotencyKey::new(
                    "retention-prepared".into(),
                )
                .expect("prepared key"),
            },
        )
        .await
        .expect("prepared claim");
    assert!(matches!(
        prepared,
        RehearsalSubmissionClaimResult::Claimed(_)
    ));
    let dispatched = store
        .claim_rehearsal_submission(
            fixture.context,
            ClaimRehearsalSubmissionCommand {
                locator: second_locator,
                attempt: dispatched_frozen.attempt,
                response: StudentResponse::Numeric { value: 3.0 },
                idempotency_key: RehearsalSubmissionIdempotencyKey::new(
                    "retention-dispatched".into(),
                )
                .expect("dispatched key"),
            },
        )
        .await
        .expect("dispatched claim");
    let RehearsalSubmissionClaimResult::Claimed(dispatched) = dispatched else {
        panic!("dispatched fixture must create a claim");
    };
    store
        .mark_rehearsal_submission_dispatched(
            fixture.context,
            MarkRehearsalSubmissionDispatchedCommand {
                locator: second_locator,
                handle: dispatched.handle,
            },
        )
        .await
        .expect("mark dispatched");

    delete_student_records(&store, &fixture, AssignmentDefinitionDisposition::Delete).await;

    for locator in [first_locator, second_locator] {
        assert!(
            store
                .read_rehearsal(fixture.context, locator)
                .await
                .is_err(),
            "normal rehearsal reads conceal the fenced archive"
        );
    }
    for reference in [first_locator.rehearsal, second_locator.rehearsal] {
        store
            .verify_rehearsal_archive_for_test(fixture.context.tenant_id(), reference)
            .expect("retained archive remains independently verifiable");
    }
    let first_snapshot = store
        .rehearsal_test_snapshot(fixture.context.tenant_id(), first_locator.rehearsal)
        .expect("retained completed archive snapshot");
    assert_eq!(first_snapshot.lifecycle, RehearsalLifecycle::Completed);
    assert_eq!(
        first_snapshot.claims[0].phase,
        domain::RehearsalSubmissionClaimPhase::Completed,
        "completed evidence and receipt stay intact"
    );
    let second_snapshot = store
        .rehearsal_test_snapshot(fixture.context.tenant_id(), second_locator.rehearsal)
        .expect("retained pending archive snapshot");
    assert_eq!(
        second_snapshot.lifecycle,
        RehearsalLifecycle::DiscardedSourceContextRemoved
    );
    assert_eq!(second_snapshot.claims.len(), 2);
    assert!(second_snapshot.claims.iter().all(|claim| {
        claim.phase == domain::RehearsalSubmissionClaimPhase::RevokedSourceContextRemoved
    }));
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn corrupt_rehearsal_aborts_the_prepared_source_deletion_without_partial_retention_effects() {
    let store = MemoryStore::default();
    let (fixture, locator, frozen) = start_and_freeze(&store).await;
    establish_retention_session(&store, &fixture).await;
    store
        .end_course_retention(fixture.context, retention_session(), fixture.course)
        .await
        .expect("end course retention");
    let retention = store
        .retention_view(fixture.context, retention_session(), fixture.course)
        .await
        .expect("retention view")
        .expect("course retention");
    let archive = store
        .request_retention_archive_if_revision(
            fixture.context,
            retention_session(),
            fixture.course,
            retention.revision,
            AssignmentDefinitionDisposition::Delete,
        )
        .await
        .expect("request delete archive");
    commit_retention_stage(
        &store,
        &fixture,
        RetentionStage::ArchiveStudentRecords,
        archive.retention.revision.value(),
    )
    .await;
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(9_999_999_999_999))
        .expect("advance retention clock");
    let command = prepare_due_retention_stage(
        &store,
        &fixture,
        RetentionStage::DeleteStudentRecords,
        archive.retention.revision.value(),
    )
    .await;
    store
        .corrupt_rehearsal_integrity_for_test(
            MemoryRehearsalIntegrityTestCorruption::RemoveFrozenItem {
                tenant: fixture.context.tenant_id(),
                rehearsal: locator.rehearsal,
                attempt: frozen.attempt,
            },
        )
        .expect("corrupt rehearsal aggregate");
    let before = store
        .rehearsal_state_effect_fingerprint()
        .expect("before failure");
    assert!(store.commit_retention_work(command).await.is_err());
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after failure")
            .is_unchanged_from(&before),
        "a corrupt aggregate aborts the complete staged retention transaction"
    );
    assert!(
        store
            .get_assignment_for_edit(fixture.context, fixture.assignment)
            .await
            .expect("assignment read")
            .is_some(),
        "failed fence leaves source assignment intact"
    );
    assert_eq!(
        store
            .rehearsal_test_snapshot(fixture.context.tenant_id(), locator.rehearsal)
            .expect("snapshot after failed fence")
            .lifecycle,
        RehearsalLifecycle::Active
    );
}

#[cfg(feature = "test-support")]
#[tokio::test]
async fn tenant_a_source_deletion_never_fences_a_same_shaped_foreign_rehearsal() {
    let store = MemoryStore::default();
    let (fixture, target_locator, _) = start_and_freeze(&store).await;
    establish_retention_session(&store, &fixture).await;

    let foreign_tenant = TenantId::from_uuid(uuid(990_000));
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let foreign_instructor = UserId::from_uuid(uuid(990_001));
    let foreign_course = CourseId::from_uuid(uuid(990_002));
    let foreign_course_creation_authority = sysadmin_course_creation_authority(
        &store,
        foreign_tenant,
        foreign_course,
        foreign_instructor,
    )
    .await;
    store
        .create_course(
            foreign_context,
            CreateCourseCommand {
                course: CourseRecord {
                    id: foreign_course,
                    tenant: foreign_tenant,
                    title: "Foreign rehearsal course".into(),
                    term: question_model::CourseTerm::from_parts(
                        "2026-08-24",
                        "2026-12-18",
                        "America/Chicago",
                    )
                    .expect("foreign course term"),
                },
                authority: foreign_course_creation_authority,
            },
        )
        .await
        .expect("foreign course");
    let target_assignment = store
        .get_assignment_for_edit(fixture.context, fixture.assignment)
        .await
        .expect("target assignment read")
        .expect("target assignment");
    // The foreign course owns its own published native problem.  Starting
    // through the route admission boundary freezes that ordinary item; this
    // test never fabricates cross-tenant rehearsal material.
    let foreign_problem = publish_assignment_version(
        &store,
        foreign_context,
        foreign_tenant,
        foreign_instructor,
        990_010,
        question_model::PublicationScope::Public,
    )
    .await;
    let foreign_assignment = AssignmentId::from_uuid(uuid(990_011));
    let foreign_stored = store
        .create_assignment(
            foreign_context,
            learning_data_access::CreateAssignmentCommand {
                actor: foreign_instructor,
                assignment: AssignmentRecord {
                    id: foreign_assignment,
                    tenant: foreign_tenant,
                    course_id: foreign_course,
                    lifecycle: question_model::AssignmentLifecycle::Draft,
                    items: fixed_items(vec![foreign_problem]),
                    ..target_assignment.record.clone()
                },
                base_policy: question_model::BaseAssignmentPolicy::default(),
            },
        )
        .await
        .expect("foreign same-shaped assignment");
    store
        .put_assignment_teaching_settings(
            foreign_context,
            PutAssignmentTeachingSettingsCommand {
                actor: foreign_instructor,
                course: foreign_course,
                assignment: foreign_assignment,
                expected_revision: foreign_stored.revision,
                settings: question_model::AssignmentTeachingSettings {
                    lifecycle: question_model::AssignmentLifecycle::Published,
                    instructions: target_assignment.record.instructions.clone(),
                    base_policy: target_assignment.base_policy,
                },
            },
        )
        .await
        .expect("publish foreign assignment");
    let foreign_reference = store
        .assignment_reference(foreign_context, foreign_instructor, foreign_assignment)
        .await
        .expect("foreign assignment lookup")
        .expect("foreign assignment reference");
    let foreign_revision = TeachingOperationRevision::new(
        store
            .get_assignment_for_edit(foreign_context, foreign_assignment)
            .await
            .expect("foreign assignment read")
            .expect("foreign assignment")
            .revision
            .value(),
    )
    .expect("foreign revision");
    let foreign_receipt = store
        .start_rehearsal_from_route(
            foreign_context,
            StartRehearsalRouteCommand {
                actor: foreign_instructor,
                course: foreign_course,
                assignment: foreign_reference,
                expected_revision: foreign_revision,
                subject: synthetic_start(),
                start_new_after_completion: false,
                idempotency_key: RehearsalSubmissionIdempotencyKey::new(
                    "foreign-retention-rehearsal".into(),
                )
                .expect("foreign rehearsal key"),
                request_fingerprint: RehearsalOperationDigest::from_bytes([0x99; 32]),
            },
        )
        .await
        .expect("foreign active rehearsal")
        .receipt;
    let foreign_locator = RehearsalLocator {
        actor: foreign_instructor,
        course: foreign_course,
        assignment: foreign_reference,
        revision: foreign_revision,
        rehearsal: foreign_receipt.rehearsal,
    };
    delete_student_records(&store, &fixture, AssignmentDefinitionDisposition::Delete).await;

    assert!(
        store
            .read_rehearsal(fixture.context, target_locator)
            .await
            .is_err()
    );
    store
        .verify_rehearsal_archive_for_test(fixture.context.tenant_id(), target_locator.rehearsal)
        .expect("target retained archive verification");
    assert_eq!(
        store
            .read_rehearsal(foreign_context, foreign_locator)
            .await
            .expect("foreign rehearsal remains readable")
            .lifecycle,
        RehearsalLifecycle::Active
    );
    store
        .verify_rehearsal_archive_for_test(foreign_tenant, foreign_locator.rehearsal)
        .expect("foreign active aggregate verifies");
}
