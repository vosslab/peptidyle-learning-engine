use adapter_native::flat_question::FlatQuestionDocument;
use objects::{ObjectKey, ObjectRecord, published_import_archive_object_id};
use question_model::{
    ActivityTimestamp, ObjectId, ProblemId, TenantId, UserId, VersionId, WorkspaceId,
    WorkspaceImportId,
};
use uuid::Uuid;

use super::*;

const CONVERSION_VERSION: &str = "ple-qti-profile-flat-conversion/v1";
const FLAT_SOURCE: &str = r#"{"format":"pleFlatQuestion","version":2,"title":"Favorite color","prompt":"What is my favorite color?","response":{"kind":"singleChoice","choices":[{"id":"blue","text":"Blue"},{"id":"red","text":"Red"}],"correctChoice":"blue"},"feedback":{},"points":1.0,"attemptPolicy":{"maxAttempts":null,"feedback":"immediateFull"},"timingPolicy":{"kind":"untimed"},"tags":[],"taxonomy":[],"license":{"kind":"allRightsReserved"},"language":"en-US"}"#;

fn uuid(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

fn tenant() -> TenantId {
    TenantId::from_uuid(uuid(1))
}

fn workspace() -> WorkspaceId {
    WorkspaceId::from_uuid(uuid(2))
}

fn import_id() -> WorkspaceImportId {
    WorkspaceImportId::from_uuid(uuid(3))
}

fn actor() -> UserId {
    UserId::from_uuid(uuid(4))
}

fn import_ref() -> QtiImportRef {
    QtiImportRef {
        tenant: tenant(),
        workspace: workspace(),
        import: import_id(),
    }
}

fn record(key: ObjectKey, bytes: &[u8], media_type: &str) -> ObjectRecord {
    ObjectRecord {
        id: key.object_id(),
        bucket: key.bucket(),
        category: key.category(),
        version: key.version_id(),
        key,
        sha256: Sha256Digest::compute(bytes),
        size_bytes: bytes.len() as u64,
        media_type: media_type.to_string(),
        license: "allRightsReserved".to_string(),
        provenance: "fixture provenance".to_string(),
        created_at: ActivityTimestamp::from_unix_millis(10),
    }
}

fn archive() -> ObjectRecord {
    record(
        ObjectKey::WorkspaceSource {
            tenant: tenant(),
            workspace: workspace(),
            import: import_id(),
            object: ObjectId::from_uuid(uuid(5)),
        },
        b"PK\x03\x04fixture",
        QTI_PROFILE_ARCHIVE_MEDIA_TYPE,
    )
}

fn compiled() -> (DraftRecord, FlatQuestionGradingPayload, Vec<u8>) {
    let document =
        FlatQuestionDocument::parse(FLAT_SOURCE.as_bytes()).expect("flat fixture parses");
    let canonical = document
        .canonical_bytes()
        .expect("flat fixture canonicalizes");
    let (question, private) = document
        .compile(workspace())
        .expect("flat fixture compiles")
        .into_parts();
    (
        DraftRecord {
            tenant: tenant(),
            question,
            revises: None,
            derived_from: None,
        },
        FlatQuestionGradingPayload::from_private(&private).expect("private fixture persists"),
        canonical,
    )
}

fn source(canonical: &[u8]) -> ObjectRecord {
    record(
        ObjectKey::WorkspaceQuestionSource {
            tenant: tenant(),
            workspace: workspace(),
            object: ObjectId::from_uuid(uuid(6)),
        },
        canonical,
        crate::flat_question::FLAT_QUESTION_MEDIA_TYPE,
    )
}

fn choice_map() -> FlatImportChoiceMapPayload {
    FlatImportChoiceMapPayload::from_canonical_bytes(
        br#"{"schema":"ple-qti-private-choice-map/v1","choices":[["vendor-blue","blue"],["vendor-red","red"]]}"#.to_vec(),
    )
    .expect("choice-map fixture is bounded")
}

fn digests(map: &FlatImportChoiceMapPayload) -> FlatImportIntegrityDigests {
    FlatImportIntegrityDigests {
        normalized_item_sha256: Sha256Digest::compute(b"normalized"),
        profile_report_sha256: Sha256Digest::compute(b"report"),
        public_mapping_sha256: Sha256Digest::compute(b"public"),
        private_mapping_sha256: Sha256Digest::compute(b"private"),
        mapping_sha256: Sha256Digest::compute(b"combined"),
        warning_sha256: Sha256Digest::compute(b"warnings"),
        choice_map_sha256: map.sha256(),
    }
}

fn origin(canonical_sha256: Sha256Digest) -> WorkspaceFlatImportOrigin {
    let map = choice_map();
    WorkspaceFlatImportOrigin::new(
        import_ref(),
        "canvas-item-1".to_string(),
        PersistedFlatImportProfile::CanvasQti12V1,
        FlatImportConversionVersion::new(CONVERSION_VERSION).expect("version fixture"),
        archive(),
        digests(&map),
        canonical_sha256,
        actor(),
        ActivityTimestamp::from_unix_millis(20),
        map,
    )
    .expect("origin fixture is valid")
}

fn import_evidence(identifier: String) -> QtiProfileImportEvidence {
    let map = choice_map();
    QtiProfileImportEvidence::new(
        import_ref(),
        identifier,
        PersistedFlatImportProfile::CanvasQti12V1,
        digests(&map),
    )
    .expect("profile import evidence fixture is valid")
}

#[test]
fn profile_import_evidence_enforces_unicode_scalar_identifier_bounds() {
    assert!(
        QtiProfileImportEvidence::new(
            import_ref(),
            String::new(),
            PersistedFlatImportProfile::CanvasQti12V1,
            digests(&choice_map()),
        )
        .is_err()
    );
    assert!(
        QtiProfileImportEvidence::new(
            import_ref(),
            "\u{2003}".to_string(),
            PersistedFlatImportProfile::CanvasQti12V1,
            digests(&choice_map()),
        )
        .is_err()
    );

    let boundary = "\u{00e9}".repeat(MAX_SOURCE_ITEM_IDENTIFIER_CHARS);
    let evidence = import_evidence(boundary.clone());
    assert_eq!(
        evidence.persistence_parts().source_item_identifier,
        boundary
    );
    assert!(
        QtiProfileImportEvidence::new(
            import_ref(),
            format!("{boundary}\u{00e9}"),
            PersistedFlatImportProfile::CanvasQti12V1,
            digests(&choice_map()),
        )
        .is_err()
    );
}

#[test]
fn profile_import_evidence_exact_replay_preserves_typed_identity() {
    let first = import_evidence("canvas-item-1".to_string());
    let replay = import_evidence("canvas-item-1".to_string());
    assert!(first == replay);

    let persisted = first.persistence_parts();
    assert_eq!(persisted.import, import_ref());
    assert_eq!(persisted.profile, PersistedFlatImportProfile::CanvasQti12V1);
    assert_eq!(persisted.digests, digests(&choice_map()));
}

#[test]
fn profile_tuple_and_conversion_version_are_closed_and_stable() {
    let canvas = PersistedFlatImportProfile::CanvasQti12V1;
    assert_eq!(
        (
            canvas.profile_id(),
            canvas.profile_version(),
            canvas.mapping_version()
        ),
        ("canvas-qti-1.2-static-single-choice/v1", "v1", "v1")
    );
    assert_eq!(
        PersistedFlatImportProfile::from_stored(
            canvas.profile_id(),
            canvas.profile_version(),
            canvas.mapping_version()
        ),
        Ok(canvas)
    );
    assert!(PersistedFlatImportProfile::from_stored(canvas.profile_id(), "v2", "v1").is_err());

    let version = FlatImportConversionVersion::new(CONVERSION_VERSION).expect("valid version");
    assert_eq!(version.as_str(), CONVERSION_VERSION);
    assert!(FlatImportConversionVersion::new("Uppercase/v1").is_err());
    assert!(FlatImportConversionVersion::new("../v1").is_err());
}

#[test]
fn opaque_choice_map_is_bounded_checksummed_and_redacted() {
    let map = choice_map();
    assert_eq!(map.sha256(), Sha256Digest::compute(map.bytes()));
    assert!(format!("{map:?}").contains("[redacted]"));
    assert!(!format!("{map:?}").contains("vendor-blue"));
    assert!(FlatImportChoiceMapPayload::from_canonical_bytes(Vec::new()).is_err());
    assert!(
        FlatImportChoiceMapPayload::from_canonical_bytes(vec![
            0;
            MAX_FLAT_IMPORT_CHOICE_MAP_BYTES + 1
        ])
        .is_err()
    );
}

#[test]
fn origin_refuses_hostile_identity_archive_and_payload_mismatch() {
    let canonical = Sha256Digest::compute(FLAT_SOURCE.as_bytes());
    let map = choice_map();
    let mut wrong = digests(&map);
    wrong.choice_map_sha256 = Sha256Digest::compute(b"wrong map");
    assert!(
        WorkspaceFlatImportOrigin::new(
            import_ref(),
            "item".to_string(),
            PersistedFlatImportProfile::CanvasQti12V1,
            FlatImportConversionVersion::new(CONVERSION_VERSION).expect("version"),
            archive(),
            wrong,
            canonical,
            actor(),
            ActivityTimestamp::from_unix_millis(20),
            map,
        )
        .is_err()
    );
    let mut bad_archive = archive();
    bad_archive.media_type = "text/xml".to_string();
    let map = choice_map();
    assert!(
        WorkspaceFlatImportOrigin::new(
            import_ref(),
            " ".to_string(),
            PersistedFlatImportProfile::CanvasQti12V1,
            FlatImportConversionVersion::new(CONVERSION_VERSION).expect("version"),
            bad_archive,
            digests(&map),
            canonical,
            actor(),
            ActivityTimestamp::from_unix_millis(20),
            map,
        )
        .is_err()
    );
}

#[test]
fn conversion_command_revalidates_source_origin_and_private_binding() {
    let (draft, grading, canonical) = compiled();
    let source_record = source(&canonical);
    let current_origin = origin(source_record.sha256);
    let command = QtiProfileFlatConversionCommand::new(
        None,
        draft.clone(),
        source_record.clone(),
        source_record.sha256.to_string(),
        grading.public_binding_sha256().to_string(),
        grading.clone(),
        current_origin.clone(),
    )
    .expect("exact conversion command validates");
    assert!(format!("{command:?}").contains("[redacted]"));
    assert!(!format!("{command:?}").contains("canvas-item-1"));

    assert!(
        QtiProfileFlatConversionCommand::new(
            None,
            draft,
            source_record,
            Sha256Digest::compute(b"other").to_string(),
            grading.public_binding_sha256().to_string(),
            grading,
            current_origin,
        )
        .is_err()
    );

    let (draft, grading, canonical) = compiled();
    let valid_source = source(&canonical);
    let origin = origin(valid_source.sha256);
    let mut invalid_sources = Vec::new();
    let mut wrong_media = valid_source.clone();
    wrong_media.media_type = "application/json".to_string();
    invalid_sources.push(wrong_media);
    let mut empty = valid_source.clone();
    empty.size_bytes = 0;
    invalid_sources.push(empty);
    let mut oversized = valid_source.clone();
    oversized.size_bytes = (crate::flat_question::MAX_FLAT_QUESTION_PAYLOAD_BYTES + 1) as u64;
    invalid_sources.push(oversized);
    let mut missing_license = valid_source.clone();
    missing_license.license.clear();
    invalid_sources.push(missing_license);
    let mut missing_provenance = valid_source;
    missing_provenance.provenance.clear();
    invalid_sources.push(missing_provenance);
    for invalid_source in invalid_sources {
        assert!(
            QtiProfileFlatConversionCommand::new(
                None,
                draft.clone(),
                invalid_source.clone(),
                invalid_source.sha256.to_string(),
                grading.public_binding_sha256().to_string(),
                grading.clone(),
                origin.clone(),
            )
            .is_err()
        );
    }
}

#[test]
fn publication_candidate_is_deterministic_and_copies_only_current_origin() {
    let (_, _, canonical) = compiled();
    let current = origin(Sha256Digest::compute(&canonical));
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(7)),
        version: VersionId::from_uuid(uuid(8)),
    };
    let published_object = published_import_archive_object_id(
        tenant(),
        reference.problem,
        reference.version,
        import_id(),
        current.source_archive().sha256,
    );
    let published = record(
        ObjectKey::PublishedImportArchive {
            tenant: tenant(),
            problem: reference.problem,
            version: reference.version,
            import: import_id(),
            object: published_object,
        },
        b"PK\x03\x04fixture",
        QTI_PROFILE_ARCHIVE_MEDIA_TYPE,
    );
    let promotion = FlatImportPublicationPromotion::new(&current, reference, published.clone())
        .expect("exact candidate validates");
    assert!(promotion.expected_current_origin() == &current.identity());
    assert_eq!(promotion.published_archive(), &published);
    assert!(!format!("{promotion:?}").contains("canvas-item-1"));

    let immutable = PublishedFlatImportOrigin::from_current(&current, reference, published)
        .expect("Store can copy the locked current origin");
    assert_eq!(immutable.owner_tenant(), tenant());
    assert_eq!(immutable.reference(), reference);
    assert_eq!(
        immutable.published_archive().sha256,
        current.source_archive().sha256
    );
    assert_eq!(immutable.import(), import_id());
    let persisted = immutable.persistence_parts();
    assert_eq!(persisted.source_item_identifier, "canvas-item-1");
    assert_eq!(persisted.profile, PersistedFlatImportProfile::CanvasQti12V1);
    assert_eq!(persisted.conversion_version, CONVERSION_VERSION);
    assert_eq!(
        persisted.choice_map.sha256(),
        persisted.digests.choice_map_sha256
    );
}

#[test]
fn publication_validation_defers_optional_origin_decision_to_locked_backend_state() {
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(7)),
        version: VersionId::from_uuid(uuid(8)),
    };
    let context = TenantContext::from_authenticated_session(tenant());
    crate::publication_validation::validate_flat_import_publication_promotion(
        context, reference, None,
    )
    .expect("manual flat publication may omit imported lineage");

    let (_, _, canonical) = compiled();
    let current = origin(Sha256Digest::compute(&canonical));
    let published_object = published_import_archive_object_id(
        tenant(),
        reference.problem,
        reference.version,
        import_id(),
        current.source_archive().sha256,
    );
    let published = record(
        ObjectKey::PublishedImportArchive {
            tenant: tenant(),
            problem: reference.problem,
            version: reference.version,
            import: import_id(),
            object: published_object,
        },
        b"PK\x03\x04fixture",
        QTI_PROFILE_ARCHIVE_MEDIA_TYPE,
    );
    let promotion = FlatImportPublicationPromotion::new(&current, reference, published)
        .expect("exact candidate validates");
    crate::publication_validation::validate_flat_import_publication_promotion(
        context,
        reference,
        Some(&promotion),
    )
    .expect("structural origin reaches backend locked-origin verification");

    let foreign_context = TenantContext::from_authenticated_session(TenantId::from_uuid(uuid(9)));
    assert!(
        crate::publication_validation::validate_flat_import_publication_promotion(
            foreign_context,
            reference,
            Some(&promotion),
        )
        .is_err(),
        "structural validation refuses another tenant before backend verification"
    );
}

#[test]
fn published_archive_annotations_match_sql_character_bounds() {
    let (_, _, canonical) = compiled();
    let current = origin(Sha256Digest::compute(&canonical));
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid(7)),
        version: VersionId::from_uuid(uuid(8)),
    };
    let published_object = published_import_archive_object_id(
        tenant(),
        reference.problem,
        reference.version,
        import_id(),
        current.source_archive().sha256,
    );
    let published = record(
        ObjectKey::PublishedImportArchive {
            tenant: tenant(),
            problem: reference.problem,
            version: reference.version,
            import: import_id(),
            object: published_object,
        },
        b"PK\x03\x04fixture",
        QTI_PROFILE_ARCHIVE_MEDIA_TYPE,
    );
    let mut boundary = published.clone();
    boundary.license = format!(" {} ", "\u{03bb}".repeat(512));
    boundary.provenance = format!(" {} ", "\u{03bb}".repeat(2_048));
    let promotion = FlatImportPublicationPromotion::new(&current, reference, boundary)
        .expect("published archive accepts SQL character-length boundaries");
    crate::publication_validation::validate_flat_import_publication_promotion(
        TenantContext::from_authenticated_session(tenant()),
        reference,
        Some(&promotion),
    )
    .expect("publication revalidation accepts SQL character-length boundaries");

    let mut license_too_long = published.clone();
    license_too_long.license = format!(" {} ", "\u{03bb}".repeat(513));
    assert!(FlatImportPublicationPromotion::new(&current, reference, license_too_long).is_err());

    let mut provenance_too_long = published.clone();
    provenance_too_long.provenance = format!(" {} ", "\u{03bb}".repeat(2_049));
    assert!(FlatImportPublicationPromotion::new(&current, reference, provenance_too_long).is_err());

    let mut revalidation_too_long = published;
    revalidation_too_long.license = "\u{03bb}".repeat(513);
    let invalid_promotion = FlatImportPublicationPromotion {
        expected_current_origin: current.identity(),
        published_archive: revalidation_too_long,
    };
    assert!(
        crate::publication_validation::validate_flat_import_publication_promotion(
            TenantContext::from_authenticated_session(tenant()),
            reference,
            Some(&invalid_promotion),
        )
        .is_err(),
        "publication revalidation must reject an archive that bypassed construction"
    );
}
