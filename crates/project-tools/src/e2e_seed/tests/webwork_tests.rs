use super::*;
use question_model::{ActivityTimestamp, Capability};

#[test]
fn tracked_webwork_fixture_matches_declared_digest_and_provenance() {
    let source_digest = objects::Sha256Digest::compute(WEBWORK_PILOT_SOURCE).to_string();
    let provenance: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../content/pilot/webwork/which_hydrophobic-simple.provenance.json"
    )))
    .expect("tracked provenance is JSON");
    assert_eq!(provenance["sha256"], source_digest);
    assert_eq!(
        provenance["copiedFrom"],
        "OTHER_REPOS/biology-problems-website/site_docs/biochemistry/topic01/downloads/which_hydrophobic-simple.pgml"
    );
    assert_eq!(
        provenance["title"],
        "Biochemistry: Identify hydrophobic compounds from formulas"
    );
    assert_eq!(provenance["license"], "CC-BY-4.0");
    assert_eq!(
        provenance["licenseUrl"],
        "https://creativecommons.org/licenses/by/4.0/"
    );
    assert_eq!(provenance["author"], "Dr. Neil R. Voss");
    assert_eq!(provenance["institution"], "Roosevelt University");
    assert_eq!(provenance["date"], "2026-01-23");
    assert!(
        String::from_utf8_lossy(WEBWORK_PILOT_SOURCE)
            .contains("# Source code portions are licensed under LGPLv3.")
    );
}

#[test]
fn webwork_pilot_draft_uses_immutable_source_and_declared_capabilities() {
    let draft = webwork_pilot_draft(WorkspaceId::from_uuid(Uuid::from_u128(12)));
    assert_eq!(
        draft.source,
        DraftQuestionSource::Webwork {
            pg_path: WEBWORK_PILOT_SOURCE_PATH.to_string(),
        }
    );
    let capabilities = webwork_capabilities();
    assert!(capabilities.supports(Capability::AlgorithmicGeneration));
    assert!(capabilities.supports(Capability::ServerGrading));
    assert!(!capabilities.supports(Capability::PartialCredit));
    assert!(domain::policy::validate_draft_for_publication(&draft, &capabilities).is_empty());
}

#[test]
fn webwork_pilot_source_key_binds_fresh_publication_identity() {
    let ids = WebworkPilotSeedIds::fresh_for_installation();
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    assert_eq!(
        webwork_pilot_published_source(),
        QuestionSource::Webwork {
            pg_path: WEBWORK_PILOT_SOURCE_PATH.to_string(),
        }
    );
    assert_eq!(
        webwork_pilot_source_key(reference, ids.source_object),
        ObjectKey::ProblemSource {
            problem: ids.problem,
            version: ids.version,
            object: ids.source_object,
        }
    );
}

#[test]
fn webwork_pilot_keeps_scaffold_stable_while_publication_ids_are_fresh() {
    let first = WebworkPilotSeedIds::fresh_for_installation();
    let second = WebworkPilotSeedIds::fresh_for_installation();
    let native = SeedIds::fresh_for_installation();
    assert_eq!(first.assignment, second.assignment);
    assert_ne!(first.problem.as_uuid(), first.source_object.as_uuid());
    assert_ne!(first.problem.as_uuid(), native.problem.as_uuid());
    assert_ne!(first.assignment.as_uuid(), native.assignment.as_uuid());
    assert_ne!(first.problem, second.problem);
}

#[test]
fn webwork_catalog_baseline_uses_only_deterministic_provider_identities() {
    let first = WebworkCatalogBaselineIds::for_installation();
    let second = WebworkCatalogBaselineIds::for_installation();
    let pilot = WebworkPilotSeedIds::fresh_for_installation();
    assert_eq!(first.workspace, second.workspace);
    assert_eq!(first.problem, second.problem);
    assert_eq!(first.version, second.version);
    assert_eq!(first.source_object, second.source_object);
    assert_ne!(first.problem, pilot.problem);
    assert_ne!(first.source_object.as_uuid(), first.problem.as_uuid());
}

#[test]
fn webwork_pilot_refuses_s3_endpoint_credentials_without_echoing_them() {
    let error = validate_s3_endpoint("http://public-value:private-value@127.0.0.1:9000")
        .expect_err("credential-bearing endpoint must refuse");
    assert!(error.to_string().contains("must not include credentials"));
    assert!(!error.to_string().contains("public-value"));
    assert!(!error.to_string().contains("private-value"));
}

#[test]
fn webwork_catalog_baseline_refuses_modified_source_before_storage() {
    let mut modified = WEBWORK_PILOT_SOURCE.to_vec();
    modified.push(b'!');
    let error = validate_webwork_pilot_source_provenance(&modified)
        .expect_err("modified source must not enter private storage");
    assert!(error.to_string().contains("recorded provenance"));
}

#[tokio::test]
async fn injected_draft_create_conflict_rereads_and_accepts_only_exact_seed_content() {
    let store = learning_data_access::in_memory::MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(81));
    let actor = UserId::from_uuid(Uuid::from_u128(82));
    let context = TenantContext::from_authenticated_session(tenant);
    let draft = DraftRecord {
        tenant,
        question: webwork_pilot_draft(WorkspaceId::from_uuid(Uuid::from_u128(83))),
        derived_from: None,
    };
    let raced = store
        .upsert_draft(context, actor, None, draft.clone())
        .await
        .expect("the competing seeder wrote the draft first");
    let resumed = reconcile_webwork_pilot_draft(Some(raced), &draft)
        .expect("typed conflict reread accepts the exact competing draft")
        .expect("competing draft remains available");
    assert_eq!(resumed.record, draft);
    assert!(
        reconcile_webwork_pilot_draft(None, &draft)
            .expect("a competing publisher may consume the draft before reread")
            .is_none()
    );
    let mut different = draft.clone();
    different.question.metadata.title = "different seeded content".to_string();
    assert!(reconcile_webwork_pilot_draft(Some(resumed), &different).is_err());
}

#[tokio::test]
async fn webwork_catalog_baseline_reconciles_one_publication_without_product_state() {
    let store = learning_data_access::in_memory::MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(94));
    let instructor = UserId::from_uuid(Uuid::from_u128(95));
    let student = UserId::from_uuid(Uuid::from_u128(96));
    let context = TenantContext::from_authenticated_session(tenant);
    let ids = WebworkCatalogBaselineIds::for_installation();
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let source_record = objects::ObjectRecord {
        id: ids.source_object,
        bucket: objects::Bucket::PrivateContent,
        key: webwork_pilot_source_key(reference, ids.source_object),
        sha256: objects::Sha256Digest::compute(WEBWORK_PILOT_SOURCE),
        size_bytes: u64::try_from(WEBWORK_PILOT_SOURCE.len()).expect("fixture fits u64"),
        media_type: "text/x-wework-pg".to_string(),
        category: ObjectCategory::Source,
        version: Some(ids.version),
        license: "CC-BY-4.0".to_string(),
        provenance: WEBWORK_PILOT_SOURCE_PROVENANCE.to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
    };
    let draft = DraftRecord {
        tenant,
        question: webwork_pilot_draft(ids.workspace),
        derived_from: None,
    };
    let first = ensure_webwork_pilot_publication(
        &store,
        context,
        instructor,
        draft.clone(),
        reference,
        source_record.clone(),
        webwork_capabilities(),
    )
    .await
    .expect("catalog baseline publication converges");
    let second = ensure_webwork_pilot_publication(
        &store,
        context,
        instructor,
        draft,
        reference,
        source_record,
        webwork_capabilities(),
    )
    .await
    .expect("catalog baseline rerun verifies the same publication");
    assert_eq!(second, first);

    let receipt = WebworkCatalogBaselineReceipt::from_published(&first)
        .expect("published catalog item has a public receipt");
    let encoded = serde_json::to_value(receipt).expect("public receipt serializes");
    assert_eq!(encoded["questionId"], first.question_id.to_string());
    assert_eq!(encoded["title"], first.question.metadata.title);
    let receipt_keys = encoded
        .as_object()
        .expect("receipt is an object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(receipt_keys, vec!["questionId", "title"]);

    let product_ids = WebworkPilotSeedIds::fresh_for_installation();
    assert!(
        store
            .get_course(context, product_ids.course)
            .await
            .expect("catalog verification reads course boundary")
            .is_none()
    );
    assert!(
        store
            .get_assignment_for_edit(context, product_ids.assignment)
            .await
            .expect("catalog verification reads assignment boundary")
            .is_none()
    );
    assert!(
        store
            .get_current_course_membership(context, product_ids.course, student)
            .await
            .expect("catalog verification reads roster boundary")
            .is_none()
    );
}

#[tokio::test]
async fn webwork_pilot_converges_after_every_persisted_prefix_and_on_rerun() {
    let store = learning_data_access::in_memory::MemoryStore::default();
    let tenant = TenantId::from_uuid(Uuid::from_u128(91));
    let instructor = UserId::from_uuid(Uuid::from_u128(92));
    let student = UserId::from_uuid(Uuid::from_u128(93));
    let context = TenantContext::from_authenticated_session(tenant);
    let ids = WebworkPilotSeedIds::fresh_for_installation();
    let reference = ProblemVersionRef {
        problem: ids.problem,
        version: ids.version,
    };
    let source_key = webwork_pilot_source_key(reference, ids.source_object);
    let source_record = objects::ObjectRecord {
        id: ids.source_object,
        bucket: objects::Bucket::PrivateContent,
        key: source_key,
        sha256: objects::Sha256Digest::compute(WEBWORK_PILOT_SOURCE),
        size_bytes: u64::try_from(WEBWORK_PILOT_SOURCE.len()).expect("fixture fits u64"),
        media_type: "text/x-wework-pg".to_string(),
        category: ObjectCategory::Source,
        version: Some(ids.version),
        license: "CC-BY-4.0".to_string(),
        provenance: WEBWORK_PILOT_SOURCE_PROVENANCE.to_string(),
        created_at: ActivityTimestamp::from_unix_millis(1),
    };
    let draft = DraftRecord {
        tenant,
        question: webwork_pilot_draft(ids.workspace),
        derived_from: None,
    };
    let capabilities = webwork_capabilities();
    ensure_webwork_pilot_publication(
        &store,
        context,
        instructor,
        draft.clone(),
        reference,
        source_record.clone(),
        capabilities.clone(),
    )
    .await
    .expect("publication prefix converges");

    let course = CourseRecord {
        id: ids.course,
        tenant,
        title: "PLE WebWork pilot E2E course".to_string(),
        term: question_model::CourseTerm::from_parts("2026-08-24", "2026-12-18", "America/Chicago")
            .expect("explicit fixture course term"),
    };
    ensure_webwork_pilot_course(&store, context, instructor, course.clone())
        .await
        .expect("course prefix converges");
    let assignment = AssignmentRecord {
        id: ids.assignment,
        tenant,
        course_id: ids.course,
        title: "PLE WebWork pilot E2E assignment".to_string(),
        lifecycle: question_model::AssignmentLifecycle::Published,
        instructions: question_model::AssignmentInstructions::try_new(
            "Solve the guided WeBWorK pilot problem, then explain your reasoning.".to_string(),
        )
        .expect("WebWork pilot instructions are valid"),
        audience: question_model::AssignmentAudience::CourseWide,
        items: vec![AssignmentItem {
            id: ids.assignment_item,
            reference,
            position: 0,
            points_possible: PointValue::from_whole(1),
            delivery_state: AssignmentDeliveryState::Active,
            scoring_mode: AssignmentScoringMode::Normal,
        }],
        selection_groups: Vec::new(),
        disclosure_policy: question_model::StudentDisclosurePolicy::default(),
        policies: RunPolicies {
            completion: CompletionRequirement::AnswerAll,
            grade: GradePolicy::Highest,
            continued_practice: ContinuedPractice::Unlimited,
            variation: VariationPolicy::NewSeeds,
        },
    };
    ensure_webwork_pilot_assignment(&store, context, instructor, assignment.clone())
        .await
        .expect("assignment prefix converges");
    ensure_webwork_pilot_enrollment(
        &store,
        context,
        instructor,
        student,
        ids.course,
        ids.assignment,
    )
    .await
    .expect("enrollment prefix converges");

    ensure_webwork_pilot_publication(
        &store,
        context,
        instructor,
        draft,
        reference,
        source_record.clone(),
        capabilities,
    )
    .await
    .expect("published rerun verifies rather than republishes");
    ensure_webwork_pilot_course(&store, context, instructor, course)
        .await
        .expect("course rerun verifies rather than mutates");
    ensure_webwork_pilot_assignment(&store, context, instructor, assignment)
        .await
        .expect("assignment rerun verifies rather than mutates");
    ensure_webwork_pilot_enrollment(
        &store,
        context,
        instructor,
        student,
        ids.course,
        ids.assignment,
    )
    .await
    .expect("enrollment rerun verifies rather than mutates");
    let error = ensure_webwork_pilot_publication(
        &store,
        context,
        instructor,
        DraftRecord {
            tenant,
            question: webwork_pilot_draft(ids.workspace),
            derived_from: None,
        },
        reference,
        source_record,
        BackendCapabilities::from_iter([Capability::ServerGrading]),
    )
    .await
    .expect_err("capability mismatch must refuse instead of mutating a published source");
    assert!(
        error
            .to_string()
            .contains("publication differs from the deterministic seed")
    );
}
