//! Memory conformance for the audited Instructor Student-work detail read.

use super::external_tool::{ExternalToolFixture, external_begin, external_tool_fixture};
use super::*;
use learning_data_access::{
    CourseRosterEntry, CourseRosterStore, JobClaimFilter, JobLeaseDuration, JobPayload, JobStore,
    NavigationReferenceStore, RetentionApiStore, RetentionStore, RetentionWorkerCommand,
    RetentionWorkerStore, RevokeCourseMember, StudentWorkInspectionFocusTarget,
    StudentWorkInspectionReturnContext, StudentWorkInspectionStore,
    TeachingAuthorityReferenceStore,
};

async fn inspection_request(
    store: &MemoryStore,
    fixture: &RunApiFixture,
) -> learning_data_access::InspectStudentWorkRequest {
    let course = store
        .course_reference(fixture.context, fixture.publisher, fixture.course)
        .await
        .expect("course reference")
        .expect("authorized course reference");
    let assignment = store
        .assignment_reference(fixture.context, fixture.publisher, fixture.assignment)
        .await
        .expect("assignment reference")
        .expect("authorized assignment reference");
    let run = store
        .run_reference(fixture.context, fixture.student_user, fixture.run.id)
        .await
        .expect("run reference")
        .expect("Student run reference");
    let membership = store
        .list_course_active_student_membership_reference_views(
            fixture.context,
            fixture.publisher,
            fixture.course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("active Student references")
        .items
        .into_iter()
        .next()
        .expect("fixture Student membership")
        .reference;
    learning_data_access::InspectStudentWorkRequest {
        course,
        membership,
        assignment,
        run,
        return_context: StudentWorkInspectionReturnContext::Gradebook {
            course,
            membership,
            assignment,
            focus: StudentWorkInspectionFocusTarget::GradebookCell {
                membership,
                assignment,
            },
        },
    }
}

async fn inspection_session(
    store: &MemoryStore,
    fixture: &RunApiFixture,
    material: &[u8],
) -> SessionTokenHash {
    let session = SessionTokenHash::compute(material);
    store
        .create_session(
            session,
            SessionSubject::new(
                fixture.tenant,
                fixture.publisher,
                "Student-work inspection Instructor",
                vec![UserRole::Instructor],
            )
            .expect("Instructor session"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("session persists");
    session
}

async fn commit_external_tool_submission(store: &MemoryStore, fixture: &ExternalToolFixture) {
    let begin = external_begin(fixture, "inspection-external-tool");
    let ExternalToolBegin::Lease(lease) = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("external verification lease")
    else {
        panic!("first external inspection exchange must lease");
    };
    let launch = store
        .create_external_tool_launch_session(
            fixture.context,
            CreateExternalToolLaunchSessionCommand {
                actor: fixture.actor,
                student_work_binding: fixture.student_work_binding(),
                attempt: fixture.attempt,
                binding: fixture.binding.clone(),
                encrypted_provider_state: None,
                lifetime_millis: 60_000,
            },
        )
        .await
        .expect("external launch session");
    let result = AttemptResult {
        correct: true,
        points_earned: 1.0,
        points_possible: 1.0,
    };
    store
        .stage_external_tool_verification(
            fixture.context,
            StageExternalToolVerificationCommand {
                actor: fixture.actor,
                student_work_binding: fixture.student_work_binding(),
                attempt: fixture.attempt,
                response: StudentResponse::ExternalTool {},
                idempotency_key: begin.idempotency_key.clone(),
                binding: fixture.binding.clone(),
                correlation: lease.correlation,
                lease_token: lease.token,
                result,
            },
        )
        .await
        .expect("external verification");
    let ExternalToolBegin::VerifiedPending(verified) = store
        .begin_or_resume_external_grade(fixture.context, begin.clone())
        .await
        .expect("verified external submission")
    else {
        panic!("verified external submission must await commit");
    };
    store
        .commit_verified_external_tool_submission(
            fixture.context,
            CommitVerifiedExternalToolSubmissionCommand {
                actor: fixture.actor,
                student_work_binding: fixture.student_work_binding(),
                attempt: fixture.attempt,
                response: StudentResponse::ExternalTool {},
                idempotency_key: begin.idempotency_key,
                binding: verified.binding,
                correlation: verified.correlation,
                launch_proof: ExternalToolLaunchProof {
                    session_id: launch.id,
                    token: launch.token,
                },
            },
        )
        .await
        .expect("external receipt commit");
}

#[tokio::test]
async fn inspection_accepts_verified_external_tool_receipts_without_presentation() {
    let store = MemoryStore::default();
    let fixture = external_tool_fixture(&store).await;
    commit_external_tool_submission(&store, &fixture).await;
    let session = SessionTokenHash::compute(b"inspection-external-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                fixture.context.tenant_id(),
                fixture.instructor,
                "Student-work inspection Instructor",
                vec![UserRole::Instructor],
            )
            .expect("Instructor session"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("session persists");
    let course = store
        .course_reference(fixture.context, fixture.instructor, fixture.course)
        .await
        .expect("course reference")
        .expect("course reference visible");
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment reference")
        .expect("assignment reference visible");
    let run = store
        .run_reference(fixture.context, fixture.instructor, fixture.run)
        .await
        .expect("run reference")
        .expect("run reference visible");
    let membership = store
        .course_membership_reference(
            fixture.context,
            fixture.instructor,
            fixture.course,
            question_model::CourseMembershipId::from_uuid(
                store
                    .list_course_roster(
                        fixture.context,
                        session,
                        fixture.course,
                        PageRequest::first(PageSize::new(10).expect("page size")),
                    )
                    .await
                    .expect("course roster")
                    .entries
                    .items
                    .into_iter()
                    .find_map(|entry| match entry {
                        CourseRosterEntry::Member(member) if member.user == fixture.actor => {
                            Some(member.id.as_uuid())
                        }
                        CourseRosterEntry::Member(_) | CourseRosterEntry::Invitation(_) => None,
                    })
                    .expect("external Student roster member"),
            ),
        )
        .await
        .expect("Student membership reference")
        .expect("external Student membership");
    let detail = store
        .inspect_student_work(
            fixture.context,
            session,
            learning_data_access::InspectStudentWorkRequest {
                course,
                membership,
                assignment,
                run,
                return_context: StudentWorkInspectionReturnContext::Gradebook {
                    course,
                    membership,
                    assignment,
                    focus: StudentWorkInspectionFocusTarget::GradebookCell {
                        membership,
                        assignment,
                    },
                },
            },
        )
        .await
        .expect("verified ExternalTool inspection");
    assert!(detail.submissions.iter().all(|submission| matches!(
        submission.evidence,
        learning_data_access::InspectedSubmissionEvidenceV1::PresentationNotApplicable
    )));
}

#[tokio::test]
async fn memory_inspection_reads_verified_receipts_and_appends_paired_audits() {
    let store = MemoryStore::default();
    let fixture =
        exercise_run_api_receipts(&store, StudentDisclosurePolicy::default(), 80_000).await;
    let session = SessionTokenHash::compute(b"student-work-inspection-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                fixture.tenant,
                fixture.publisher,
                "Student-work inspection Instructor",
                vec![UserRole::Instructor],
            )
            .expect("Instructor session"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("session persists");
    let course = store
        .course_reference(fixture.context, fixture.publisher, fixture.course)
        .await
        .expect("course reference")
        .expect("authorized course reference");
    let assignment = store
        .assignment_reference(fixture.context, fixture.publisher, fixture.assignment)
        .await
        .expect("assignment reference")
        .expect("authorized assignment reference");
    let run = store
        .run_reference(fixture.context, fixture.student_user, fixture.run.id)
        .await
        .expect("run reference")
        .expect("Student run reference");
    let membership = store
        .list_course_active_student_membership_reference_views(
            fixture.context,
            fixture.publisher,
            fixture.course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("active Student references")
        .items
        .into_iter()
        .next()
        .expect("fixture Student membership")
        .reference;
    let before = store
        .student_work_inspection_audit_facts()
        .expect("audit facts before read");
    let detail = store
        .inspect_student_work(
            fixture.context,
            session,
            learning_data_access::InspectStudentWorkRequest {
                course,
                membership,
                assignment,
                run,
                return_context: StudentWorkInspectionReturnContext::Gradebook {
                    course,
                    membership,
                    assignment,
                    focus: StudentWorkInspectionFocusTarget::GradebookCell {
                        membership,
                        assignment,
                    },
                },
            },
        )
        .await
        .expect("authorized receipt-backed inspection");
    assert!(!detail.submissions.is_empty());
    assert_eq!(detail.student_display_label.as_str(), "Run learner");
    assert_eq!(detail.assignment_title, "Run API assignment");
    assert!(detail.submissions.iter().all(|submission| matches!(
        submission.evidence,
        learning_data_access::InspectedSubmissionEvidenceV1::IssuedPresentation { .. }
    )));
    assert!(detail.submissions.iter().all(|submission| {
        submission.feedback.correctness.is_some()
            || submission.feedback.points_earned.is_some()
            || submission.feedback.points_possible.is_some()
    }));
    let debug = format!("{detail:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(
        !debug.contains("18.0"),
        "detail Debug must not expose response values"
    );
    let after = store
        .student_work_inspection_audit_facts()
        .expect("audit facts after read");
    assert_eq!(after.0.len(), before.0.len() + 1);
    assert_eq!(after.1.len(), before.1.len() + 1);
    let record_access = after.0.last().expect("record access");
    let audit = after.1.last().expect("audit event");
    assert_eq!(record_access.actor, fixture.publisher);
    assert_eq!(record_access.intent, audit.intent);
    assert_eq!(record_access.occurred_at, audit.occurred_at);
    assert_eq!(record_access.actor, audit.actor);
    assert_eq!(record_access.tenant, audit.tenant);
    assert_eq!(record_access.course, audit.course);
    assert_eq!(record_access.membership, audit.membership);
    assert_eq!(record_access.assignment, audit.assignment);
    assert_eq!(record_access.run, audit.run);
    assert_eq!(record_access.scoring_generation, audit.scoring_generation);
    assert_eq!(record_access.scoring_status, audit.scoring_status);
    assert_eq!(record_access.submissions, audit.submissions);
    assert_eq!(record_access.tenant, fixture.tenant);
    assert_eq!(record_access.course, fixture.course);
    assert!(
        record_access.submissions.windows(2).all(|pair| {
            (pair[0].submitted_at, pair[0].attempt) <= (pair[1].submitted_at, pair[1].attempt)
        }),
        "inspection audit witnesses retain deterministic receipt ordering"
    );
}

#[tokio::test]
async fn inspection_conceals_invalid_composites_and_never_audits_failed_reads() {
    let store = MemoryStore::default();
    let fixture =
        exercise_run_api_receipts(&store, StudentDisclosurePolicy::default(), 81_000).await;
    let session = SessionTokenHash::compute(b"student-work-inspection-failure-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                fixture.tenant,
                fixture.publisher,
                "Student-work inspection Instructor",
                vec![UserRole::Instructor],
            )
            .expect("Instructor session"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("session persists");
    let course = store
        .course_reference(fixture.context, fixture.publisher, fixture.course)
        .await
        .expect("course reference")
        .expect("authorized course reference");
    let assignment = store
        .assignment_reference(fixture.context, fixture.publisher, fixture.assignment)
        .await
        .expect("assignment reference")
        .expect("authorized assignment reference");
    let run = store
        .run_reference(fixture.context, fixture.student_user, fixture.run.id)
        .await
        .expect("run reference")
        .expect("Student run reference");
    let membership = store
        .list_course_active_student_membership_reference_views(
            fixture.context,
            fixture.publisher,
            fixture.course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("active Student references")
        .items
        .into_iter()
        .next()
        .expect("fixture Student membership")
        .reference;
    let request = learning_data_access::InspectStudentWorkRequest {
        course,
        membership,
        assignment,
        run,
        return_context: StudentWorkInspectionReturnContext::Gradebook {
            course,
            membership,
            assignment,
            focus: StudentWorkInspectionFocusTarget::GradebookCell {
                membership,
                assignment,
            },
        },
    };
    let before = store
        .student_work_inspection_audit_facts()
        .expect("audit facts before failures");
    let mut invalid_composites = vec![request];
    invalid_composites.push(learning_data_access::InspectStudentWorkRequest {
        course: question_model::CourseReference::new(99_001).expect("reference"),
        ..request
    });
    invalid_composites.push(learning_data_access::InspectStudentWorkRequest {
        assignment: question_model::AssignmentReference::new(99_002).expect("reference"),
        ..request
    });
    invalid_composites.push(learning_data_access::InspectStudentWorkRequest {
        run: question_model::RunReference::new(99_003).expect("reference"),
        ..request
    });
    invalid_composites.push(learning_data_access::InspectStudentWorkRequest {
        return_context: StudentWorkInspectionReturnContext::Gradebook {
            course,
            membership,
            assignment,
            focus: StudentWorkInspectionFocusTarget::GradebookCell {
                membership: question_model::CourseMembershipReference::new(99_004)
                    .expect("reference"),
                assignment,
            },
        },
        ..request
    });
    for invalid in invalid_composites.into_iter().skip(1) {
        assert_eq!(
            store
                .inspect_student_work(fixture.context, session, invalid)
                .await,
            Err(StoreError::NotFound),
        );
        assert_eq!(
            store
                .student_work_inspection_audit_facts()
                .expect("audit facts"),
            before
        );
    }
}

#[tokio::test]
async fn inspection_conceals_incomplete_runs_without_audit() {
    let store = MemoryStore::default();
    let fixture =
        exercise_run_api_receipts(&store, StudentDisclosurePolicy::default(), 81_500).await;
    let session = inspection_session(&store, &fixture, b"inspection-incomplete-run").await;
    let completed_request = inspection_request(&store, &fixture).await;
    let incomplete = store
        .start_or_resume_run(
            fixture.context,
            fixture.student_user,
            StudentWorkRoutingBinding::new(fixture.course, fixture.assignment),
            RunId::from_uuid(uuid(81_501)),
        )
        .await
        .expect("incomplete run");
    let run = store
        .run_reference(fixture.context, fixture.student_user, incomplete.id)
        .await
        .expect("run reference")
        .expect("incomplete run reference");
    let before = store
        .student_work_inspection_audit_facts()
        .expect("audit facts before incomplete read");
    assert_eq!(
        store
            .inspect_student_work(
                fixture.context,
                session,
                learning_data_access::InspectStudentWorkRequest {
                    run,
                    ..completed_request
                },
            )
            .await,
        Err(StoreError::NotFound),
    );
    assert_eq!(
        store
            .student_work_inspection_audit_facts()
            .expect("audit facts after incomplete read"),
        before
    );
}

#[tokio::test]
async fn inspection_conceals_tampered_private_response_and_preserves_audit_vectors() {
    let store = MemoryStore::default();
    let fixture =
        exercise_run_api_receipts(&store, StudentDisclosurePolicy::default(), 82_000).await;
    let session = SessionTokenHash::compute(b"student-work-inspection-tamper-instructor");
    store
        .create_session(
            session,
            SessionSubject::new(
                fixture.tenant,
                fixture.publisher,
                "Student-work inspection Instructor",
                vec![UserRole::Instructor],
            )
            .expect("Instructor session"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime"),
        )
        .await
        .expect("session persists");
    let course = store
        .course_reference(fixture.context, fixture.publisher, fixture.course)
        .await
        .expect("course reference")
        .expect("authorized course reference");
    let assignment = store
        .assignment_reference(fixture.context, fixture.publisher, fixture.assignment)
        .await
        .expect("assignment reference")
        .expect("authorized assignment reference");
    let run = store
        .run_reference(fixture.context, fixture.student_user, fixture.run.id)
        .await
        .expect("run reference")
        .expect("Student run reference");
    let membership = store
        .list_course_active_student_membership_reference_views(
            fixture.context,
            fixture.publisher,
            fixture.course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("active Student references")
        .items
        .into_iter()
        .next()
        .expect("fixture Student membership")
        .reference;
    let attempt = store
        .list_question_attempts(
            fixture.context,
            fixture.run.id,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("attempts")
        .items
        .into_iter()
        .next()
        .expect("submitted attempt")
        .id;
    store
        .tamper_private_submission_response_witness_for_test(fixture.tenant, attempt)
        .expect("tamper fixture witness");
    let before = store
        .student_work_inspection_audit_facts()
        .expect("audit facts before tamper read");
    assert_eq!(
        store
            .inspect_student_work(
                fixture.context,
                session,
                learning_data_access::InspectStudentWorkRequest {
                    course,
                    membership,
                    assignment,
                    run,
                    return_context: StudentWorkInspectionReturnContext::Gradebook {
                        course,
                        membership,
                        assignment,
                        focus: StudentWorkInspectionFocusTarget::GradebookCell {
                            membership,
                            assignment
                        },
                    },
                }
            )
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store
            .student_work_inspection_audit_facts()
            .expect("audit facts after tamper read"),
        before
    );
}

#[tokio::test]
async fn inspection_conceals_revoked_student_membership_without_audit() {
    let store = MemoryStore::default();
    let fixture =
        exercise_run_api_receipts(&store, StudentDisclosurePolicy::default(), 83_000).await;
    let session = inspection_session(&store, &fixture, b"inspection-revoked-student").await;
    let request = inspection_request(&store, &fixture).await;
    let roster = store
        .list_course_roster(
            fixture.context,
            session,
            fixture.course,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("course roster");
    let member = roster
        .entries
        .items
        .into_iter()
        .find_map(|entry| match entry {
            CourseRosterEntry::Member(member) if member.user == fixture.student_user => {
                Some(member)
            }
            CourseRosterEntry::Member(_) | CourseRosterEntry::Invitation(_) => None,
        })
        .expect("fixture Student roster member");
    store
        .revoke_course_member(
            fixture.context,
            session,
            RevokeCourseMember {
                course: fixture.course,
                member: member.id,
                expected_revision: roster.policy.revision,
            },
        )
        .await
        .expect("revoke Student membership");
    let before = store
        .student_work_inspection_audit_facts()
        .expect("audit facts before read");
    assert_eq!(
        store
            .inspect_student_work(fixture.context, session, request)
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store
            .student_work_inspection_audit_facts()
            .expect("audit facts after read"),
        before
    );
}

#[tokio::test]
async fn inspection_conceals_retention_deleted_student_work_without_audit() {
    let store = MemoryStore::default();
    let fixture =
        exercise_run_api_receipts(&store, StudentDisclosurePolicy::default(), 84_000).await;
    let session = inspection_session(&store, &fixture, b"inspection-retention-delete").await;
    let request = inspection_request(&store, &fixture).await;
    store
        .inspect_student_work(fixture.context, session, request)
        .await
        .expect("pre-retention inspection creates paired facts");
    store
        .end_course_retention(fixture.context, session, fixture.course)
        .await
        .expect("end course retention");
    let view = store
        .retention_view(fixture.context, session, fixture.course)
        .await
        .expect("retention view")
        .expect("ended retention view");
    store
        .request_retention_delete_if_revision(
            fixture.context,
            session,
            fixture.course,
            view.revision,
        )
        .await
        .expect("schedule Student-record deletion");
    let job = store
        .claim_next_job(
            &JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(30).expect("lease duration"),
        )
        .await
        .expect("claim retention work")
        .expect("retention job");
    assert_eq!(
        job.payload,
        JobPayload::Retention {
            course: fixture.course,
            stage: learning_data_access::RetentionStage::DeleteStudentRecords,
            generation: 2,
        }
    );
    let command = RetentionWorkerCommand {
        course: fixture.course,
        stage: learning_data_access::RetentionStage::DeleteStudentRecords,
        generation: 2,
        job: job.id,
        lease: job.lease_token,
    };
    store
        .prepare_retention_work(command)
        .await
        .expect("prepare deletion");
    store
        .commit_retention_work(command)
        .await
        .expect("commit deletion");
    let before = store
        .student_work_inspection_audit_facts()
        .expect("audit facts after purge");
    assert!(before.0.is_empty());
    assert!(before.1.is_empty());
    assert_eq!(
        store
            .inspect_student_work(fixture.context, session, request)
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        store
            .student_work_inspection_audit_facts()
            .expect("audit facts after read"),
        before
    );
}

#[test]
fn inspected_response_debug_is_redacted() {
    let response = question_model::presentation::InspectedStudentResponseV1::ShortText {
        text: "Student private answer".to_string(),
    };
    let rendered = format!("{response:?}");
    assert!(rendered.contains("short_text"));
    assert!(!rendered.contains("Student private answer"));
}
