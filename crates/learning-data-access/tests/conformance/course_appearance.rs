use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    COURSE_BANNER_HEIGHT, COURSE_BANNER_WIDTH, CourseAppearanceStore, CourseBannerCleanupBatch,
    CourseRecord, RegisterCourseBannerCandidate, SaveCourseAppearance, SessionLifetime,
    SessionStore, SessionSubject, SessionTokenHash, Store, StoreError, TenantContext,
};
use objects::{ObjectCategory, ObjectKey, ObjectRecord, Sha256Digest};
use question_model::{
    ActivityTimestamp, CourseAppearanceRevision, CourseAppearanceUpdate,
    CourseBannerAlternativeText, CourseBannerCandidateId, CourseBannerId, CourseBannerMutation,
    CourseId, CourseMembership, CourseMembershipRole, CourseThemeId, TenantId, UserId, UserRole,
};
use uuid::Uuid;

fn id(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn banner_object_record(key: ObjectKey, bytes: &[u8], created_at: i64) -> ObjectRecord {
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
        provenance: "course appearance conformance".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(created_at),
    }
}

async fn create_session(
    store: &MemoryStore,
    tenant: TenantId,
    user: UserId,
    roles: Vec<UserRole>,
    token: &'static [u8],
) -> SessionTokenHash {
    let hash = SessionTokenHash::compute(token);
    store
        .create_session(
            hash,
            SessionSubject::new(tenant, user, "Appearance fixture", roles)
                .expect("session subject should validate"),
            SessionLifetime::from_seconds(3_600).expect("session lifetime should validate"),
        )
        .await
        .expect("session should persist");
    hash
}

fn candidate_command(
    tenant: TenantId,
    course: CourseId,
    candidate: CourseBannerCandidateId,
    banner: CourseBannerId,
    bytes: &[u8],
) -> RegisterCourseBannerCandidate {
    RegisterCourseBannerCandidate {
        candidate,
        object: banner_object_record(
            ObjectKey::CourseBannerCandidate {
                tenant,
                course,
                candidate,
            },
            bytes,
            1_000,
        ),
        banner,
        width: COURSE_BANNER_WIDTH,
        height: COURSE_BANNER_HEIGHT,
        expires_at: ActivityTimestamp::from_unix_millis(5_000),
    }
}

fn promoted_object(
    tenant: TenantId,
    course: CourseId,
    banner: CourseBannerId,
    bytes: &[u8],
) -> ObjectRecord {
    banner_object_record(
        ObjectKey::CourseBanner {
            tenant,
            course,
            banner,
        },
        bytes,
        1_100,
    )
}

#[tokio::test]
async fn memory_course_appearance_cas_delivery_membership_and_cleanup_conform() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(1_000))
        .expect("memory clock should be writable");
    let tenant = TenantId::from_uuid(id(70_001));
    let course = CourseId::from_uuid(id(70_002));
    let instructor = UserId::from_uuid(id(70_003));
    let student = UserId::from_uuid(id(70_004));
    let outsider = UserId::from_uuid(id(70_005));
    let sysadmin = UserId::from_uuid(id(70_006));
    let replacement_instructor = UserId::from_uuid(id(70_007));
    let context = TenantContext::from_authenticated_session(tenant);
    let instructor_session = create_session(
        &store,
        tenant,
        instructor,
        vec![UserRole::Instructor],
        b"appearance-instructor",
    )
    .await;
    let student_session = create_session(
        &store,
        tenant,
        student,
        vec![UserRole::Student],
        b"appearance-student",
    )
    .await;
    let outsider_session = create_session(
        &store,
        tenant,
        outsider,
        vec![UserRole::Instructor],
        b"appearance-outsider",
    )
    .await;
    let sysadmin_session = create_session(
        &store,
        tenant,
        sysadmin,
        vec![UserRole::Sysadmin],
        b"appearance-sysadmin",
    )
    .await;
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Appearance course".to_string(),
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
        .expect("course should persist");

    let initial = store
        .course_appearance(context, instructor_session, course)
        .await
        .expect("appearance read should run")
        .expect("instructor should see appearance");
    assert_eq!(initial.theme, CourseThemeId::Grass);
    assert_eq!(initial.revision, CourseAppearanceRevision::INITIAL);
    assert!(initial.banner.is_none());
    assert!(
        store
            .course_appearance(context, student_session, course)
            .await
            .expect("student read should run")
            .is_some()
    );
    assert!(
        store
            .course_appearance(context, outsider_session, course)
            .await
            .expect("outsider read should run")
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
                        theme: CourseThemeId::Forest,
                        banner: CourseBannerMutation::Remove,
                    },
                    promoted_object: None,
                },
            )
            .await,
        Err(StoreError::Forbidden)
    );

    let first_candidate = CourseBannerCandidateId::from_uuid(id(70_010));
    let first_banner = CourseBannerId::from_uuid(id(70_011));
    let first_bytes = b"first normalized banner";
    store
        .register_course_banner_candidate(
            context,
            instructor_session,
            course,
            candidate_command(tenant, course, first_candidate, first_banner, first_bytes),
        )
        .await
        .expect("first candidate should persist");
    let first_promotion = store
        .course_banner_promotion(context, instructor_session, course, first_candidate)
        .await
        .expect("candidate owner should resolve the hidden promotion identity");
    assert_eq!(first_promotion.candidate, first_candidate);
    assert_eq!(first_promotion.banner, first_banner);
    assert_eq!(first_promotion.sha256, Sha256Digest::compute(first_bytes));
    assert_eq!(first_promotion.size_bytes, first_bytes.len() as u64);
    assert_eq!(
        store
            .course_banner_promotion(context, outsider_session, course, first_candidate)
            .await,
        Err(StoreError::NotFound),
        "a candidate must not disclose its hidden banner identity to an outsider"
    );
    let first = store
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
                promoted_object: Some(promoted_object(tenant, course, first_banner, first_bytes)),
            },
        )
        .await
        .expect("first replacement should save");
    assert_eq!(first.revision.value(), 2);
    store
        .authorize_course_banner_delivery(context, student_session, first_banner)
        .await
        .expect("student should receive the exact current banner");

    let second_candidate = CourseBannerCandidateId::from_uuid(id(70_012));
    let second_banner = CourseBannerId::from_uuid(id(70_013));
    let second_bytes = b"second normalized banner";
    store
        .register_course_banner_candidate(
            context,
            instructor_session,
            course,
            candidate_command(
                tenant,
                course,
                second_candidate,
                second_banner,
                second_bytes,
            ),
        )
        .await
        .expect("second candidate should persist");
    let stale = SaveCourseAppearance {
        expected_revision: initial.revision,
        update: CourseAppearanceUpdate {
            theme: CourseThemeId::Desert,
            banner: CourseBannerMutation::Replace {
                candidate: second_candidate,
                alternative_text: CourseBannerAlternativeText::Decorative,
            },
        },
        promoted_object: Some(promoted_object(tenant, course, second_banner, second_bytes)),
    };
    assert_eq!(
        store
            .save_course_appearance(context, instructor_session, course, stale.clone())
            .await,
        Err(StoreError::Conflict),
        "stale CAS must retain but not select the copied object"
    );
    assert_eq!(
        store
            .authorize_course_banner_delivery(context, student_session, second_banner)
            .await,
        Err(StoreError::NotFound),
        "a candidate-owned copied object is not current"
    );
    let second = store
        .save_course_appearance(
            context,
            instructor_session,
            course,
            SaveCourseAppearance {
                expected_revision: first.revision,
                ..stale
            },
        )
        .await
        .expect("retry with the current revision should reuse the exact promoted object");
    assert_eq!(second.revision.value(), 3);
    assert_eq!(
        store
            .authorize_course_banner_delivery(context, student_session, first_banner)
            .await,
        Err(StoreError::NotFound),
        "a superseded banner must stop delivering immediately"
    );
    store
        .authorize_course_banner_delivery(context, student_session, second_banner)
        .await
        .expect("replacement should become the one current delivery");

    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Appearance course".to_string(),
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
        .expect("membership update should persist without resetting appearance");
    assert_eq!(
        store
            .save_course_appearance(
                context,
                instructor_session,
                course,
                SaveCourseAppearance {
                    expected_revision: second.revision,
                    update: CourseAppearanceUpdate {
                        theme: CourseThemeId::Beach,
                        banner: CourseBannerMutation::Remove,
                    },
                    promoted_object: None,
                },
            )
            .await,
        Err(StoreError::NotFound),
        "persisted membership removal must revoke a previously valid session"
    );
    assert_eq!(
        store
            .course_appearance(context, sysadmin_session, course)
            .await,
        Ok(None),
        "sysadmin status alone must not expose a course"
    );

    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(6_000))
        .expect("memory clock should advance");
    let claims = store
        .claim_course_banner_cleanup(
            context,
            CourseBannerCleanupBatch::new(10).expect("cleanup batch should validate"),
        )
        .await
        .expect("cleanup claims should run");
    assert_eq!(claims.len(), 2);
    let first_claim = claims
        .iter()
        .find(|claim| claim.candidate == first_candidate)
        .expect("superseded candidate should be selected");
    assert!(first_claim.candidate_object.is_some());
    assert!(first_claim.promoted_object.is_some());
    let second_claim = claims
        .iter()
        .find(|claim| claim.candidate == second_candidate)
        .expect("current candidate bytes should be selected");
    assert!(second_claim.candidate_object.is_some());
    assert!(second_claim.promoted_object.is_none());
    for claim in claims {
        assert!(
            store
                .complete_course_banner_cleanup(context, claim)
                .await
                .expect("cleanup completion should run")
        );
    }
    store
        .authorize_course_banner_delivery(context, student_session, second_banner)
        .await
        .expect("cleanup must preserve the exact current immutable banner");
    assert_eq!(
        store
            .authorize_course_banner_delivery(context, student_session, first_banner)
            .await,
        Err(StoreError::NotFound)
    );
    assert_eq!(
        ObjectCategory::CourseContent,
        promoted_object(tenant, course, second_banner, second_bytes).category
    );
}
