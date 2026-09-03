use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use grading::QuestionGradingOutcome;
use objects::ObjectAddress;
use objects::Sha256Checksum;
use objects::memory::MemoryObjectStore;
use question_model::QuestionContentBlock;
use question_model::answer::ResponseSelectionRule;
use question_model::capability::Capability;
use question_model::generation::QuestionSeed;
use question_model::response::{QuestionChoice, QuestionResponseFormat, ResponseItemReference};
use question_model::{
    ObjectId, QuestionBackend, QuestionEvaluation, QuestionId, QuestionRendererVersion,
    QuestionRevisionNumber, QuestionRevisionReference, QuestionVariation, SourceObjectChecksum,
    SourceObjectReference,
};
use uuid::Uuid;

use super::*;
use crate::WebworkQuestionSourceBinding;
use crate::renderer_contract::{
    GradeRequest, RenderedWebworkQuestion, WebworkQuestionAttemptReplayDetails,
    WebworkUpstreamControl,
};

const OPL_FIXTURE: &str = concat!(
    "## Recorded OPL-style example: a small multiple-choice PG question.\n",
    "DOCUMENT();\n",
    "loadMacros(\"PGstandard.pl\", \"PGchoicemacros.pl\");\n",
    "BEGIN_TEXT\n",
    "Which molecule is water?\n",
    "END_TEXT\n",
    "$showPartialCorrectAnswers = 0;\n",
    "ANS(str_cmp(\"H2O\"));\n",
    "ENDDOCUMENT();\n",
);

fn question_revision(number: u32) -> QuestionRevisionReference {
    QuestionRevisionReference {
        question_id: QuestionId::from_canonical_parts("ABCDEF", 'G').expect("Question ID"),
        revision_number: QuestionRevisionNumber::new(number).expect("positive version"),
    }
}

#[derive(Clone)]
struct RecordedRenderer {
    calls: Arc<AtomicUsize>,
    failure: Option<RendererFailure>,
    identity: QuestionRendererVersion,
}

#[async_trait]
impl WebworkRenderer for RecordedRenderer {
    fn identity(&self) -> &QuestionRendererVersion {
        &self.identity
    }

    async fn render(
        &self,
        request: RenderRequest<'_>,
    ) -> Result<RenderedWebworkQuestion, RendererFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        if request.pg_source != OPL_FIXTURE.as_bytes()
            || request.pg_path != "Library/OPL/select-one.pg"
        {
            return Err(RendererFailure::InvalidOutput(
                "recorded fixture source did not match request".to_string(),
            ));
        }
        Ok(RenderedWebworkQuestion {
            presentation: QuestionVariationPresentation {
                variation: QuestionVariation::from_question_revision_and_seed(
                    request.question_revision.clone(),
                    QuestionSeed::new(request.seed),
                ),
                title: "Untrusted renderer title".to_string(),
                prompt: vec![QuestionContentBlock::Text {
                    markdown: "Which molecule is water?".to_string(),
                }],
                response: QuestionResponseFormat::MultipleChoice {
                    choices: vec![
                        QuestionChoice {
                            id: ResponseItemReference::new("water"),
                            body: vec![QuestionContentBlock::Text {
                                markdown: "H&#x2082;O".to_string(),
                            }],
                        },
                        QuestionChoice {
                            id: ResponseItemReference::new("oxygen"),
                            body: vec![QuestionContentBlock::Text {
                                markdown: "O&#x2082;".to_string(),
                            }],
                        },
                    ],
                    selection: ResponseSelectionRule::ExactlyOne,
                },
            },
            renderer_version: self.identity.clone(),
            replay: Some(recorded_replay()),
        })
    }

    async fn grade(
        &self,
        request: GradeRequest<'_>,
    ) -> Result<QuestionGradingOutcome, RendererFailure> {
        if request.pg_source != OPL_FIXTURE.as_bytes()
            || request.pg_path != "Library/OPL/select-one.pg"
            || request.replay != &recorded_replay()
        {
            return Err(RendererFailure::InvalidOutput(
                "recorded grade request did not match issuance".to_string(),
            ));
        }
        let selected_water = matches!(
            request.response,
            StudentResponse::MultipleChoice { selected }
                if selected == &[ResponseItemReference::new("water")]
        );
        Ok(QuestionGradingOutcome::Evaluated(
            QuestionEvaluation::new(selected_water, f64::from(selected_water))
                .expect("fixed evaluation is valid"),
        ))
    }
}

fn recorded_replay() -> WebworkQuestionAttemptReplayDetails {
    WebworkQuestionAttemptReplayDetails::SingleChoice {
        controls: [
            (
                ResponseItemReference::new("water"),
                WebworkUpstreamControl {
                    field: "AnSwEr0001".into(),
                    value: "0".into(),
                },
            ),
            (
                ResponseItemReference::new("oxygen"),
                WebworkUpstreamControl {
                    field: "AnSwEr0001".into(),
                    value: "1".into(),
                },
            ),
        ]
        .into_iter()
        .collect(),
    }
}

fn recorded_renderer(calls: Arc<AtomicUsize>) -> RecordedRenderer {
    RecordedRenderer {
        calls,
        failure: None,
        identity: QuestionRendererVersion {
            name: "recorded-opl-renderer".to_string(),
            version: "1".to_string(),
        },
    }
}

fn binding(number: u32) -> WebworkQuestionSourceBinding {
    WebworkQuestionSourceBinding::new(
        question_revision(number),
        "Library/OPL/select-one.pg".to_string(),
    )
    .expect("fixed PG path is valid")
}

async fn source(
    store: &MemoryObjectStore,
    binding: WebworkQuestionSourceBinding,
) -> ResolvedWebworkQuestionSource {
    let source_object_reference = SourceObjectReference {
        object: ObjectId::from_uuid(Uuid::from_u128(4)),
    };
    let source_object_checksum =
        SourceObjectChecksum::parse(Sha256Checksum::compute(OPL_FIXTURE.as_bytes()).to_string())
            .expect("computed checksum is canonical");
    store
        .put(PutObject {
            address: ObjectAddress::QuestionSource {
                question_revision: binding.question_revision().clone(),
                object: source_object_reference.object,
            },
            bytes: OPL_FIXTURE.as_bytes().to_vec(),
            media_type: "text/x-wework-pg".to_string(),
            created_at: Timestamp::from_unix_millis(1),
        })
        .await
        .expect("fixture source should be stored under its immutable key");
    ResolvedWebworkQuestionSource::resolve(
        store,
        binding,
        source_object_reference,
        source_object_checksum,
    )
    .await
    .expect("fixture source should resolve through trusted storage")
}

#[tokio::test]
async fn recorded_opl_fixture_renders_and_grades_through_the_shared_model() {
    let calls = Arc::new(AtomicUsize::new(0));
    let store = MemoryObjectStore::default();
    let adapter = WebworkAdapter::new(store.clone(), recorded_renderer(calls.clone()));
    let source = source(&store, binding(2)).await;
    let issued = adapter
        .issue(
            QuestionSeed::new(17),
            &source,
            Timestamp::from_unix_millis(1),
        )
        .await
        .expect("recorded OPL fixture should render");
    assert!(!issued.cache_hit);
    assert_eq!(
        issued.presentation.variation.question_seed,
        QuestionSeed::new(17)
    );
    assert_eq!(issued.presentation.title, "Untrusted renderer title");
    assert!(
        !serde_json::to_string(&issued.presentation)
            .expect("browser Question Presentation serializes")
            .contains("\"correct\"")
    );

    let correct = adapter
        .grade(
            QuestionSeed::new(17),
            &source,
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("water")],
            },
            issued.replay.as_ref().expect("issued replay state"),
        )
        .await
        .expect("renderer should grade server-side");
    assert!(matches!(correct, QuestionGradingOutcome::Evaluated(result) if result.correct()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn registered_pg_path_refuses_unsafe_private_routing() {
    for path in [
        "/unsafe.pg",
        "../outside.pg",
        "a/../b.pg",
        "a//b.pg",
        "./a.pg",
    ] {
        assert_eq!(
            WebworkQuestionSourceBinding::new(question_revision(2), path.to_string()),
            Err(WebworkAdapterError::InvalidPgPath)
        );
    }
}

#[tokio::test]
async fn repeated_version_and_seed_are_served_without_a_renderer_call() {
    let calls = Arc::new(AtomicUsize::new(0));
    let store = MemoryObjectStore::default();
    let adapter = WebworkAdapter::new(store.clone(), recorded_renderer(calls.clone()));
    let source = source(&store, binding(2)).await;
    let first = adapter
        .issue(
            QuestionSeed::new(18),
            &source,
            Timestamp::from_unix_millis(1),
        )
        .await
        .expect("first render should fill the cache");
    let second = adapter
        .reproduce(QuestionSeed::new(18), &source)
        .await
        .expect("second request should use the cache");
    assert!(!first.cache_hit);
    assert!(second.cache_hit);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(first.presentation, second.presentation);
    assert_eq!(second.presentation.title, "Untrusted renderer title");
}

#[test]
fn cache_boundary_emits_one_renderer_call_then_one_cache_hit() {
    let _ = take_test_cache_events();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("current-thread runtime builds");
    let (first, second, calls) = runtime.block_on(async {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = MemoryObjectStore::default();
        let adapter = WebworkAdapter::new(store.clone(), recorded_renderer(calls.clone()));
        let source = source(&store, binding(2)).await;
        let first = adapter
            .issue(
                QuestionSeed::new(181),
                &source,
                Timestamp::from_unix_millis(1),
            )
            .await
            .expect("first render should fill the cache");
        let second = adapter
            .reproduce(QuestionSeed::new(181), &source)
            .await
            .expect("second render should use the verified cache");
        (first, second, calls.load(Ordering::SeqCst))
    });
    assert!(!first.cache_hit);
    assert!(second.cache_hit);
    assert_eq!(calls, 1);
    assert_eq!(take_test_cache_events(), ["renderer_call", "cache_hit"]);
}

#[tokio::test]
async fn renderer_outage_is_an_explicit_backend_local_failure() {
    let calls = Arc::new(AtomicUsize::new(0));
    let store = MemoryObjectStore::default();
    let adapter = WebworkAdapter::new(
        store.clone(),
        RecordedRenderer {
            failure: Some(RendererFailure::TimedOut),
            ..recorded_renderer(calls.clone())
        },
    );
    let source = source(&store, binding(2)).await;
    assert_eq!(
        adapter
            .issue(
                QuestionSeed::new(19),
                &source,
                Timestamp::from_unix_millis(1),
            )
            .await,
        Err(WebworkAdapterError::Renderer(RendererFailure::TimedOut))
    );
    assert!(
        adapter
            .capabilities(&source)
            .expect("WeBWorK capability declaration remains available")
            .supports(Capability::ServerGrading)
    );
}

#[tokio::test]
async fn cache_reuse_refuses_a_different_active_renderer_without_calling_it() {
    let store = MemoryObjectStore::default();
    let source = source(&store, binding(2)).await;
    let first_calls = Arc::new(AtomicUsize::new(0));
    let first_renderer = RecordedRenderer {
        identity: QuestionRendererVersion {
            name: "renderer-a".to_string(),
            version: "1".to_string(),
        },
        ..recorded_renderer(first_calls.clone())
    };
    let first_adapter = WebworkAdapter::new(store.clone(), first_renderer);
    let first = first_adapter
        .issue(
            QuestionSeed::new(21),
            &source,
            Timestamp::from_unix_millis(1),
        )
        .await
        .expect("first renderer should populate cache");

    let second_calls = Arc::new(AtomicUsize::new(0));
    let second_renderer = RecordedRenderer {
        identity: QuestionRendererVersion {
            name: "renderer-b".to_string(),
            version: "2".to_string(),
        },
        ..recorded_renderer(second_calls.clone())
    };
    let second_adapter = WebworkAdapter::new(store, second_renderer);
    let result = second_adapter
        .reproduce(QuestionSeed::new(21), &source)
        .await;
    assert!(!first.cache_hit);
    assert_eq!(first_calls.load(Ordering::SeqCst), 1);
    assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    assert!(matches!(result, Err(WebworkAdapterError::InvalidCache(_))));
}

#[tokio::test]
async fn source_resolution_refuses_digest_and_published_key_mismatches() {
    let store = MemoryObjectStore::default();
    let trusted = source(&store, binding(2)).await;
    let wrong_checksum =
        SourceObjectChecksum::parse("00".repeat(32)).expect("fixed checksum is canonical");
    assert_eq!(
        ResolvedWebworkQuestionSource::resolve(
            &store,
            binding(2),
            trusted.source_object_reference().clone(),
            wrong_checksum,
        )
        .await,
        Err(WebworkAdapterError::UntrustedSource)
    );
    assert_eq!(
        ResolvedWebworkQuestionSource::resolve(
            &store,
            WebworkQuestionSourceBinding::new(
                QuestionRevisionReference {
                    question_id: QuestionId::from_canonical_parts("BCDEFG", 'H')
                        .expect("Question ID"),
                    revision_number: QuestionRevisionNumber::new(2).expect("positive version"),
                },
                "Library/OPL/select-one.pg".to_string(),
            )
            .expect("fixed PG path is valid"),
            trusted.source_object_reference().clone(),
            trusted.source_object_checksum().clone(),
        )
        .await,
        Err(WebworkAdapterError::ObjectStore(ObjectStoreError::NotFound))
    );
}

#[test]
fn reviewed_chapter_matching_sources_claim_partial_credit_without_widening_near_misses() {
    for (pg_path, source_sha256) in [
        (
            "content/pilot/sources/genetics/genetic_disorders-matching.pgml",
            "ae59425dce95bbffe0992aa5e072cd01370b736ef958685e409004d7580d2718",
        ),
        (
            "content/pilot/sources/biochemistry/biochemical_functional_groups-matching.pgml",
            "42c52281516511410623e56a315ed74f687f412a24c6ca1d028ffbe3eab12f17",
        ),
    ] {
        let capabilities =
            reviewed_webwork_source_capabilities(QuestionBackend::Webwork, pg_path, source_sha256)
                .expect("reviewed WeBWorK source is supported");
        assert!(capabilities.supports(Capability::PartialCredit));
        assert!(
            !webwork_source_capabilities(QuestionBackend::Webwork)
                .expect("arbitrary PG retains conservative support")
                .supports(Capability::PartialCredit)
        );
        assert!(
            !reviewed_webwork_source_capabilities(
                QuestionBackend::Webwork,
                pg_path,
                &"0".repeat(64)
            )
            .expect("same-path source with different bytes retains common support")
            .supports(Capability::PartialCredit)
        );
    }
    let near_miss = reviewed_webwork_source_capabilities(
        QuestionBackend::Webwork,
        "content/pilot/sources/genetics/other-matching.pgml",
        "ae59425dce95bbffe0992aa5e072cd01370b736ef958685e409004d7580d2718",
    )
    .expect("unreviewed WeBWorK source retains common support");
    assert!(!near_miss.supports(Capability::PartialCredit));
}

#[test]
fn reviewed_chapter_sources_admit_immediate_correctness_without_widening_pg_support() {
    for (pg_path, source_sha256) in [
        (
            "content/pilot/sources/genetics/genetic_disorders-which_one.pgml",
            "810fc1ed93a5ed60ec79e94aa86ded3caebe2bdf8627fb71d6fecd7c6b4f062c",
        ),
        (
            "content/pilot/sources/genetics/genetic_disorders-matching.pgml",
            "ae59425dce95bbffe0992aa5e072cd01370b736ef958685e409004d7580d2718",
        ),
        (
            "content/pilot/sources/biochemistry/biochemical_functional_groups-which_one.pgml",
            "7e27357885fc8d71410bf42431105a515bdc75a776359a2d02013813e362b5fa",
        ),
        (
            "content/pilot/sources/biochemistry/biochemical_functional_groups-matching.pgml",
            "42c52281516511410623e56a315ed74f687f412a24c6ca1d028ffbe3eab12f17",
        ),
    ] {
        assert!(
            reviewed_webwork_source_profile_capabilities(
                QuestionBackend::Webwork,
                pg_path,
                source_sha256
            )
            .expect("reviewed Chapter 1 source has a capability profile")
            .supports(Capability::Hints)
        );
    }
    let historical = reviewed_webwork_source_capabilities(
        QuestionBackend::Webwork,
        "content/pilot/sources/genetics/genetic_disorders-which_one.pgml",
        "810fc1ed93a5ed60ec79e94aa86ded3caebe2bdf8627fb71d6fecd7c6b4f062c",
    )
    .expect("historical reviewed source has a conservative profile");
    assert!(!historical.supports(Capability::Hints));
    assert!(
        !reviewed_webwork_source_capabilities(
            QuestionBackend::Webwork,
            "content/pilot/sources/genetics/genetic_disorders-which_one.pgml",
            &"0".repeat(64),
        )
        .expect("changed source bytes keep only conservative capabilities")
        .supports(Capability::Hints)
    );
}

#[tokio::test]
async fn unreviewed_source_grading_returns_the_backend_evaluation() {
    let calls = Arc::new(AtomicUsize::new(0));
    let store = MemoryObjectStore::default();
    let adapter = WebworkAdapter::new(store.clone(), recorded_renderer(calls));
    let source = source(&store, binding(2)).await;
    let outcome = adapter
        .grade(
            QuestionSeed::new(17),
            &source,
            &StudentResponse::MultipleChoice {
                selected: vec![ResponseItemReference::new("water")],
            },
            &recorded_replay(),
        )
        .await
        .expect("backend evaluation should remain available");
    assert!(matches!(
        outcome,
        QuestionGradingOutcome::Evaluated(result) if result.correct()
    ));
}
