#![cfg(feature = "postgres")]

//! Connected authority oracle for the sealed Student-work inspection broker.
//!
//! Fixture-heavy projection coverage belongs to the Store conformance suite.
//! This disposable connected oracle proves the real role/function boundary that
//! cannot be represented by Memory.

#[path = "support/acceptance_runtime.rs"]
mod acceptance_runtime;
#[path = "postgres_course_creation_support.rs"]
mod course_creation_support;
#[path = "postgres_student_work_inspection_live/fixture.rs"]
mod fixture;
#[path = "fixtures/published_assignment.rs"]
mod published_assignment;

use acceptance_runtime::load as load_acceptance_runtime;
use course_creation_support::sysadmin_course_creation_authority;
use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    InspectStudentWorkRequest, NavigationReferenceStore, SessionLifetime, SessionStore,
    SessionSubject, SessionTokenHash, StudentWorkInspectionFocusTarget,
    StudentWorkInspectionReturnContext, StudentWorkInspectionStore,
    TeachingAuthorityReferenceStore,
};
use objects::Sha256Digest;
use published_assignment::create_published_assignment;
use question_model::UserRole;
use serde_json::Value;
use sqlx::Row;
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

async fn audit_counts(
    pool: &sqlx::PgPool,
    tenant: question_model::TenantId,
    course: question_model::CourseId,
) -> (i64, i64) {
    let record_access: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM record_access_log WHERE tenant_id = $1 AND course_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(pool)
    .await
    .expect("count inspection record-access facts");
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_event WHERE tenant_id = $1 AND course_id = $2 \
         AND action = 'gradebook_inspection'",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .fetch_one(pool)
    .await
    .expect("count inspection audit facts");
    (record_access, audit)
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_student_work_inspection_broker_is_execute_only_and_fail_closed() {
    let runtime = load_acceptance_runtime();
    let pool = lazy_pool(runtime.admin_url().expose()).expect("PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("migrated PostgreSQL schema");
    let function =
        "public.ple_inspect_student_work_v1(uuid,character,integer,integer,integer,integer)";
    let row = sqlx::query(
        "SELECT procedure_row.prosecdef, procedure_row.proconfig, \
                procedure_row.proowner::regrole::text AS owner \
         FROM pg_catalog.pg_proc AS procedure_row \
         WHERE procedure_row.oid = $1::regprocedure",
    )
    .bind(function)
    .fetch_one(&pool)
    .await
    .expect("inspection broker catalog row");
    assert!(
        row.try_get::<bool, _>("prosecdef")
            .expect("security definer")
    );
    assert_eq!(
        row.try_get::<String, _>("owner").expect("broker owner"),
        "ple_student_work_inspection_broker"
    );
    assert_eq!(
        row.try_get::<Vec<String>, _>("proconfig")
            .expect("fixed search path"),
        vec!["search_path=pg_catalog, public, pg_temp"]
    );

    let mut transaction = pool.begin().await.expect("app transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("app role");
    assert!(
        sqlx::query("SELECT response_canonical_json FROM accepted_submission_private_response")
            .fetch_optional(&mut *transaction)
            .await
            .is_err()
    );
    transaction
        .rollback()
        .await
        .expect("rollback denied direct-read transaction");

    let mut transaction = pool.begin().await.expect("broker transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("app role");
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ple_inspect_student_work_v1( \
         '00000000-0000-4000-8000-0000000000a1', repeat('0', 64)::character(64), 1, 1, 1, 1)",
    )
    .fetch_one(&mut *transaction)
    .await
    .expect("closed broker call");
    assert_eq!(count, 0, "invalid route and session remain concealed");
    transaction
        .rollback()
        .await
        .expect("rollback broker transaction");

    let store = PostgresStore::with_question_id_secret(pool.clone(), [0x18; 32]);
    let fixture = fixture::create(&store, runtime.fast_path_url().expose()).await;
    assert_eq!(
        fixture.presentation_digest.to_hex().len(),
        64,
        "the submitted EnvelopeV1 receipt retains a full presentation digest"
    );
    let session = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            session,
            SessionSubject::new(
                fixture.tenant,
                fixture.instructor,
                "Student-work inspection fixture Instructor",
                vec![UserRole::Instructor],
            )
            .expect("valid direct Instructor session"),
            SessionLifetime::from_seconds(3_600).expect("positive fixture session lifetime"),
        )
        .await
        .expect("persist direct Instructor session");
    let course = store
        .course_reference(fixture.context, fixture.instructor, fixture.course)
        .await
        .expect("course reference")
        .expect("Instructor course reference");
    let membership = store
        .course_membership_reference(
            fixture.context,
            fixture.instructor,
            fixture.course,
            fixture.membership,
        )
        .await
        .expect("Student membership reference")
        .expect("Instructor Student membership reference");
    let assignment = store
        .assignment_reference(fixture.context, fixture.instructor, fixture.assignment)
        .await
        .expect("assignment reference")
        .expect("Instructor assignment reference");
    let run = store
        .run_reference(fixture.context, fixture.instructor, fixture.run)
        .await
        .expect("run reference")
        .expect("Instructor run reference");
    let request = InspectStudentWorkRequest {
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
    let counts_before = audit_counts(&pool, fixture.tenant, fixture.course).await;
    let detail = store
        .inspect_student_work(fixture.context, session, request)
        .await
        .expect("direct Instructor inspects completed Student work");
    assert_eq!(detail.course, course);
    assert_eq!(detail.membership, membership);
    assert_eq!(detail.assignment, assignment);
    assert_eq!(detail.run, run);
    assert_eq!(
        detail.student_display_label.as_str(),
        "Inspection fixture Student",
        "the broker returns the active roster display label"
    );
    assert_eq!(
        detail.assignment_title, "Student-work inspection fixture assignment",
        "the broker returns the current assignment title"
    );
    assert_eq!(
        detail.submissions.len(),
        1,
        "one issued response is inspected"
    );
    let inspected = &detail.submissions[0];
    assert_eq!(
        inspected.response,
        question_model::presentation::InspectedStudentResponseV1::Numeric { value: 18.0 }
    );
    assert_eq!(inspected.scoring_generation, fixture.scoring_generation);
    assert_eq!(
        inspected.scoring_status,
        question_model::ScoringStatus::Current
    );
    assert_eq!(inspected.feedback.correctness, Some(true));
    assert_eq!(inspected.feedback.points_earned, Some(1.0));
    assert_eq!(inspected.feedback.points_possible, Some(1.0));
    let learning_data_access::InspectedSubmissionEvidenceV1::IssuedPresentation {
        presentation,
        issued_presentation_digest,
    } = &inspected.evidence
    else {
        panic!("EnvelopeV1 fixture retains issued-presentation evidence");
    };
    assert_eq!(
        presentation.envelope.presentation_nonce,
        presentation_binding_nonce()
    );
    assert_eq!(*issued_presentation_digest, fixture.presentation_digest);
    let counts_after_success = audit_counts(&pool, fixture.tenant, fixture.course).await;
    assert_eq!(counts_after_success.0, counts_before.0 + 1);
    assert_eq!(counts_after_success.1, counts_before.1 + 1);

    let access_row = sqlx::query(
        "SELECT floor(extract(epoch FROM occurred_at) * 1000)::bigint AS occurred_at_millis, \
                payload::text AS payload_text, payload_sha256, delivery_scope, delivery_id, course_id \
         FROM record_access_log WHERE tenant_id = $1 AND course_id = $2 \
         ORDER BY occurred_at DESC, access_log_id DESC LIMIT 1",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("paired record-access fact");
    let audit_row = sqlx::query(
        "SELECT floor(extract(epoch FROM occurred_at) * 1000)::bigint AS occurred_at_millis, \
                actor_id, action, target_kind, target_id, payload::text AS payload_text, payload_sha256 \
         FROM audit_event WHERE tenant_id = $1 AND course_id = $2 \
           AND action = 'gradebook_inspection' \
         ORDER BY occurred_at DESC, audit_event_id DESC LIMIT 1",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .fetch_one(&pool)
    .await
    .expect("paired audit-event fact");
    let access_payload: String = access_row.try_get("payload_text").expect("access payload");
    let audit_payload: String = audit_row.try_get("payload_text").expect("audit payload");
    let access_digest: String = access_row.try_get("payload_sha256").expect("access digest");
    let audit_digest: String = audit_row.try_get("payload_sha256").expect("audit digest");
    assert_eq!(access_payload, audit_payload);
    assert_eq!(access_digest, audit_digest);
    assert_eq!(
        access_digest,
        Sha256Digest::compute(access_payload.as_bytes()).to_string(),
        "paired audit payload digest uses PostgreSQL's canonical JSON text"
    );
    assert_eq!(
        access_row
            .try_get::<i64, _>("occurred_at_millis")
            .expect("access timestamp"),
        audit_row
            .try_get::<i64, _>("occurred_at_millis")
            .expect("audit timestamp"),
        "both protected facts share one authoritative inspection time"
    );
    assert_eq!(
        access_row
            .try_get::<String, _>("delivery_scope")
            .expect("access scope"),
        "student_record"
    );
    assert_eq!(
        access_row
            .try_get::<Uuid, _>("delivery_id")
            .expect("access run"),
        fixture.run.as_uuid()
    );
    assert_eq!(
        access_row
            .try_get::<Uuid, _>("course_id")
            .expect("access course"),
        fixture.course.as_uuid()
    );
    assert_eq!(
        audit_row
            .try_get::<Uuid, _>("actor_id")
            .expect("audit actor"),
        fixture.instructor.as_uuid()
    );
    assert_eq!(
        audit_row
            .try_get::<String, _>("action")
            .expect("audit action"),
        "gradebook_inspection"
    );
    assert_eq!(
        audit_row
            .try_get::<String, _>("target_kind")
            .expect("audit target kind"),
        "student_work_inspection"
    );
    assert_eq!(
        audit_row
            .try_get::<Uuid, _>("target_id")
            .expect("audit target"),
        fixture.run.as_uuid()
    );
    let payload: Value = serde_json::from_str(&access_payload).expect("canonical audit JSON");
    assert_eq!(payload["purpose"], "gradebook_inspection");
    assert_eq!(payload["actorId"], fixture.instructor.as_uuid().to_string());
    assert_eq!(
        payload["membershipId"],
        fixture.membership.as_uuid().to_string()
    );
    assert_eq!(
        payload["assignmentId"],
        fixture.assignment.as_uuid().to_string()
    );
    assert_eq!(payload["runId"], fixture.run.as_uuid().to_string());
    assert_eq!(
        payload["submissions"][0]["attemptId"],
        fixture.attempt.as_uuid().to_string()
    );
    assert_eq!(payload["submissions"][0]["evidence"], "issued_presentation");
    assert_eq!(
        payload["submissions"][0]["presentationDigest"],
        fixture.presentation_digest.to_hex()
    );
    assert!(
        !access_payload.contains("Inspection fixture Student")
            && !access_payload.contains("Student-work inspection fixture assignment"),
        "the immutable audit fact omits concrete current presentation values regardless of field spelling"
    );

    let mismatched_return = InspectStudentWorkRequest {
        return_context: StudentWorkInspectionReturnContext::Gradebook {
            course,
            membership,
            assignment,
            focus: StudentWorkInspectionFocusTarget::GradebookCell {
                membership: question_model::CourseMembershipReference::new(
                    u64::from(membership.number()) + 1,
                )
                .expect("different valid public reference"),
                assignment,
            },
        },
        ..request
    };
    assert_eq!(
        store
            .inspect_student_work(fixture.context, session, mismatched_return)
            .await,
        Err(learning_data_access::StoreError::NotFound),
        "Rust rejects a mismatched closed return context after broker rows are decoded"
    );
    assert_eq!(
        audit_counts(&pool, fixture.tenant, fixture.course).await,
        counts_after_success,
        "failed Rust validation rolls the paired SQL writes back"
    );

    let outsider = question_model::UserId::from_uuid(id());
    let outsider_session = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            outsider_session,
            SessionSubject::new(
                fixture.tenant,
                outsider,
                "Unaffiliated fixture Instructor",
                vec![UserRole::Instructor],
            )
            .expect("valid unaffiliated Instructor session"),
            SessionLifetime::from_seconds(3_600).expect("positive fixture session lifetime"),
        )
        .await
        .expect("persist unaffiliated Instructor session");
    assert_eq!(
        store
            .inspect_student_work(fixture.context, outsider_session, request)
            .await,
        Err(learning_data_access::StoreError::NotFound),
        "an unaffiliated Instructor session receives the same concealed result"
    );
    assert_eq!(
        audit_counts(&pool, fixture.tenant, fixture.course).await,
        counts_after_success,
        "a denied exact-session role check emits no successful Student-record audit"
    );

    let mut malformed_label = pool.begin().await.expect("malformed-label transaction");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *malformed_label)
        .await
        .expect("bind malformed-label fixture tenant");
    sqlx::query(
        "ALTER TABLE public.course_roster_profile \
         DROP CONSTRAINT course_roster_profile_display_name_check",
    )
    .execute(&mut *malformed_label)
    .await
    .expect("disposable fixture permits malformed-label evidence");
    sqlx::query(
        "UPDATE public.course_roster_profile SET display_name = ' ' \
         WHERE tenant_id = $1 AND course_id = $2 AND course_membership_id = $3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.membership.as_uuid())
    .execute(&mut *malformed_label)
    .await
    .expect("seed malformed current roster display label");
    malformed_label
        .commit()
        .await
        .expect("commit malformed-label fixture");
    assert_eq!(
        store
            .inspect_student_work(fixture.context, session, request)
            .await,
        Err(learning_data_access::StoreError::NotFound),
        "a malformed current label conceals the inspection detail"
    );
    assert_eq!(
        audit_counts(&pool, fixture.tenant, fixture.course).await,
        counts_after_success,
        "malformed label evidence writes neither successful audit fact"
    );
    let mut restored_label = pool.begin().await.expect("restore-label transaction");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(fixture.tenant.to_string())
        .execute(&mut *restored_label)
        .await
        .expect("bind restored-label fixture tenant");
    sqlx::query(
        "UPDATE public.course_roster_profile \
         SET display_name = 'Inspection fixture Student' \
         WHERE tenant_id = $1 AND course_id = $2 AND course_membership_id = $3",
    )
    .bind(fixture.tenant.as_uuid())
    .bind(fixture.course.as_uuid())
    .bind(fixture.membership.as_uuid())
    .execute(&mut *restored_label)
    .await
    .expect("restore disposable fixture display label");
    sqlx::query(
        "ALTER TABLE public.course_roster_profile \
         ADD CONSTRAINT course_roster_profile_display_name_check \
         CHECK (char_length(display_name) BETWEEN 1 AND 200 \
             AND display_name = btrim(display_name))",
    )
    .execute(&mut *restored_label)
    .await
    .expect("restore disposable fixture display-label constraint");
    restored_label
        .commit()
        .await
        .expect("commit restored-label fixture");
}

fn presentation_binding_nonce() -> question_model::presentation::PresentationNonceV1 {
    let seed = 18_u64;
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&seed.to_le_bytes());
    bytes[8..].copy_from_slice(&seed.rotate_left(7).to_le_bytes());
    question_model::presentation::PresentationNonceV1::from_bytes(bytes)
}
