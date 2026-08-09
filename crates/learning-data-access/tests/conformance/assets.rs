use super::*;

pub(super) fn source_artifact(
    reference: ProblemVersionRef,
    backend: QuestionBackend,
    object: ObjectId,
) -> PublishedSourceArtifact {
    PublishedSourceArtifact {
        reference,
        backend,
        object: object_record(
            ObjectKey::ProblemSource {
                problem: reference.problem,
                version: reference.version,
                object,
            },
            b"immutable source fixture",
            1_000,
        ),
    }
}

async fn exercise_asset_store<S>(store: &S)
where
    S: Store + CatalogStore + AssetStore,
{
    let tenant = TenantId::from_uuid(uuid(401));
    let foreign_tenant = TenantId::from_uuid(uuid(402));
    let context = TenantContext::from_authenticated_session(tenant);
    let foreign_context = TenantContext::from_authenticated_session(foreign_tenant);
    let publisher = UserId::from_uuid(uuid(403));
    let student = UserId::from_uuid(uuid(404));
    let stranger = UserId::from_uuid(uuid(405));
    let course = CourseId::from_uuid(uuid(405_001));
    store
        .upsert_course(
            context,
            CourseRecord {
                id: course,
                tenant,
                title: "Asset delivery course".to_string(),
                members: vec![
                    CourseMembership {
                        user: publisher,
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
        .expect("asset delivery course");
    let public_problem = ProblemId::from_uuid(uuid(406));
    let public_version = VersionId::from_uuid(uuid(407));
    let institution_problem = ProblemId::from_uuid(uuid(408));
    let institution_version = VersionId::from_uuid(uuid(409));

    for (problem, version, workspace, scope) in [
        (
            public_problem,
            public_version,
            WorkspaceId::from_uuid(uuid(410)),
            PublicationScope::Public,
        ),
        (
            institution_problem,
            institution_version,
            WorkspaceId::from_uuid(uuid(411)),
            PublicationScope::Institution,
        ),
    ] {
        let draft = DraftRecord {
            tenant,
            question: draft_question(workspace),
            revises: None,
            derived_from: None,
        };
        let saved_draft = store
            .upsert_draft(context, publisher, None, draft.clone())
            .await
            .expect("asset fixture draft should save");
        store
            .publish_draft(
                context,
                publisher,
                PublishDraftCommand {
                    expected_draft: draft,
                    expected_revision: saved_draft.revision,
                    publication: ProblemVersionRef { problem, version },
                    published_source: published_source(),
                    source_artifact: None,
                    qti_promotion: None,
                    flat_question_promotion: None,
                    publisher,
                    scope,
                    capabilities: BackendCapabilities::from_iter([Capability::ServerGrading]),
                },
            )
            .await
            .expect("asset fixture should publish");
    }

    let public_asset = AssetId::from_uuid(uuid(412));
    let public_object = ObjectId::from_uuid(uuid(413));
    let public_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(public_asset),
        object: object_record(
            ObjectKey::ProblemAsset {
                problem: public_problem,
                version: public_version,
                asset: public_asset,
                object: public_object,
            },
            b"public",
            1_000,
        ),
        scope: AssetDeliveryScope::Catalog {
            asset: public_asset,
            reference: ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        },
    };
    let second_public_asset = AssetId::from_uuid(uuid(419));
    let second_public_object = ObjectId::from_uuid(uuid(420));
    let second_public_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(second_public_asset),
        object: object_record(
            ObjectKey::ProblemAsset {
                problem: public_problem,
                version: public_version,
                asset: second_public_asset,
                object: second_public_object,
            },
            b"second public asset",
            1_000,
        ),
        scope: AssetDeliveryScope::Catalog {
            asset: second_public_asset,
            reference: ProblemVersionRef {
                problem: public_problem,
                version: public_version,
            },
        },
    };
    let institution_asset = AssetId::from_uuid(uuid(414));
    let institution_object = ObjectId::from_uuid(uuid(415));
    let institution_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_asset(institution_asset),
        object: object_record(
            ObjectKey::ProblemAsset {
                problem: institution_problem,
                version: institution_version,
                asset: institution_asset,
                object: institution_object,
            },
            b"institution",
            1_000,
        ),
        scope: AssetDeliveryScope::Catalog {
            asset: institution_asset,
            reference: ProblemVersionRef {
                problem: institution_problem,
                version: institution_version,
            },
        },
    };
    let student_object = ObjectId::from_uuid(uuid(416));
    let student_delivery = AssetDeliveryRecord {
        id: AssetDeliveryId::from_object(student_object),
        object: object_record(
            ObjectKey::StudentRecord {
                tenant,
                object: student_object,
            },
            b"student export",
            1_000,
        ),
        scope: AssetDeliveryScope::StudentRecord {
            tenant,
            course,
            authorized_users: vec![student],
        },
    };

    for record in [
        public_delivery.clone(),
        second_public_delivery,
        institution_delivery.clone(),
        student_delivery.clone(),
    ] {
        store
            .register_asset_delivery(context, record)
            .await
            .expect("valid asset delivery should register");
    }
    assert_eq!(
        store
            .register_asset_delivery(context, public_delivery.clone())
            .await,
        Err(StoreError::AlreadyExists),
        "delivery records are immutable"
    );

    assert_eq!(
        store
            .get_public_asset_delivery(public_delivery.id)
            .await
            .expect("public lookup should run"),
        Some(public_delivery.clone())
    );
    assert_eq!(
        store
            .get_public_asset_delivery(institution_delivery.id)
            .await
            .expect("institution lookup should run"),
        None
    );
    assert_eq!(
        store
            .get_public_asset_delivery(student_delivery.id)
            .await
            .expect("student-record lookup should run"),
        None
    );

    let public_reference = ProblemVersionRef {
        problem: public_problem,
        version: public_version,
    };
    let institution_reference = ProblemVersionRef {
        problem: institution_problem,
        version: institution_version,
    };
    let public_bindings = store
        .catalog_asset_bindings(context, public_reference)
        .await
        .expect("catalog asset bindings should resolve");
    assert_eq!(
        public_bindings,
        vec![
            learning_data_access::CatalogAssetBinding {
                asset: public_asset,
                object: public_object,
            },
            learning_data_access::CatalogAssetBinding {
                asset: second_public_asset,
                object: second_public_object,
            },
        ],
        "the resolver must select only the exact published version"
    );
    assert_eq!(
        store
            .catalog_asset_bindings(context, public_reference)
            .await
            .expect("repeat catalog asset resolution should run"),
        public_bindings,
        "catalog asset bindings must be deterministic"
    );
    assert_eq!(
        store
            .catalog_asset_bindings(context, institution_reference)
            .await
            .expect("institution catalog asset resolution should run"),
        vec![learning_data_access::CatalogAssetBinding {
            asset: institution_asset,
            object: institution_object,
        }],
        "student records and another catalog version must not leak into the result"
    );
    assert!(
        store
            .catalog_asset_bindings(foreign_context, institution_reference)
            .await
            .expect("foreign catalog asset resolution should run")
            .is_empty(),
        "a foreign tenant must not learn institution catalog asset bindings"
    );
    assert!(
        store
            .catalog_asset_bindings(
                context,
                ProblemVersionRef {
                    problem: public_problem,
                    version: VersionId::from_uuid(uuid(418)),
                },
            )
            .await
            .expect("unknown exact version lookup should run")
            .is_empty(),
        "an absent version may resolve to an empty visible result"
    );

    let institution_authorized = store
        .authorize_asset_delivery(context, student, institution_delivery.id)
        .await
        .expect("institution asset should be visible in its tenant");
    assert_eq!(institution_authorized.record, institution_delivery);
    assert_eq!(
        store
            .authorize_asset_delivery(foreign_context, student, institution_delivery.id)
            .await,
        Err(StoreError::NotFound),
        "institution assets must not cross tenant grants"
    );
    let student_authorized = store
        .authorize_asset_delivery(context, student, student_delivery.id)
        .await
        .expect("named student should receive their record");
    assert_eq!(student_authorized.record, student_delivery);
    assert_eq!(
        store
            .authorize_asset_delivery(context, stranger, student_authorized.record.id)
            .await,
        Err(StoreError::NotFound),
        "unauthorized identities must not learn that a student record exists"
    );
    assert_eq!(
        store
            .authorize_asset_delivery(foreign_context, student, student_authorized.record.id,)
            .await,
        Err(StoreError::NotFound),
        "RLS tenant context must protect student records"
    );

    let temporary = ObjectId::from_uuid(uuid(417));
    let invalid = AssetDeliveryRecord {
        id: AssetDeliveryId::from_object(temporary),
        object: object_record(
            ObjectKey::Temporary { object: temporary },
            b"temporary",
            1_000,
        ),
        scope: AssetDeliveryScope::StudentRecord {
            tenant,
            course,
            authorized_users: vec![student],
        },
    };
    assert!(matches!(
        store.register_asset_delivery(context, invalid).await,
        Err(StoreError::InvalidRecord(_))
    ));
}

#[tokio::test]
async fn memory_asset_store_conforms_and_records_protected_access() {
    let store = MemoryStore::default();
    store
        .set_authoritative_time(ActivityTimestamp::from_unix_millis(7_000))
        .expect("memory clock should be writable");
    exercise_asset_store(&store).await;
    let events = store
        .asset_access_events()
        .expect("memory audit events should be readable");
    assert_eq!(events.len(), 2, "only authorized protected requests log");
    assert!(
        events
            .iter()
            .all(|event| event.occurred_at == ActivityTimestamp::from_unix_millis(7_000))
    );
}
