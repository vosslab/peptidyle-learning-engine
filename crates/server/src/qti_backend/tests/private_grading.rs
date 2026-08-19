use super::*;
use image::ImageEncoder as _;

#[tokio::test]
async fn published_qti_reparses_and_grades_only_through_the_private_grader() {
    let fixture = fixture().await;
    let issued = fixture
        .backend
        .issue(fixture.context, fixture.reference, &fixture.question, 41)
        .await
        .expect("immutable QTI issues");
    let stored = attempt(&fixture, issued.clone());
    let envelope = fixture
        .backend
        .reproduce(
            fixture.context,
            fixture.reference,
            &fixture.question,
            &stored,
        )
        .await
        .expect("immutable QTI replays");
    assert_eq!(envelope.prompt, fixture.question.prompt);
    assert!(
        !serde_json::to_string(&envelope)
            .expect("envelope serializes")
            .contains("\"correct\":")
    );
    let right = fixture
        .backend
        .grade(
            fixture.context,
            fixture.reference,
            &fixture.question,
            &stored,
            &StudentResponse::MultipleChoice {
                selected: vec![fixture.correct],
            },
        )
        .await
        .expect("private grader grades correct response");
    assert!(matches!(right, grading::GradeOutcome::Graded(result) if result.correct));
    let wrong = fixture
        .backend
        .grade(
            fixture.context,
            fixture.reference,
            &fixture.question,
            &stored,
            &StudentResponse::MultipleChoice {
                selected: vec![fixture.incorrect],
            },
        )
        .await
        .expect("private grader grades wrong response");
    assert!(matches!(wrong, grading::GradeOutcome::Graded(result) if !result.correct));
    assert_eq!(fixture.grader_calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn foreign_or_tampered_qti_attempt_refuses_before_grading() {
    let fixture = fixture().await;
    let issued = fixture
        .backend
        .issue(fixture.context, fixture.reference, &fixture.question, 41)
        .await
        .expect("immutable QTI issues");
    let mut stored = attempt(&fixture, issued);
    stored.provenance.rendered_question_sha256 = "tampered".to_string();
    assert!(matches!(
        fixture
            .backend
            .grade(
                fixture.context,
                fixture.reference,
                &fixture.question,
                &stored,
                &StudentResponse::MultipleChoice {
                    selected: vec![fixture.correct.clone()],
                },
            )
            .await,
        Err(RunBackendError::Invalid(_))
    ));
    assert_eq!(
        fixture.grader_calls.load(Ordering::SeqCst),
        0,
        "tampered provenance never reaches the private grader"
    );
    let foreign = TenantContext::from_authenticated_session(TenantId::from_uuid(
        uuid::Uuid::from_u128(7_099),
    ));
    assert!(matches!(
        fixture
            .backend
            .issue(foreign, fixture.reference, &fixture.question, 41)
            .await,
        Err(RunBackendError::Invalid(_))
    ));
    assert_eq!(
        fixture.grader_calls.load(Ordering::SeqCst),
        0,
        "foreign source resolution never reaches the private grader"
    );
}

#[tokio::test]
async fn qti_asset_resolution_uses_catalog_key_without_public_private_fallback() {
    let tenant = TenantId::from_uuid(uuid::Uuid::from_u128(7_120));
    let context = TenantContext::from_authenticated_session(tenant);
    let reference = ProblemVersionRef {
        problem: ProblemId::from_uuid(uuid::Uuid::from_u128(7_121)),
        version: VersionId::from_uuid(uuid::Uuid::from_u128(7_122)),
    };
    let workspace = WorkspaceId::from_uuid(uuid::Uuid::from_u128(7_123));
    let source_object = ObjectId::from_uuid(uuid::Uuid::from_u128(7_124));
    let replacement_object = ObjectId::from_uuid(uuid::Uuid::from_u128(7_125));
    let bytes = choice_image_package();
    let parsed = adapter_qti::QtiImporter::default()
        .import(&bytes)
        .expect("choice-image QTI fixture parses");
    let imported = parsed.questions.first().expect("choice-image item").clone();
    let expected = qti_question_asset_checksums(&imported)
        .expect("choice-image asset reference is internally consistent");
    let (asset, original_checksum) = expected
        .into_iter()
        .next()
        .expect("choice body references one asset");
    let correct = parsed
        .worker_correct_choice(&imported.item_id)
        .expect("private correct choice exists");
    let objects = Arc::new(MemoryObjectStore::default());
    let replacement_bytes = still_png([12, 34, 56]);
    assert!(
        objects::image_validation::verify_still_image(&replacement_bytes).is_ok(),
        "the fallback oracle must use an allowed still raster, not malformed bytes"
    );
    let source = objects
        .put(PutObject {
            key: ObjectKey::ProblemSource {
                problem: reference.problem,
                version: reference.version,
                object: source_object,
            },
            bytes,
            media_type: "application/zip".to_string(),
            license: "CC-BY-4.0".to_string(),
            provenance: "choice-image QTI fixture".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .expect("published QTI source stores");
    let replacement = objects
        .put(PutObject {
            key: ObjectKey::ProblemAsset {
                problem: reference.problem,
                version: reference.version,
                asset,
                object: replacement_object,
            },
            bytes: replacement_bytes,
            media_type: "image/png".to_string(),
            license: "CC-BY-4.0".to_string(),
            provenance: "public-only fallback oracle".to_string(),
            created_at: ActivityTimestamp::from_unix_millis(1),
        })
        .await
        .expect("replacement asset stores");
    assert_eq!(replacement.sha256.to_string(), original_checksum);
    let question = QuestionDefinition {
        problem: reference.problem,
        version: reference.version,
        workspace,
        source: QuestionSource::Qti {
            item_id: imported.item_id.clone(),
            package_object: source_object,
            package_sha256: source.sha256.to_string(),
        },
        prompt: imported.prompt,
        response: imported.response,
        attempt_policy: AttemptPolicy { max_attempts: None },
        timing_policy: TimingPolicy::Untimed,
        randomization: question_model::generation::RandomizationDefinition::Static,
        grading: GradingDefinition::AllOrNothing { points: 1.0 },
        metadata: QuestionMetadata {
            title: "Choice image checksum guard".to_string(),
            tags: Vec::new(),
            taxonomy: Vec::new(),
            license: License::CcBy,
            language: "en-US".to_string(),
        },
    };
    let sources = Arc::new(FixtureSources {
        tenant,
        artifact: PublishedSourceArtifact {
            reference,
            backend: question_model::QuestionBackend::Qti,
            object: source,
        },
        bindings: vec![CatalogAssetBinding {
            asset,
            object: replacement_object,
            key: ObjectKey::RestrictedProblemAsset {
                problem: reference.problem,
                version: reference.version,
                asset,
                object: replacement_object,
            },
            rendition_checksum: replacement.sha256,
            media_type: "image/png".to_string(),
            intrinsic_width: None,
            intrinsic_height: None,
        }],
    });
    let grader_calls = Arc::new(AtomicUsize::new(0));
    let grader = Arc::new(RecordedGrader {
        tenant,
        reference,
        item: imported.item_id,
        payload: QtiImportGradingPayload::new(
            serde_json::to_vec(&correct).expect("choice serializes"),
        )
        .expect("private payload is bounded"),
        calls: Arc::clone(&grader_calls),
    });
    let backend = QtiBackend::new(sources, grader, objects);
    assert!(matches!(
        backend.issue(context, reference, &question, 41).await,
        Err(RunBackendError::Invalid(_))
    ));
    assert_eq!(
        grader_calls.load(Ordering::SeqCst),
        0,
        "the resolver must use the exact catalog key and never probe the public fallback"
    );
}

fn choice_image_package() -> Vec<u8> {
    let image_bytes = still_png([12, 34, 56]);
    let manifest = "<manifest identifier='package'><resources><resource identifier='choice' type='imsqti_item_xmlv2p1' href='items/choice.xml'/></resources></manifest>";
    let item = "<assessmentItem identifier='choice-image'><responseDeclaration identifier='R'><correctResponse><value>b</value></correctResponse></responseDeclaration><itemBody><p>Choose the illustrated answer.</p><choiceInteraction responseIdentifier='R' maxChoices='1'><simpleChoice identifier='a'>A</simpleChoice><simpleChoice identifier='b'><img src='../assets/choice.png' alt='choice diagram'/>B</simpleChoice></choiceInteraction></itemBody></assessmentItem>";
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("imsmanifest.xml", manifest.as_bytes()),
        ("items/choice.xml", item.as_bytes()),
        ("assets/choice.png", image_bytes.as_slice()),
    ] {
        writer
            .start_file(path, zip::write::SimpleFileOptions::default())
            .expect("choice-image QTI fixture starts entry");
        std::io::Write::write_all(&mut writer, bytes)
            .expect("choice-image QTI fixture writes entry");
    }
    writer
        .finish()
        .expect("choice-image QTI fixture finishes")
        .into_inner()
}

fn still_png(color: [u8; 3]) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(2, 1, image::Rgb(color));
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgb8,
        )
        .expect("choice-image PNG fixture encodes");
    bytes
}

#[test]
fn choice_body_images_are_part_of_the_published_qti_asset_contract() {
    let prompt_asset = AssetId::from_uuid(uuid::Uuid::from_u128(7_100));
    let choice_asset = AssetId::from_uuid(uuid::Uuid::from_u128(7_101));
    let prompt = vec![ContentBlock::Image {
        asset: AssetRef {
            asset: prompt_asset,
            checksum: "a".repeat(64),
        },
        description: "prompt image".to_string(),
    }];
    let response = ResponseDefinition::MultipleChoice {
        choices: vec![ChoiceOption {
            id: ChoiceId::new("choice-a"),
            body: vec![ContentBlock::Image {
                asset: AssetRef {
                    asset: choice_asset,
                    checksum: "b".repeat(64),
                },
                description: "choice image".to_string(),
            }],
        }],
        selection: question_model::answer::SelectionCardinality::Exactly { count: 1 },
    };
    assert_eq!(
        qti_question_asset_checksums(&adapter_qti::ImportedQtiQuestion {
            item_id: "choice-image".to_string(),
            prompt,
            response,
        })
        .expect("distinct image references"),
        std::collections::BTreeMap::from([
            (prompt_asset, "a".repeat(64)),
            (choice_asset, "b".repeat(64)),
        ])
    );
}
