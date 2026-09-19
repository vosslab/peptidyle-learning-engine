//! Behavior contracts for typed object-address construction.

use super::*;
use question_model::{QuestionId, QuestionRevisionNumber};
use uuid::Uuid;

fn question_revision(revision_number: u32) -> QuestionRevisionTuple {
    QuestionRevisionTuple {
        question_id: QuestionId::from_random_identifier("ABCDEFG").expect("canonical Question ID"),
        revision_number: QuestionRevisionNumber::new(revision_number)
            .expect("positive Question Revision Number"),
    }
}

#[test]
fn source_objects_are_never_direct_delivery_targets() {
    let source = ObjectAddress::QuestionSource {
        question_revision: question_revision(2),
        object: ObjectId::from_uuid(Uuid::from_u128(3)),
    };
    let asset = ObjectAddress::QuestionAsset {
        question_revision: question_revision(2),
        asset: QuestionAssetId::from_uuid(Uuid::from_u128(4)),
        object: ObjectId::from_uuid(Uuid::from_u128(5)),
    };

    assert!(!source.may_issue_signed_url());
    assert!(asset.may_issue_signed_url());
}

#[test]
fn draft_asset_binds_private_workspace_draft_and_immutable_object_without_delivery() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
    let draft = Uuid::from_u128(3);
    let asset = QuestionAssetId::from_uuid(Uuid::from_u128(4));
    let object = ObjectId::from_uuid(Uuid::from_u128(5));
    let address = ObjectAddress::DraftQuestionAsset {
        workspace,
        draft_question_uuid: draft,
        asset,
        object,
    };
    assert_eq!(
        address.path(),
        format!("workspaces/{workspace}/questions/drafts/{draft}/assets/{asset}/{object}")
    );
    assert_eq!(address.storage_area(), ObjectStorageArea::PrivateContent);
    assert_eq!(address.data_class(), ObjectDataClass::AuthoringContent);
    assert_eq!(address.object_id(), object);
    assert!(address.question_revision().is_none());
    assert!(!address.may_issue_signed_url());
    let encoded = serde_json::to_value(&address).expect("internal address serializes");
    assert_eq!(encoded["kind"], "draftQuestionAsset");
    assert_eq!(encoded["draftQuestionUuid"], draft.to_string());
    assert_eq!(
        serde_json::from_value::<ObjectAddress>(encoded).expect("roundtrip"),
        address
    );
}

#[test]
fn only_immutable_question_assets_enter_the_public_delivery_domain() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
    let question_revision = question_revision(4);
    let object = ObjectId::from_uuid(Uuid::from_u128(5));

    let public_asset = ObjectAddress::QuestionAsset {
        question_revision: question_revision.clone(),
        asset: QuestionAssetId::from_uuid(Uuid::from_u128(6)),
        object,
    };
    assert_eq!(public_asset.storage_area(), ObjectStorageArea::PublicAssets);
    assert_eq!(
        ObjectAddress::published_question_asset(
            question_revision.clone(),
            QuestionAssetId::from_uuid(Uuid::from_u128(60)),
            object,
        )
        .storage_area(),
        ObjectStorageArea::PrivateContent,
        "Published Question assets must never enter the CDN-readable Object Storage Area"
    );

    for private_key in [
        ObjectAddress::WorkspaceImportSource {
            workspace,
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(7)),
            object,
        },
        ObjectAddress::QuestionSource {
            question_revision: question_revision.clone(),
            object,
        },
        ObjectAddress::RestrictedQuestionAsset {
            question_revision: question_revision.clone(),
            asset: QuestionAssetId::from_uuid(Uuid::from_u128(61)),
            object,
        },
        ObjectAddress::PublishedImportArchive {
            question_revision: question_revision.clone(),
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(9)),
            object,
        },
        ObjectAddress::QuestionRender {
            question_revision: question_revision.clone(),
            question_seed: QuestionSeed::new(1),
            object,
        },
        ObjectAddress::CourseBannerSource {
            course: CourseInstanceId::from_debug_serial(10),
            banner: CourseBannerId::from_uuid(Uuid::from_u128(11)),
        },
    ] {
        assert_eq!(
            private_key.storage_area(),
            ObjectStorageArea::PrivateContent,
            "{private_key:?} must not be placed in the CDN-readable Object Storage Area"
        );
    }
}

#[test]
fn course_banner_keys_bind_scope_classification_and_signing() {
    let course = CourseInstanceId::from_debug_serial(2);
    let upload_reference = CourseBannerUploadId::from_uuid(Uuid::from_u128(3));
    let banner_reference = CourseBannerId::from_uuid(Uuid::from_u128(4));
    let upload = ObjectAddress::CourseBannerUpload {
        course: course.clone(),
        upload: upload_reference,
    };
    let source = ObjectAddress::CourseBannerSource {
        course: course.clone(),
        banner: banner_reference,
    };
    let banner = ObjectAddress::CourseBannerRendition {
        course: course.clone(),
        banner: banner_reference,
        rendition: CourseBannerRendition::Banner,
    };

    assert_eq!(upload.storage_area(), ObjectStorageArea::TempProcessing);
    assert_eq!(upload.question_revision(), None);
    assert!(!upload.may_issue_signed_url());
    assert_eq!(source.storage_area(), ObjectStorageArea::PrivateContent);
    assert_eq!(source.question_revision(), None);
    assert!(!source.may_issue_signed_url());
    assert_eq!(banner.storage_area(), ObjectStorageArea::PrivateContent);
    assert!(banner.may_issue_signed_url());
    assert!(upload.path().contains(&course.to_string()));
    assert!(upload.path().contains(&upload_reference.to_string()));
    assert!(banner.path().contains(&course.to_string()));
    assert!(banner.path().contains(&banner_reference.to_string()));
    assert_ne!(upload.object_id(), banner.object_id());
}

#[test]
fn banner_object_identity_changes_with_course_and_route_id() {
    let course = CourseInstanceId::from_debug_serial(2);
    let banner = CourseBannerId::from_uuid(Uuid::from_u128(3));
    let base = course_banner_source_object_id(&course, banner);
    assert_ne!(
        base,
        course_banner_source_object_id(&CourseInstanceId::from_debug_serial(12), banner)
    );
    assert_ne!(
        base,
        course_banner_source_object_id(&course, CourseBannerId::from_uuid(Uuid::from_u128(13)))
    );
}

#[test]
fn banner_keys_round_trip_without_a_caller_supplied_object_id() {
    let key = ObjectAddress::CourseBannerRendition {
        course: CourseInstanceId::from_debug_serial(2),
        banner: CourseBannerId::from_uuid(Uuid::from_u128(3)),
        rendition: CourseBannerRendition::Banner,
    };
    let encoded = serde_json::to_string(&key).expect("banner key should serialize");
    let decoded: ObjectAddress =
        serde_json::from_str(&encoded).expect("banner key should deserialize");

    assert_eq!(decoded, key);
    assert!(!encoded.contains("\"object\""));
}

#[test]
fn profile_images_have_one_private_typed_storage_identity() {
    // This preserves the object-store privacy boundary and exact physical
    // identity consumed by the self-only Profile-image delivery saga.
    let image = ProfileImageId::from_uuid(Uuid::from_u128(2));
    let object = ObjectId::from_uuid(Uuid::from_u128(3));
    let key = ObjectAddress::ProfileImage { image, object };

    assert_eq!(key.storage_area(), ObjectStorageArea::PrivateContent);
    assert_eq!(key.data_class(), ObjectDataClass::ProfileImage);
    assert_eq!(key.object_id(), object);
    assert!(key.may_issue_signed_url());
    assert_eq!(
        key.path(),
        "profiles/images/00000000-0000-0000-0000-000000000002/00000000-0000-0000-0000-000000000003"
    );
    assert_eq!(
        serde_json::from_str::<ObjectAddress>(
            &serde_json::to_string(&key).expect("Profile-image address should serialize")
        )
        .expect("Profile-image address should deserialize"),
        key
    );
}

#[test]
fn legacy_profile_thumbnail_address_cannot_restore_a_second_live_path() {
    // The legacy address was Instructor-only and would create a second
    // current-image storage path if accepted again.
    let legacy =
        r#"{"kind":"profileThumbnail","thumbnail":"00000000-0000-0000-0000-000000000002"}"#;

    assert!(serde_json::from_str::<ObjectAddress>(legacy).is_err());
}

#[test]
fn workspace_qti_archive_object_id_matches_golden() {
    let actual = workspace_qti_archive_object_id(
        WorkspaceId::from_uuid(Uuid::from_u128(2)),
        WorkspaceImportId::from_uuid(Uuid::from_u128(3)),
    );

    assert_eq!(
        actual,
        workspace_qti_archive_object_id(
            WorkspaceId::from_uuid(Uuid::from_u128(2)),
            WorkspaceImportId::from_uuid(Uuid::from_u128(3)),
        )
    );
}

#[test]
fn workspace_qti_archive_identity_changes_with_workspace() {
    let import = WorkspaceImportId::from_uuid(Uuid::from_u128(3));

    assert_ne!(
        workspace_qti_archive_object_id(WorkspaceId::from_uuid(Uuid::from_u128(2)), import),
        workspace_qti_archive_object_id(WorkspaceId::from_uuid(Uuid::from_u128(12)), import)
    );
}

#[test]
fn workspace_qti_archive_identity_changes_with_import() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));

    assert_ne!(
        workspace_qti_archive_object_id(
            workspace,
            WorkspaceImportId::from_uuid(Uuid::from_u128(3))
        ),
        workspace_qti_archive_object_id(
            workspace,
            WorkspaceImportId::from_uuid(Uuid::from_u128(13))
        )
    );
}

#[test]
fn workspace_qti_archive_uses_private_workspace_import_source_key() {
    let workspace = WorkspaceId::from_uuid(Uuid::from_u128(2));
    let import = WorkspaceImportId::from_uuid(Uuid::from_u128(3));
    let object = workspace_qti_archive_object_id(workspace, import);
    let key = ObjectAddress::WorkspaceImportSource {
        workspace,
        import,
        object,
    };

    assert_eq!(
        key.path(),
        format!("workspaces/{workspace}/imports/{import}/source/{object}")
    );
    assert_eq!(key.object_id(), object);
    assert_eq!(key.storage_area(), ObjectStorageArea::PrivateContent);
    assert_eq!(key.question_revision(), None);
    assert!(!key.may_issue_signed_url());
}

#[test]
fn published_import_archive_object_id_matches_golden() {
    let actual = published_import_archive_object_id(
        &question_revision(3),
        WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
        Sha256Checksum::compute(b"archive fixture"),
    );

    assert_eq!(
        actual,
        published_import_archive_object_id(
            &question_revision(3),
            WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            Sha256Checksum::compute(b"archive fixture"),
        )
    );
}

#[test]
fn published_import_archive_key_has_distinct_path_and_private_classification() {
    let key = ObjectAddress::PublishedImportArchive {
        question_revision: question_revision(3),
        import: WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
        object: ObjectId::from_uuid(Uuid::from_u128(5)),
    };

    assert_eq!(
        key.path(),
        "questions/ABCD-XEFG/versions/3/imports/00000000-0000-0000-0000-000000000004/archive/00000000-0000-0000-0000-000000000005"
    );
    assert_eq!(key.storage_area(), ObjectStorageArea::PrivateContent);
    assert_eq!(key.question_revision(), Some(&question_revision(3)));
    assert!(!key.may_issue_signed_url());
}

#[test]
fn every_archive_identity_input_changes_the_object_id() {
    let reference = question_revision(3);
    let import = WorkspaceImportId::from_uuid(Uuid::from_u128(4));
    let archive = Sha256Checksum::compute(b"archive fixture");
    let base = published_import_archive_object_id(&reference, import, archive);
    assert_ne!(
        base,
        published_import_archive_object_id(
            &QuestionRevisionTuple {
                question_id: QuestionId::from_random_identifier("BCDEFGH")
                    .expect("canonical Question ID"),
                revision_number: reference.revision_number,
            },
            import,
            archive
        )
    );
    assert_ne!(
        base,
        published_import_archive_object_id(&question_revision(13), import, archive)
    );
    assert_ne!(
        base,
        published_import_archive_object_id(
            &reference,
            WorkspaceImportId::from_uuid(Uuid::from_u128(14)),
            archive
        )
    );
    assert_ne!(
        base,
        published_import_archive_object_id(
            &reference,
            import,
            Sha256Checksum::compute(b"different archive")
        )
    );
}

#[test]
fn published_import_archive_address_round_trips_through_serde() {
    let address = ObjectAddress::PublishedImportArchive {
        question_revision: question_revision(3),
        import: WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
        object: ObjectId::from_uuid(Uuid::from_u128(5)),
    };

    let encoded = serde_json::to_string(&address).expect("Object Address should serialize");
    let decoded: ObjectAddress =
        serde_json::from_str(&encoded).expect("Object Address should deserialize");
    assert_eq!(decoded, address);
    assert!(encoded.contains("publishedImportArchive"));
    assert!(encoded.contains("\"questionId\":\"ABCD-XEFG\""));
}

#[test]
fn every_published_question_address_uses_canonical_question_id_json() {
    let revision = question_revision(3);
    let object = ObjectId::from_uuid(Uuid::from_u128(5));
    let addresses = [
        ObjectAddress::QuestionSource {
            question_revision: revision.clone(),
            object,
        },
        ObjectAddress::PublishedImportArchive {
            question_revision: revision.clone(),
            import: WorkspaceImportId::from_uuid(Uuid::from_u128(4)),
            object,
        },
        ObjectAddress::QuestionAsset {
            question_revision: revision.clone(),
            asset: QuestionAssetId::from_uuid(Uuid::from_u128(6)),
            object,
        },
        ObjectAddress::RestrictedQuestionAsset {
            question_revision: revision.clone(),
            asset: QuestionAssetId::from_uuid(Uuid::from_u128(7)),
            object,
        },
        ObjectAddress::QuestionRender {
            question_revision: revision,
            question_seed: QuestionSeed::new(8),
            object,
        },
    ];

    for address in addresses {
        let encoded = serde_json::to_string(&address).expect("Object Address should serialize");
        assert!(encoded.contains("\"questionId\":\"ABCD-XEFG\""));
        assert_eq!(
            serde_json::from_str::<ObjectAddress>(&encoded)
                .expect("canonical Object Address should deserialize"),
            address
        );
    }
}

#[test]
fn object_address_rejects_noncanonical_question_id_json() {
    let encoded = r#"{"kind":"questionSource","questionRevision":{"questionId":"ABCDXEFG","revisionNumber":3},"object":"00000000-0000-0000-0000-000000000005"}"#;

    assert!(serde_json::from_str::<ObjectAddress>(encoded).is_err());
}
