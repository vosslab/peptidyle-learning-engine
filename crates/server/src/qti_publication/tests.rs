use std::{io::Cursor, sync::Arc};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use learning_data_access::in_memory::MemoryStore;
use learning_data_access::{
    CommitPreparedQtiImport, CommitPreparedQtiImportOutcome, CreateQtiImportCommand, EnqueueJob,
    JobLeaseDuration, JobPayload, JobStore, PageRequest, PageSize, QtiImportGradingPayload,
    QtiImportItem, QtiImportItemRegistration, QtiImportItemResult, QtiImportItemStatus,
    QtiImportRef, QtiImportRegistry, QtiImportStore, SessionLifetime, SessionSubject,
};
use objects::memory::MemoryObjectStore;
use objects::{ObjectCategory, Sha256Digest};
use question_model::answer::SelectionCardinality;
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::generation::RandomizationDefinition;
use question_model::response::{ChoiceId, ChoiceOption};
use question_model::run_policy::{AttemptPolicy, FeedbackDisclosure, TimingPolicy};
use question_model::taxonomy::License;
use question_model::{
    ActivityTimestamp, AssetId, BackendCapabilities, Capability, GradingDefinition,
    QuestionMetadata, ResponseDefinition, UserRole, WorkspaceId,
};
use tower::ServiceExt;

use crate::qti_import::QtiImportHandler;
use crate::worker::{JobExecution, JobHandler};

use super::*;

struct QtiRegistry;

impl BackendRegistry for QtiRegistry {
    fn capabilities(
        &self,
        source: &question_model::DraftQuestionSource,
    ) -> Result<BackendCapabilities, BackendRegistryError> {
        if matches!(source, question_model::DraftQuestionSource::Qti { .. }) {
            Ok(BackendCapabilities::from_iter([
                Capability::ServerGrading,
                Capability::Hints,
            ]))
        } else {
            Err(BackendRegistryError::Unsupported)
        }
    }
}

struct RevisionChangingReview {
    store: Arc<MemoryStore>,
    actor: UserId,
}

#[async_trait]
impl PublicReviewGate for RevisionChangingReview {
    async fn allows_publication(
        &self,
        context: TenantContext,
        _publisher: UserId,
        draft: &DraftRecord,
    ) -> Result<bool, crate::catalog::ReviewGateError> {
        let current = self
            .store
            .get_draft(context, self.actor, draft.question.workspace)
            .await
            .map_err(|error| crate::catalog::ReviewGateError(error.to_string()))?
            .ok_or_else(|| crate::catalog::ReviewGateError("fixture draft missing".to_string()))?;
        let mut changed = current.record;
        changed.question.metadata.title = "Changed during review".to_string();
        self.store
            .upsert_draft(context, self.actor, Some(current.revision), changed)
            .await
            .map_err(|error| crate::catalog::ReviewGateError(error.to_string()))?;
        Ok(true)
    }
}

const VALID_PACKAGE: &str = concat!(
    "UEsDBBQAAAAIAHS7B13yXbGdXwAAAIsAAAAPAAAAaW1zbWFuaWZlc3QueG1sVY5RDkAwEESv0uwBNHxXryLClg2l",
    "uku4vYoIfiYvM5PJGF9P5JBFUYuTkCOMJYShA2si8rzGBvnFX4sEPSg5Aib2vAhVl1XtftyKkIPqI7q7xvrSLCWg",
    "rdGfZf0csCdQSwMEFAAAAAgAdLsHXcJKi+S6AAAAiwEAAA4AAABpdGVtcy9pdGVtLnhtbH2QSw7CMAxErxLlAETs",
    "XUu0sOgGUDlBCEaN1CZVHH63J7QgKEXsrPEbe2zQzMTckotlpFbYQ6rs0VLIpE2CRAjEnXdMSzKNDjpa70ZYtdpt",
    "N+vdKqHGh0AmVk8Hwlk3J8I9qKEANSHUj/EIj9W5P9wQOixq75lErEk83UI7vlCYgerSztpbQ6WLFLTpw70mlr9C",
    "ilZfi97CmZynzGzbrqFBGt2lJS5Afbb/wHuJ+TesJtGS9r5MjV+Pd1BLAQIUAxQAAAAIAHS7B13yXbGdXwAAAIsA",
    "AAAPAAAAAAAAAAAAAACAAQAAAABpbXNtYW5pZmVzdC54bWxQSwECFAMUAAAACAB0uwddwkqL5LoAAACLAQAADgAA",
    "AAAAAAAAAAAAgAGMAAAAaXRlbXMvaXRlbS54bWxQSwUGAAAAAAIAAgB5AAAAcgEAAAAA",
);

fn id(value: u128) -> uuid::Uuid {
    uuid::Uuid::from_u128(value)
}

fn legacy_svg_package(svg: &str) -> Vec<u8> {
    let manifest = "<manifest identifier='package'><resources><resource identifier='choice' type='imsqti_item_xmlv2p1' href='items/item.xml'/></resources></manifest>";
    let item = "<assessmentItem identifier='legacy-svg'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><p>Look <img src='../assets/legacy.svg' alt='plot'/></p><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>";
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("imsmanifest.xml", manifest.as_bytes()),
        ("items/item.xml", item.as_bytes()),
        ("assets/legacy.svg", svg.as_bytes()),
    ] {
        writer
            .start_file(path, zip::write::SimpleFileOptions::default())
            .expect("start legacy QTI fixture entry");
        std::io::Write::write_all(&mut writer, bytes).expect("write legacy QTI fixture entry");
    }
    writer
        .finish()
        .expect("finish legacy QTI fixture")
        .into_inner()
}

async fn issued_cookie(store: &MemoryStore, user: UserId, roles: Vec<UserRole>) -> String {
    let issued = crate::auth::issue_session(
        store,
        SessionSubject::new(
            question_model::TenantId::from_uuid(id(1)),
            user,
            "QTI route fixture",
            roles,
        )
        .expect("fixture identity"),
        crate::auth::SessionConfig::new(
            SessionLifetime::from_seconds(3_600).expect("valid fixture lifetime"),
            crate::auth::CookieTransport::LocalHttp,
        ),
    )
    .await
    .expect("fixture session");
    issued
        .set_cookie
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

struct Fixture {
    store: Arc<MemoryStore>,
    objects: Arc<MemoryObjectStore>,
    context: TenantContext,
    draft: DraftRecord,
    import: WorkspaceImportId,
}

fn imported_qti_draft(
    tenant: question_model::TenantId,
    workspace: WorkspaceId,
    import: WorkspaceImportId,
    item: &adapter_qti::ImportedQtiQuestion,
) -> DraftRecord {
    DraftRecord {
        tenant,
        question: question_model::DraftQuestionDefinition {
            workspace,
            source: question_model::DraftQuestionSource::Qti {
                item_id: item.item_id.clone(),
                import_id: import,
            },
            prompt: item.prompt.clone(),
            response: item.response.clone(),
            attempt_policy: AttemptPolicy {
                max_attempts: None,
                feedback: FeedbackDisclosure::ImmediateCorrectness,
            },
            timing_policy: TimingPolicy::Untimed,
            randomization: RandomizationDefinition::Static,
            grading: GradingDefinition::AllOrNothing { points: 1.0 },
            metadata: QuestionMetadata {
                title: "Imported QTI question".to_string(),
                tags: Vec::new(),
                taxonomy: Vec::new(),
                license: License::CcBy,
                language: "en-US".to_string(),
            },
        },
        derived_from: None,
    }
}

async fn committed_fixture(owner: UserId) -> Fixture {
    let tenant = question_model::TenantId::from_uuid(id(1));
    let workspace = WorkspaceId::from_uuid(id(2));
    let import = WorkspaceImportId::from_uuid(id(3));
    let source_object = ObjectId::from_uuid(id(4));
    let bytes = STANDARD
        .decode(VALID_PACKAGE.trim())
        .expect("fixture base64 must decode");
    let parsed = QtiImporter::default()
        .import(&bytes)
        .expect("fixture package must parse");
    let item = parsed.questions.first().expect("fixture item").clone();
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjectStore::default());
    let context = TenantContext::from_authenticated_session(tenant);
    let draft = imported_qti_draft(tenant, workspace, import, &item);
    store
        .upsert_draft(context, owner, None, draft.clone())
        .await
        .expect("fixture QTI draft saves before staging");
    objects
        .put(PutObject {
            key: ObjectKey::WorkspaceSource {
                tenant,
                workspace,
                import,
                object: source_object,
            },
            bytes,
            media_type: "application/zip".to_string(),
            license: "private-workspace-import".to_string(),
            provenance: "QTI test source".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .expect("fixture source persists");
    let handler = QtiImportHandler::new(Arc::clone(&store), Arc::clone(&objects));
    handler
        .prepare(
            context,
            JobPayload::QtiImport {
                workspace,
                import,
                source_object,
            },
            JobExecution::new(),
        )
        .await
        .expect("fixture QTI preparation");
    let job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::QtiImport {
                    workspace,
                    import,
                    source_object,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("fixture QTI job");
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("valid lease"),
        )
        .await
        .expect("claim query")
        .expect("fixture claim");
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference: learning_data_access::QtiImportRef {
                        tenant,
                        workspace,
                        import,
                    },
                    source_object,
                },
            )
            .await
            .expect("fixture import commit"),
        CommitPreparedQtiImportOutcome::Committed
    );
    Fixture {
        store,
        objects,
        context,
        draft,
        import,
    }
}

#[tokio::test]
async fn dedicated_qti_route_validates_then_copies_exact_source_bytes_first() {
    let fixture = committed_fixture(UserId::from_uuid(id(7))).await;
    let preparer =
        QtiPublicationPreparer::new(Arc::clone(&fixture.store), Arc::clone(&fixture.objects));
    let question_model::DraftQuestionSource::Qti { item_id, .. } = &fixture.draft.question.source
    else {
        panic!("fixture must be QTI");
    };
    let validated = preparer
        .validate(
            fixture.context,
            &fixture.draft.question,
            fixture.import,
            item_id,
        )
        .await
        .expect("committed matching QTI validates before identity minting");
    let reference = ProblemVersionRef {
        problem: question_model::ProblemId::from_uuid(id(5)),
        version: question_model::VersionId::from_uuid(id(6)),
    };
    let prepared = preparer
        .copy_candidates(
            &fixture.draft,
            reference,
            PublicationScope::Public,
            validated,
        )
        .await
        .expect("validated QTI copies candidate objects");

    let QuestionSource::Qti {
        item_id: published_item,
        package_object,
        package_sha256,
    } = &prepared.published_source
    else {
        panic!("prepared source must remain QTI");
    };
    assert_eq!(published_item, item_id);
    assert_eq!(package_object, &prepared.source_artifact.object.id);
    assert_eq!(
        package_sha256,
        &prepared.source_artifact.object.sha256.to_string()
    );
    assert_eq!(prepared.source_artifact.reference, reference);
    assert_eq!(
        prepared.source_artifact.object.category,
        ObjectCategory::Source
    );
    assert!(
        prepared.promotion.assets.iter().all(|asset| matches!(
            asset.object.key,
            ObjectKey::ProblemAsset { .. }
        ) && asset.object.bucket
            == objects::Bucket::PublicAssets),
        "only globally public QTI publication may create CDN-readable asset keys"
    );
    assert!(matches!(
        prepared.source_artifact.object.key,
        ObjectKey::ProblemSource { problem, version, .. }
            if problem == reference.problem && version == reference.version
    ));
    assert!(prepared.promotion.assets.is_empty());
    let candidate = fixture
        .objects
        .get(&prepared.source_artifact.object.key)
        .await
        .expect("candidate source is written before Store promotion");
    assert_eq!(candidate.record, prepared.source_artifact.object);
    assert_eq!(
        candidate.bytes,
        STANDARD
            .decode(VALID_PACKAGE.trim())
            .expect("fixture bytes")
    );
}

#[tokio::test]
async fn publication_reparse_refuses_legacy_staged_svg_before_candidate_copy() {
    const SECRET: &str = "legacy_svg_publication_secret";
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' onload='{SECRET}()'><script>{SECRET}()</script></svg>"
    );
    let source_bytes = legacy_svg_package(&svg);
    let tenant = question_model::TenantId::from_uuid(id(101));
    let workspace = WorkspaceId::from_uuid(id(102));
    let import = WorkspaceImportId::from_uuid(id(103));
    let source_object = ObjectId::from_uuid(id(104));
    let asset = AssetId::from_uuid(id(105));
    let asset_object = ObjectId::from_uuid(id(106));
    let store = Arc::new(MemoryStore::default());
    let objects = Arc::new(MemoryObjectStore::default());
    let source = objects
        .put(PutObject {
            key: ObjectKey::WorkspaceSource {
                tenant,
                workspace,
                import,
                object: source_object,
            },
            bytes: source_bytes,
            media_type: "application/zip".to_string(),
            license: "private-workspace-import".to_string(),
            provenance: "legacy QTI source fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .expect("legacy source persists");
    let staged_asset = objects
        .put(PutObject {
            key: ObjectKey::WorkspaceAsset {
                tenant,
                workspace,
                import,
                asset,
                object: asset_object,
            },
            bytes: svg.into_bytes(),
            media_type: "image/svg+xml".to_string(),
            license: "private-workspace-import".to_string(),
            provenance: "legacy QTI asset fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .expect("legacy staged asset persists");
    let question = adapter_qti::ImportedQtiQuestion {
        item_id: "legacy-svg".to_string(),
        prompt: vec![ContentBlock::Image {
            asset: AssetRef {
                asset,
                checksum: staged_asset.sha256.to_string(),
            },
            description: "plot".to_string(),
        }],
        response: ResponseDefinition::MultipleChoice {
            choices: vec![
                ChoiceOption {
                    id: ChoiceId::new("a"),
                    body: vec![ContentBlock::Text {
                        markdown: "A".to_string(),
                    }],
                },
                ChoiceOption {
                    id: ChoiceId::new("b"),
                    body: vec![ContentBlock::Text {
                        markdown: "B".to_string(),
                    }],
                },
            ],
            selection: SelectionCardinality::ExactlyOne,
        },
    };
    let model_sha256 = Sha256Digest::compute(
        &serde_json::to_vec(&question).expect("legacy normalized model serializes"),
    );
    let staged_item = QtiImportItem {
        item_id: question.item_id.clone(),
        model_sha256,
        assets: vec![asset],
    };
    let reference = QtiImportRef {
        tenant,
        workspace,
        import,
    };
    let context = TenantContext::from_authenticated_session(tenant);
    store
        .upsert_draft(
            context,
            UserId::from_uuid(id(107)),
            None,
            imported_qti_draft(tenant, workspace, import, &question),
        )
        .await
        .expect("legacy fixture draft saves before staging");
    store
        .prepare_qti_import(
            context,
            CreateQtiImportCommand {
                registry: QtiImportRegistry {
                    reference,
                    source,
                    source_format: "qti".to_string(),
                    source_identifier: Some("legacy-package".to_string()),
                    importer: "adapter_qti".to_string(),
                    parse_schema: adapter_qti::QtiProfileId::GENERIC.to_string(),
                    adapter_version: "legacy-svg-fixture".to_string(),
                    profile_summary: None,
                    items: vec![staged_item.clone()],
                    item_results: vec![QtiImportItemResult {
                        source_identifier: "choice".to_string(),
                        title: Some("Legacy choice item".to_string()),
                        item_id: Some(question.item_id.clone()),
                        normalized_sha256: Some(model_sha256),
                        status: QtiImportItemStatus::Accepted,
                        diagnostics: Vec::new(),
                        defaults: Vec::new(),
                        warnings: Vec::new(),
                    }],
                    assets: vec![staged_asset],
                    unsupported_features: Vec::new(),
                },
                item_bindings: vec![QtiImportItemRegistration {
                    item: staged_item,
                    grading: QtiImportGradingPayload::new(
                        serde_json::to_vec(&ChoiceId::new("b"))
                            .expect("legacy grading fixture serializes"),
                    )
                    .expect("legacy grading fixture is bounded"),
                }],
            },
        )
        .await
        .expect("legacy accepted staging is represented");
    let job = store
        .enqueue_job(
            context,
            EnqueueJob {
                tenant,
                payload: JobPayload::QtiImport {
                    workspace,
                    import,
                    source_object,
                },
                max_attempts: 1,
            },
        )
        .await
        .expect("legacy fixture job enqueues");
    let claim = store
        .claim_next_job(
            &learning_data_access::JobClaimFilter::all(),
            JobLeaseDuration::from_seconds(60).expect("bounded fixture lease"),
        )
        .await
        .expect("legacy fixture claim query")
        .expect("legacy fixture job claims");
    assert_eq!(claim.id, job);
    assert_eq!(
        store
            .commit_prepared_qti_import(
                context,
                CommitPreparedQtiImport {
                    job,
                    lease: claim.lease_token,
                    reference,
                    source_object,
                },
            )
            .await
            .expect("legacy staging commits"),
        CommitPreparedQtiImportOutcome::Committed
    );
    let draft = question_model::DraftQuestionDefinition {
        workspace,
        source: question_model::DraftQuestionSource::Qti {
            item_id: question.item_id.clone(),
            import_id: import,
        },
        prompt: question.prompt,
        response: question.response,
        attempt_policy: AttemptPolicy {
            max_attempts: None,
            feedback: FeedbackDisclosure::ImmediateCorrectness,
        },
        timing_policy: TimingPolicy::Untimed,
        randomization: RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Legacy SVG QTI question".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    };
    let preparer = QtiPublicationPreparer::new(store, objects);
    let error = preparer
        .validate(context, &draft, import, "legacy-svg")
        .await
        .expect_err("publication must reparse and reject previously staged SVG");
    assert_eq!(error, StoreError::NotFound);
    assert!(!error.to_string().contains(SECRET));
}

#[tokio::test]
async fn dedicated_qti_route_refuses_changed_draft_before_candidate_copy() {
    let mut fixture = committed_fixture(UserId::from_uuid(id(17))).await;
    fixture.draft.question.prompt.push(ContentBlock::Text {
        markdown: "browser substitution".to_string(),
    });
    let preparer =
        QtiPublicationPreparer::new(Arc::clone(&fixture.store), Arc::clone(&fixture.objects));
    let question_model::DraftQuestionSource::Qti { item_id, .. } = &fixture.draft.question.source
    else {
        panic!("fixture must be QTI");
    };
    assert_eq!(
        preparer
            .validate(
                fixture.context,
                &fixture.draft.question,
                fixture.import,
                item_id,
            )
            .await,
        Err(StoreError::Conflict)
    );
}

#[tokio::test]
async fn dedicated_qti_route_refuses_foreign_tenant_before_object_lookup() {
    let fixture = committed_fixture(UserId::from_uuid(id(27))).await;
    let preparer =
        QtiPublicationPreparer::new(Arc::clone(&fixture.store), Arc::clone(&fixture.objects));
    let question_model::DraftQuestionSource::Qti { item_id, .. } = &fixture.draft.question.source
    else {
        panic!("fixture must be QTI");
    };
    assert_eq!(
        preparer
            .validate(
                TenantContext::from_authenticated_session(question_model::TenantId::from_uuid(id(
                    99
                ))),
                &fixture.draft.question,
                fixture.import,
                item_id,
            )
            .await,
        Err(StoreError::NotFound)
    );
}

#[tokio::test]
async fn qti_publish_endpoint_is_the_only_route_that_promotes_committed_staging() {
    let publisher = UserId::from_uuid(id(8));
    let fixture = committed_fixture(publisher).await;
    let saved = fixture
        .store
        .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
        .await
        .expect("owner draft lookup")
        .expect("fixture draft exists");
    let cookie = issued_cookie(&fixture.store, publisher, vec![UserRole::Instructor]).await;
    let app = router(
        Arc::clone(&fixture.store),
        Arc::clone(&fixture.objects),
        Arc::new(QtiRegistry),
        Arc::new(crate::catalog::ReviewNotRequired),
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/problems/{}/qti-publish",
                    fixture.draft.question.workspace
                ))
                .header("cookie", cookie)
                .header("if-match", format!("\"{}\"", saved.revision.value()))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("QTI publish request"),
        )
        .await
        .expect("QTI publish response");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("published response body");
    assert_eq!(
        status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&body)
    );
    let published: serde_json::Value =
        serde_json::from_slice(&body).expect("published browser projection");
    assert_eq!(published["backend"], "qti");
    assert!(published.get("source").is_none());
    assert!(
        fixture
            .store
            .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
            .await
            .expect("post-publish draft lookup")
            .is_none()
    );
}

#[tokio::test]
async fn qti_publish_requires_one_current_strong_workspace_revision() {
    let publisher = UserId::from_uuid(id(18));
    let fixture = committed_fixture(publisher).await;
    let saved = fixture
        .store
        .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
        .await
        .expect("owner draft lookup")
        .expect("fixture draft exists");
    let cookie = issued_cookie(&fixture.store, publisher, vec![UserRole::Instructor]).await;
    let app = router(
        Arc::clone(&fixture.store),
        Arc::clone(&fixture.objects),
        Arc::new(QtiRegistry),
        Arc::new(crate::catalog::ReviewNotRequired),
    );
    for (header, expected) in [
        (None, StatusCode::PRECONDITION_REQUIRED),
        (Some("W/\"1\""), StatusCode::UNPROCESSABLE_ENTITY),
        (Some("\"0\""), StatusCode::UNPROCESSABLE_ENTITY),
        (
            Some("\"9223372036854775808\""),
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (Some("\"999\""), StatusCode::CONFLICT),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri(format!(
                "/api/problems/{}/qti-publish",
                fixture.draft.question.workspace
            ))
            .header("cookie", &cookie)
            .header("content-type", "application/json");
        if let Some(header) = header {
            request = request.header("if-match", header);
        }
        let response = app
            .clone()
            .oneshot(
                request
                    .body(Body::from(
                        r#"{"scope":"institution","byline":{"names":["PLE fixture"]}}"#,
                    ))
                    .expect("QTI publish request"),
            )
            .await
            .expect("QTI publish response");
        assert_eq!(response.status(), expected);
    }
    assert_eq!(saved.revision.value(), 1);
    assert!(
        fixture
            .store
            .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
            .await
            .expect("revision failures retain draft")
            .is_some()
    );
}

#[tokio::test]
async fn qti_publish_rejects_review_time_draft_change_without_visible_version() {
    let publisher = UserId::from_uuid(id(28));
    let fixture = committed_fixture(publisher).await;
    let saved = fixture
        .store
        .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
        .await
        .expect("owner draft lookup")
        .expect("fixture draft exists");
    let cookie = issued_cookie(&fixture.store, publisher, vec![UserRole::Instructor]).await;
    let app = router(
        Arc::clone(&fixture.store),
        Arc::clone(&fixture.objects),
        Arc::new(QtiRegistry),
        Arc::new(RevisionChangingReview {
            store: Arc::clone(&fixture.store),
            actor: publisher,
        }),
    );
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/problems/{}/qti-publish",
                    fixture.draft.question.workspace
                ))
                .header("cookie", cookie)
                .header("if-match", format!("\"{}\"", saved.revision.value()))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"scope":"public","byline":{"names":["PLE fixture"]}}"#,
                ))
                .expect("QTI publish request"),
        )
        .await
        .expect("QTI publish response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let draft = fixture
        .store
        .get_draft(fixture.context, publisher, fixture.draft.question.workspace)
        .await
        .expect("changed draft lookup")
        .expect("changed draft remains");
    assert_eq!(draft.revision.value(), saved.revision.value() + 1);
    assert_eq!(
        draft.record.question.metadata.title,
        "Changed during review"
    );
    let page = fixture
        .store
        .list_catalog(
            fixture.context,
            PageRequest::first(PageSize::new(10).expect("page size")),
        )
        .await
        .expect("catalog lookup");
    assert!(
        page.items.is_empty(),
        "stale publication must stay invisible"
    );
}
