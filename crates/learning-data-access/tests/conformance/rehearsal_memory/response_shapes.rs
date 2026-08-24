//! Store-boundary response-family coverage for rehearsal frozen items.

use super::*;
use question_model::AssetId;
use question_model::answer::{SelectionCardinality, TextMatchMode};
use question_model::envelope::{AssetRef, ContentBlock};
use question_model::response::{
    ChoiceId, ChoiceOption, HotspotPoint, HotspotRegion, MatchPair, TextEntryAnswer, TextEntrySlot,
};

fn option(id: &str) -> ChoiceOption {
    ChoiceOption {
        id: ChoiceId::new(id),
        body: vec![ContentBlock::Text {
            markdown: id.into(),
        }],
    }
}

fn frozen(
    base: &RehearsalFrozenItemEvidence,
    attempt: u128,
    response_definition: ResponseDefinition,
) -> RehearsalFrozenItemEvidence {
    RehearsalFrozenItemEvidence {
        attempt: RehearsalAttemptId::from_uuid(uuid(attempt)),
        response_definition,
        ..base.clone()
    }
}

#[tokio::test]
async fn every_supported_native_response_shape_claims_through_the_frozen_rehearsal_boundary() {
    let store = MemoryStore::default();
    let (fixture, locator, base) = start_and_freeze(&store).await;
    let choice_a = ChoiceId::new("a");
    let choice_b = ChoiceId::new("b");
    let cases = vec![
        (
            ResponseDefinition::Numeric {
                tolerance: NumericTolerance::Exact,
                unit: None,
            },
            StudentResponse::Numeric { value: 3.0 },
        ),
        (
            ResponseDefinition::MultipleChoice {
                choices: vec![option("a"), option("b")],
                selection: SelectionCardinality::ExactlyOne,
            },
            StudentResponse::MultipleChoice {
                selected: vec![choice_a.clone()],
            },
        ),
        (
            ResponseDefinition::ShortText {
                match_mode: TextMatchMode::Normalized,
                max_length: 32,
            },
            StudentResponse::ShortText {
                text: "answer".into(),
            },
        ),
        (
            ResponseDefinition::MultiBlank {
                blanks: vec![TextEntrySlot {
                    id: choice_a.clone(),
                    label: vec![ContentBlock::Text {
                        markdown: "first".into(),
                    }],
                    match_mode: TextMatchMode::Exact,
                    max_length: 32,
                }],
            },
            StudentResponse::MultiBlank {
                answers: vec![TextEntryAnswer {
                    slot: choice_a.clone(),
                    text: "answer".into(),
                }],
            },
        ),
        (
            ResponseDefinition::Matching {
                prompts: vec![option("prompt")],
                choices: vec![option("choice")],
            },
            StudentResponse::Matching {
                matches: vec![MatchPair {
                    prompt: ChoiceId::new("prompt"),
                    choice: ChoiceId::new("choice"),
                }],
            },
        ),
        (
            ResponseDefinition::Ordering {
                items: vec![option("a"), option("b")],
            },
            StudentResponse::Ordering {
                order: vec![choice_a.clone(), choice_b],
            },
        ),
        (
            ResponseDefinition::Hotspot {
                surface: AssetRef {
                    asset: AssetId::from_uuid(uuid(860_001)),
                    checksum: "safe-test-checksum".into(),
                },
                description: "one labeled region".into(),
                regions: vec![HotspotRegion {
                    id: ChoiceId::new("region"),
                    label: vec![ContentBlock::Text {
                        markdown: "region".into(),
                    }],
                    x: 0,
                    y: 0,
                    width: 10_000,
                    height: 10_000,
                }],
                selection: SelectionCardinality::ExactlyOne,
            },
            StudentResponse::Hotspot {
                points: vec![HotspotPoint { x: 5_000, y: 5_000 }],
            },
        ),
    ];
    let before = store
        .rehearsal_state_effect_fingerprint()
        .expect("baseline");
    for (offset, (definition, response)) in cases.into_iter().enumerate() {
        let frozen = frozen(
            &base,
            860_100 + u128::try_from(offset).expect("fixture offset"),
            definition,
        );
        store
            .append_rehearsal_frozen_item(
                fixture.context,
                AppendRehearsalFrozenItemCommand {
                    locator,
                    frozen: frozen.clone(),
                },
            )
            .await
            .expect("freeze supported response family");
        assert!(matches!(
            store
                .claim_rehearsal_submission(
                    fixture.context,
                    ClaimRehearsalSubmissionCommand {
                        locator,
                        attempt: frozen.attempt,
                        response,
                        idempotency_key: RehearsalSubmissionIdempotencyKey::new(format!(
                            "native-family-{offset}"
                        ))
                        .expect("key"),
                    },
                )
                .await
                .expect("claim supported response family"),
            RehearsalSubmissionClaimResult::Claimed(_)
        ));
    }
    assert!(
        store
            .rehearsal_state_effect_fingerprint()
            .expect("after response families")
            .has_only_rehearsal_effects_from(&before)
    );
}

#[tokio::test]
async fn upload_and_external_response_definitions_refuse_before_frozen_delivery_evidence() {
    let store = MemoryStore::default();
    let (fixture, locator, base) = start_and_freeze(&store).await;
    let cases = [
        (
            ResponseDefinition::FileUpload {
                max_bytes: 128,
                accepted_extensions: vec!["txt".into()],
            },
            StudentResponse::FileUpload {
                object_key: "student-records/never-accepted".into(),
            },
        ),
        (
            ResponseDefinition::ExternalTool {},
            StudentResponse::ExternalTool {},
        ),
    ];
    let before = store
        .rehearsal_state_effect_fingerprint()
        .expect("before unsupported delivery");
    for (offset, (definition, response)) in cases.into_iter().enumerate() {
        let frozen = frozen(
            &base,
            860_200 + u128::try_from(offset).expect("fixture offset"),
            definition,
        );
        assert!(
            store
                .append_rehearsal_frozen_item(
                    fixture.context,
                    AppendRehearsalFrozenItemCommand {
                        locator,
                        frozen: frozen.clone(),
                    },
                )
                .await
                .is_err(),
            "unsupported family is refused before frozen delivery evidence"
        );
        assert!(
            store
                .claim_rehearsal_submission(
                    fixture.context,
                    ClaimRehearsalSubmissionCommand {
                        locator,
                        attempt: frozen.attempt,
                        response,
                        idempotency_key: RehearsalSubmissionIdempotencyKey::new(format!(
                            "unsupported-family-{offset}"
                        ))
                        .expect("key"),
                    },
                )
                .await
                .is_err(),
            "unsupported response family cannot create a rehearsal claim"
        );
        assert_eq!(
            store
                .rehearsal_state_effect_fingerprint()
                .expect("after unsupported delivery"),
            before,
            "unsupported response body and delivery leave no rehearsal mutation"
        );
    }
}
