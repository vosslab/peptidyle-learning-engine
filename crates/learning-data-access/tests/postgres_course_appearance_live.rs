#![cfg(feature = "postgres")]

//! Disposable PostgreSQL oracle for course-appearance CAS, RLS, and current delivery.

use learning_data_access::postgres::{PostgresStore, lazy_pool, verify_application_schema};
use learning_data_access::{
    AuthoritativeTimeStore, COURSE_BANNER_HEIGHT, COURSE_BANNER_WIDTH, CourseAppearanceStore,
    CourseBannerCleanupBatch, CourseRecord, RegisterCourseBannerCandidate, SaveCourseAppearance,
    SessionLifetime, SessionStore, SessionSubject, SessionTokenHash, Store, StoreError,
    TenantContext,
};
use objects::{ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{
    ActivityTimestamp, CourseAppearanceRevision, CourseAppearanceUpdate,
    CourseBannerAlternativeText, CourseBannerCandidateId, CourseBannerId, CourseBannerMutation,
    CourseId, CourseMembership, CourseMembershipRole, CourseThemeId, TenantId, UserId, UserRole,
};
use uuid::Uuid;

fn id() -> Uuid {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).expect("live fixture UUID randomness");
    Uuid::from_bytes(bytes)
}

fn object_record(key: ObjectKey, bytes: &[u8], created_at: ActivityTimestamp) -> ObjectRecord {
    ObjectRecord {
        id: key.object_id(),
        bucket: key.bucket(),
        sha256: Sha256Digest::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: "image/webp".to_string(),
        category: key.category(),
        version: key.version_id(),
        key,
        license: "tenant course branding".to_string(),
        provenance: "live course appearance oracle".to_string(),
        created_at,
    }
}

async fn session(
    store: &PostgresStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
) -> SessionTokenHash {
    let hash = SessionTokenHash::compute(id().as_bytes());
    store
        .create_session(
            hash,
            SessionSubject::new(tenant, user, "Live appearance fixture", roles)
                .expect("valid session subject"),
            SessionLifetime::from_seconds(3_600).expect("valid session lifetime"),
        )
        .await
        .expect("live session should persist");
    hash
}

#[tokio::test]
#[ignore = "requires the disposable PostgreSQL acceptance database"]
async fn postgres_course_appearance_is_revisioned_role_bound_and_current_only() {
    let database_url = std::env::var("PLE_TEST_DATABASE_URL")
        .expect("PLE_TEST_DATABASE_URL must name the disposable acceptance database");
    let pool = lazy_pool(&database_url).expect("valid live PostgreSQL URL");
    verify_application_schema(&pool)
        .await
        .expect("live PostgreSQL schema compatibility");
    let store = PostgresStore::new(pool.clone());
    let tenant = TenantId::from_uuid(id());
    let foreign_tenant = TenantId::from_uuid(id());
    let course = CourseId::from_uuid(id());
    let instructor = UserId::from_uuid(id());
    let replacement_instructor = UserId::from_uuid(id());
    let student = UserId::from_uuid(id());
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let instructor_session = session(&store, tenant, instructor, vec![UserRole::Instructor]).await;
    let student_session = session(&store, tenant, student, vec![UserRole::Student]).await;
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Live appearance course".to_string(),
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
        .expect("course and trigger-owned default appearance should persist");
    let initial = store
        .course_appearance(context, instructor_session, course)
        .await
        .expect("appearance read should run")
        .expect("instructor should see default appearance");
    assert_eq!(initial.theme, CourseThemeId::Grass);
    assert!(
        store
            .course_appearance(foreign_context, instructor_session, course)
            .await
            .expect("foreign read should run")
            .is_none()
    );
    assert_eq!(
        store
            .save_course_appearance(
                context,
                student_session,
                course,
                SaveCourseAppearance {
                    expected_revision: initial.revision,
                    update: CourseAppearanceUpdate {
                        theme: CourseThemeId::Beach,
                        banner: CourseBannerMutation::Remove,
                    },
                    promoted_object: None,
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );

    let now = store
        .authoritative_time(context)
        .await
        .expect("database time should resolve");
    let expires_at = ActivityTimestamp::from_unix_millis(
        now.as_unix_millis()
            .checked_add(60_000)
            .expect("fixture expiry should fit"),
    );
    let first_candidate = CourseBannerCandidateId::from_uuid(id());
    let first_banner = CourseBannerId::from_uuid(id());
    let bytes = b"live normalized banner";
    let candidate_object = object_record(
        ObjectKey::CourseBannerCandidate {
            tenant,
            course,
            candidate: first_candidate,
        },
        bytes,
        now,
    );
    store
        .register_course_banner_candidate(
            context,
            instructor_session,
            course,
            RegisterCourseBannerCandidate {
                candidate: first_candidate,
                object: candidate_object,
                banner: first_banner,
                width: COURSE_BANNER_WIDTH,
                height: COURSE_BANNER_HEIGHT,
                expires_at,
            },
        )
        .await
        .expect("candidate should persist");
    let promotion = store
        .course_banner_promotion(context, instructor_session, course, first_candidate)
        .await
        .expect("candidate owner should resolve hidden promotion evidence");
    assert_eq!(promotion.candidate, first_candidate);
    assert_eq!(promotion.banner, first_banner);
    assert_eq!(promotion.sha256, Sha256Digest::compute(bytes));
    assert_eq!(promotion.size_bytes, bytes.len() as u64);
    assert_eq!(
        store
            .course_banner_promotion(foreign_context, instructor_session, course, first_candidate,)
            .await,
        Err(StoreError::NotFound)
    );
    let promoted = object_record(
        ObjectKey::CourseBanner {
            tenant,
            course,
            banner: first_banner,
        },
        bytes,
        now,
    );
    assert_eq!(
        store
            .save_course_appearance(
                context,
                instructor_session,
                course,
                SaveCourseAppearance {
                    expected_revision: CourseAppearanceRevision::new(2).expect("fixture revision"),
                    update: CourseAppearanceUpdate {
                        theme: CourseThemeId::Forest,
                        banner: CourseBannerMutation::Replace {
                            candidate: first_candidate,
                            alternative_text: CourseBannerAlternativeText::Decorative,
                        },
                    },
                    promoted_object: Some(promoted.clone()),
                },
            )
            .await,
        Err(StoreError::Conflict),
        "a stale save must durably own but not select its copied object"
    );
    assert_eq!(
        store
            .authorize_course_banner_delivery(context, student_session, first_banner)
            .await,
        Err(StoreError::NotFound)
    );
    let saved = store
        .save_course_appearance(
            context,
            instructor_session,
            course,
            SaveCourseAppearance {
                expected_revision: initial.revision,
                update: CourseAppearanceUpdate {
                    theme: CourseThemeId::Forest,
                    banner: CourseBannerMutation::Replace {
                        candidate: first_candidate,
                        alternative_text: CourseBannerAlternativeText::Decorative,
                    },
                },
                promoted_object: Some(promoted),
            },
        )
        .await
        .expect("current CAS should select the tracked promoted object");
    assert_eq!(saved.revision.value(), 2);
    store
        .authorize_course_banner_delivery(context, student_session, first_banner)
        .await
        .expect("student should receive only the exact current banner");

    let other_course = CourseId::from_uuid(id());
    store
        .upsert_course(
            context,
            CourseRecord {
                id: other_course,
                tenant,
                title: "Other live appearance course".to_string(),
                members: vec![CourseMembership {
                    user: instructor,
                    role: CourseMembershipRole::Instructor,
                }],
            },
        )
        .await
        .expect("second course should persist");
    let mut transaction = pool.begin().await.expect("negative probe transaction");
    sqlx::query("SET LOCAL ROLE ple_app")
        .execute(&mut *transaction)
        .await
        .expect("negative probe should assume the application role");
    sqlx::query("SELECT set_config('ple.tenant_id', $1, true)")
        .bind(tenant.to_string())
        .execute(&mut *transaction)
        .await
        .expect("negative probe should set the tenant context");
    let cross_course_pointer = sqlx::query(
        "UPDATE course_appearance SET current_banner_delivery_id = $3, \
         banner_alt_kind = 'decorative' \
         WHERE tenant_id = $1 AND course_id = $2",
    )
    .bind(tenant.as_uuid())
    .bind(other_course.as_uuid())
    .bind(first_banner.as_uuid())
    .execute(&mut *transaction)
    .await
    .expect_err("database must reject a current pointer owned by another course");
    let error_code = cross_course_pointer
        .as_database_error()
        .and_then(|error| error.code())
        .map(|code| code.into_owned());
    assert_eq!(error_code.as_deref(), Some("23514"));
    transaction
        .rollback()
        .await
        .expect("negative probe should roll back cleanly");

    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Live appearance course".to_string(),
                members: vec![
                    CourseMembership {
                        user: replacement_instructor,
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
        .expect("membership replacement should preserve appearance");
    assert_eq!(
        store
            .save_course_appearance(
                context,
                instructor_session,
                course,
                SaveCourseAppearance {
                    expected_revision: saved.revision,
                    update: CourseAppearanceUpdate {
                        theme: CourseThemeId::Desert,
                        banner: CourseBannerMutation::Remove,
                    },
                    promoted_object: None,
                },
            )
            .await,
        Err(StoreError::NotFound)
    );

    sqlx::query(
        "UPDATE course_banner_candidate \
         SET created_at = transaction_timestamp() - interval '2 minutes', \
             expires_at = transaction_timestamp() - interval '1 minute' \
         WHERE tenant_id = $1 AND course_id = $2 AND candidate_id = $3",
    )
    .bind(tenant.as_uuid())
    .bind(course.as_uuid())
    .bind(first_candidate.as_uuid())
    .execute(&pool)
    .await
    .expect("privileged disposable oracle should age the candidate");
    let claims = store
        .claim_course_banner_cleanup(
            context,
            CourseBannerCleanupBatch::new(10).expect("valid cleanup batch"),
        )
        .await
        .expect("cleanup claim should run");
    let current_claim = claims
        .into_iter()
        .find(|claim| claim.candidate == first_candidate)
        .expect("expired current candidate bytes should be claimed");
    assert!(current_claim.candidate_object.is_some());
    assert!(current_claim.promoted_object.is_none());
    assert!(
        store
            .complete_course_banner_cleanup(context, current_claim)
            .await
            .expect("cleanup completion should run")
    );
    store
        .authorize_course_banner_delivery(context, student_session, first_banner)
        .await
        .expect("candidate cleanup must retain current promoted bytes");
}
