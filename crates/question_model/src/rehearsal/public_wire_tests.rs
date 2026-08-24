use super::*;
use crate::response::{ChoiceId, HotspotPoint, MatchPair, TextEntryAnswer};

fn id(value: &str) -> crate::RenderedItemIdV1 {
    crate::RenderedItemIdV1::parse(value).expect("valid rendered identifier")
}

fn choice(value: &str) -> RehearsalPresentedChoiceV1 {
    RehearsalPresentedChoiceV1 {
        id: id(value),
        body: vec![RehearsalContentBlockV1::Text {
            markdown: value.into(),
        }],
    }
}

fn screen(schema: RehearsalResponseSchemaV1) -> RehearsalActiveScreenV1 {
    let presentation = RehearsalQuestionPresentationV1 {
        title: "Bounded native question".into(),
        prompt: vec![RehearsalContentBlockV1::Text {
            markdown: "Prompt".into(),
        }],
        response: schema,
    };
    RehearsalActiveScreenV1::new(presentation).expect("valid screen")
}

fn asset() -> RehearsalAssetReferenceV1 {
    RehearsalAssetReferenceV1::parse("RA-0123456789abcdef").expect("asset")
}

fn assert_commitment_rejects(
    mut active: RehearsalActiveScreenV1,
    mutate: impl FnOnce(&mut RehearsalActiveScreenV1),
) {
    assert!(
        active.validate().is_ok(),
        "fixture must begin with its token"
    );
    mutate(&mut active);
    assert_eq!(
        active.validate(),
        Err(RehearsalWireValidationError::InvalidDigest)
    );
    assert_eq!(
        active.commitment(),
        Err(RehearsalWireValidationError::InvalidDigest)
    );
}

fn table() -> RehearsalContentBlockV1 {
    RehearsalContentBlockV1::Table {
        headers: vec!["Name".into(), "Value".into()],
        rows: vec![vec!["a".into(), "1".into()]],
        description: "table description".into(),
    }
}

fn hotspot_surface() -> RehearsalHotspotSurfaceV1 {
    RehearsalHotspotSurfaceV1 {
        id: id("0001"),
        asset: asset(),
        description: "hotspot surface".into(),
        regions: vec![
            RehearsalHotspotRegionV1 {
                id: id("0002"),
                label: vec![RehearsalContentBlockV1::Text {
                    markdown: "first region".into(),
                }],
                x: 10,
                y: 20,
                width: 30,
                height: 40,
            },
            RehearsalHotspotRegionV1 {
                id: id("0003"),
                label: vec![RehearsalContentBlockV1::Text {
                    markdown: "second region".into(),
                }],
                x: 50,
                y: 60,
                width: 30,
                height: 40,
            },
        ],
    }
}

#[test]
fn asset_tokens_are_opaque_and_canonical() {
    assert!(RehearsalAssetReferenceV1::parse("RA-0123456789abcdef").is_ok());
    for invalid in ["RA-short", "ra-0123456789abcdef", "RA-0123456789ABCDEf"] {
        assert!(
            RehearsalAssetReferenceV1::parse(invalid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn nested_unknown_content_fields_are_rejected() {
    let content = serde_json::json!({"kind":"image", "asset":"RA-0123456789abcdef", "description":"diagram", "privateId":"x"});
    assert!(serde_json::from_value::<RehearsalContentBlockV1>(content).is_err());
}

#[test]
fn active_screen_digest_covers_the_whole_presentation() {
    let mut active = screen(RehearsalResponseSchemaV1::FillIn { max_characters: 12 });
    active.presentation.title = "Changed after issue".into();
    assert_eq!(
        active.validate(),
        Err(RehearsalWireValidationError::InvalidDigest)
    );
}

#[test]
fn active_screen_commitment_rejects_each_presentation_dimension_mutation() {
    let active = screen(RehearsalResponseSchemaV1::FillIn { max_characters: 12 });
    let mut title = active.clone();
    title.presentation.title = "Changed title".into();
    assert_eq!(
        title.commitment(),
        Err(RehearsalWireValidationError::InvalidDigest)
    );
    let mut prompt = active.clone();
    prompt
        .presentation
        .prompt
        .push(RehearsalContentBlockV1::Text {
            markdown: "Changed prompt".into(),
        });
    assert_eq!(
        prompt.commitment(),
        Err(RehearsalWireValidationError::InvalidDigest)
    );
    let mut response = active;
    response.presentation.response = RehearsalResponseSchemaV1::FillIn { max_characters: 13 };
    assert_eq!(
        response.commitment(),
        Err(RehearsalWireValidationError::InvalidDigest)
    );
}

#[test]
fn active_screen_commitment_covers_every_visible_schema_field() {
    let content = screen(RehearsalResponseSchemaV1::FillIn { max_characters: 12 });
    assert_commitment_rejects(content.clone(), |active| {
        active.presentation.title = "Changed title".into();
    });
    assert_commitment_rejects(content.clone(), |active| {
        active.presentation.prompt[0] = RehearsalContentBlockV1::Math {
            latex: "x^2".into(),
            description: "equation".into(),
        };
    });
    assert_commitment_rejects(content.clone(), |active| {
        active.presentation.prompt[0] = RehearsalContentBlockV1::Code {
            language: "text".into(),
            source: "changed".into(),
        };
    });
    assert_commitment_rejects(content.clone(), |active| {
        active
            .presentation
            .prompt
            .push(RehearsalContentBlockV1::Text {
                markdown: "second prompt block".into(),
            });
    });
    let prompt_order = RehearsalActiveScreenV1::new(RehearsalQuestionPresentationV1 {
        title: "Prompt order".into(),
        prompt: vec![
            RehearsalContentBlockV1::Text {
                markdown: "first prompt block".into(),
            },
            RehearsalContentBlockV1::Text {
                markdown: "second prompt block".into(),
            },
        ],
        response: RehearsalResponseSchemaV1::FillIn { max_characters: 12 },
    })
    .expect("valid prompt order screen");
    assert_commitment_rejects(prompt_order, |active| {
        active.presentation.prompt.swap(0, 1);
    });

    let image_prompt = screen(RehearsalResponseSchemaV1::FillIn { max_characters: 12 });
    assert_commitment_rejects(image_prompt.clone(), |active| {
        active.presentation.prompt[0] = RehearsalContentBlockV1::Image {
            asset: asset(),
            description: "diagram".into(),
        };
    });
    assert_commitment_rejects(image_prompt.clone(), |active| {
        active.presentation.prompt[0] = RehearsalContentBlockV1::Image {
            asset: RehearsalAssetReferenceV1::parse("RA-fedcba9876543210").expect("asset"),
            description: "diagram".into(),
        };
    });
    assert_commitment_rejects(image_prompt, |active| {
        active.presentation.prompt[0] = RehearsalContentBlockV1::Image {
            asset: asset(),
            description: "revised diagram".into(),
        };
    });

    let table_prompt = screen(RehearsalResponseSchemaV1::FillIn { max_characters: 12 });
    assert_commitment_rejects(table_prompt.clone(), |active| {
        active.presentation.prompt[0] = table();
    });
    assert_commitment_rejects(table_prompt.clone(), |active| {
        active.presentation.prompt[0] = RehearsalContentBlockV1::Table {
            headers: vec!["Changed".into(), "Value".into()],
            rows: vec![vec!["a".into(), "1".into()]],
            description: "table description".into(),
        };
    });
    assert_commitment_rejects(table_prompt.clone(), |active| {
        active.presentation.prompt[0] = RehearsalContentBlockV1::Table {
            headers: vec!["Name".into(), "Value".into()],
            rows: vec![vec!["b".into(), "1".into()]],
            description: "table description".into(),
        };
    });
    assert_commitment_rejects(table_prompt, |active| {
        active.presentation.prompt[0] = RehearsalContentBlockV1::Table {
            headers: vec!["Name".into(), "Value".into()],
            rows: vec![vec!["a".into(), "1".into()]],
            description: "revised table description".into(),
        };
    });

    let choices = screen(RehearsalResponseSchemaV1::MultipleAnswer {
        choices: vec![choice("0001"), choice("0002")],
        minimum: 1,
        maximum: 2,
    });
    assert_commitment_rejects(choices.clone(), |active| {
        if let RehearsalResponseSchemaV1::MultipleAnswer { choices, .. } =
            &mut active.presentation.response
        {
            choices[0].id = id("0003");
        }
    });
    assert_commitment_rejects(choices.clone(), |active| {
        if let RehearsalResponseSchemaV1::MultipleAnswer { choices, .. } =
            &mut active.presentation.response
        {
            choices[0].body[0] = RehearsalContentBlockV1::Text {
                markdown: "changed choice".into(),
            };
        }
    });
    assert_commitment_rejects(choices, |active| {
        if let RehearsalResponseSchemaV1::MultipleAnswer { choices, .. } =
            &mut active.presentation.response
        {
            choices.swap(0, 1);
        }
    });

    let blanks = screen(RehearsalResponseSchemaV1::MultiFillIn {
        blanks: vec![
            RehearsalPresentedBlankV1 {
                id: id("0001"),
                label: vec![RehearsalContentBlockV1::Text {
                    markdown: "first".into(),
                }],
                max_characters: 4,
            },
            RehearsalPresentedBlankV1 {
                id: id("0002"),
                label: vec![RehearsalContentBlockV1::Text {
                    markdown: "second".into(),
                }],
                max_characters: 4,
            },
        ],
    });
    assert_commitment_rejects(blanks.clone(), |active| {
        if let RehearsalResponseSchemaV1::MultiFillIn { blanks } = &mut active.presentation.response
        {
            blanks[0].id = id("0003");
        }
    });
    assert_commitment_rejects(blanks.clone(), |active| {
        if let RehearsalResponseSchemaV1::MultiFillIn { blanks } = &mut active.presentation.response
        {
            blanks[0].label[0] = RehearsalContentBlockV1::Text {
                markdown: "changed label".into(),
            };
        }
    });
    assert_commitment_rejects(blanks, |active| {
        if let RehearsalResponseSchemaV1::MultiFillIn { blanks } = &mut active.presentation.response
        {
            blanks[0].max_characters = 5;
        }
    });

    let numeric = screen(RehearsalResponseSchemaV1::Numerical {
        max_characters: 12,
        displayed_unit: Some("mM".into()),
    });
    assert_commitment_rejects(numeric, |active| {
        if let RehearsalResponseSchemaV1::Numerical { displayed_unit, .. } =
            &mut active.presentation.response
        {
            *displayed_unit = Some("uM".into());
        }
    });

    let matching = screen(RehearsalResponseSchemaV1::Matching {
        prompts: vec![choice("0001"), choice("0002")],
        choices: vec![choice("0003"), choice("0004")],
        reuse_choices: false,
    });
    assert_commitment_rejects(matching.clone(), |active| {
        if let RehearsalResponseSchemaV1::Matching { prompts, .. } =
            &mut active.presentation.response
        {
            prompts[0].body[0] = RehearsalContentBlockV1::Text {
                markdown: "changed prompt".into(),
            };
        }
    });
    assert_commitment_rejects(matching.clone(), |active| {
        if let RehearsalResponseSchemaV1::Matching { choices, .. } =
            &mut active.presentation.response
        {
            choices[0].body[0] = RehearsalContentBlockV1::Text {
                markdown: "changed choice".into(),
            };
        }
    });
    assert_commitment_rejects(matching.clone(), |active| {
        if let RehearsalResponseSchemaV1::Matching { reuse_choices, .. } =
            &mut active.presentation.response
        {
            *reuse_choices = true;
        }
    });
    assert_commitment_rejects(matching.clone(), |active| {
        if let RehearsalResponseSchemaV1::Matching { prompts, .. } =
            &mut active.presentation.response
        {
            prompts.swap(0, 1);
        }
    });
    assert_commitment_rejects(matching, |active| {
        if let RehearsalResponseSchemaV1::Matching { choices, .. } =
            &mut active.presentation.response
        {
            choices.swap(0, 1);
        }
    });

    let ordering = screen(RehearsalResponseSchemaV1::Ordering {
        items: vec![choice("0001"), choice("0002")],
    });
    assert_commitment_rejects(ordering, |active| {
        if let RehearsalResponseSchemaV1::Ordering { items } = &mut active.presentation.response {
            items.swap(0, 1);
        }
    });

    let hotspot = screen(RehearsalResponseSchemaV1::Hotspot {
        surface: hotspot_surface(),
        minimum: 1,
        maximum: 2,
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.id = id("0004");
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.asset = RehearsalAssetReferenceV1::parse("RA-fedcba9876543210").expect("asset");
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.description = "changed surface".into();
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.regions[0].id = id("0004");
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.regions[0].label[0] = RehearsalContentBlockV1::Text {
                markdown: "changed region label".into(),
            };
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.regions[0].x = 11;
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.regions[0].y = 21;
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.regions[0].width = 31;
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { surface, .. } =
            &mut active.presentation.response
        {
            surface.regions[0].height = 41;
        }
    });
    assert_commitment_rejects(hotspot.clone(), |active| {
        if let RehearsalResponseSchemaV1::Hotspot { minimum, .. } =
            &mut active.presentation.response
        {
            *minimum = 0;
        }
    });
    assert_commitment_rejects(hotspot, |active| {
        if let RehearsalResponseSchemaV1::Hotspot { maximum, .. } =
            &mut active.presentation.response
        {
            *maximum = 1;
        }
    });
}

#[test]
fn visible_content_allows_normal_layout_but_rejects_other_controls() {
    let valid = RehearsalQuestionPresentationV1 {
        title: "valid".into(),
        prompt: vec![RehearsalContentBlockV1::Code {
            language: "text".into(),
            source: "a\n\tb".into(),
        }],
        response: RehearsalResponseSchemaV1::FillIn { max_characters: 4 },
    };
    assert!(valid.validate().is_ok());
    let invalid = RehearsalQuestionPresentationV1 {
        title: "valid\u{0007}".into(),
        prompt: vec![],
        response: RehearsalResponseSchemaV1::FillIn { max_characters: 4 },
    };
    assert_eq!(
        invalid.validate(),
        Err(RehearsalWireValidationError::InvalidPresentation)
    );
}

#[test]
fn selection_rejects_unknown_duplicates_and_wrong_cardinality() {
    let active = screen(RehearsalResponseSchemaV1::MultipleAnswer {
        choices: vec![choice("0001"), choice("0002")],
        minimum: 1,
        maximum: 2,
    });
    for selected in [
        vec![ChoiceId::new("ffff")],
        vec![ChoiceId::new("0001"), ChoiceId::new("0001")],
        Vec::new(),
    ] {
        let request = RehearsalSubmissionRequestV1 {
            presentation_digest: active.presentation_digest.clone(),
            response: crate::StudentResponse::MultipleChoice { selected },
        };
        assert_eq!(
            request.validate_for_screen(&active),
            Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
        );
    }
}

#[test]
fn text_and_numeric_bounds_are_enforced_before_grading() {
    let text = screen(RehearsalResponseSchemaV1::FillIn { max_characters: 2 });
    let request = RehearsalSubmissionRequestV1 {
        presentation_digest: text.presentation_digest.clone(),
        response: crate::StudentResponse::ShortText { text: "abc".into() },
    };
    assert_eq!(
        request.validate_for_screen(&text),
        Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
    );
    let numeric = screen(RehearsalResponseSchemaV1::Numerical {
        max_characters: 4,
        displayed_unit: None,
    });
    let request = RehearsalSubmissionRequestV1 {
        presentation_digest: numeric.presentation_digest.clone(),
        response: crate::StudentResponse::Numeric { value: f64::NAN },
    };
    assert_eq!(
        request.validate_for_screen(&numeric),
        Err(RehearsalWireValidationError::NonFiniteNumericResponse)
    );
}

#[test]
fn blanks_require_the_exact_issued_slot_set() {
    let active = screen(RehearsalResponseSchemaV1::MultiFillIn {
        blanks: vec![
            RehearsalPresentedBlankV1 {
                id: id("0001"),
                label: vec![],
                max_characters: 4,
            },
            RehearsalPresentedBlankV1 {
                id: id("0002"),
                label: vec![],
                max_characters: 4,
            },
        ],
    });
    let answers = vec![
        TextEntryAnswer {
            slot: ChoiceId::new("0001"),
            text: "a".into(),
        },
        TextEntryAnswer {
            slot: ChoiceId::new("0001"),
            text: "b".into(),
        },
    ];
    let request = RehearsalSubmissionRequestV1 {
        presentation_digest: active.presentation_digest.clone(),
        response: crate::StudentResponse::MultiBlank { answers },
    };
    assert_eq!(
        request.validate_for_screen(&active),
        Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
    );
}

#[test]
fn matching_requires_each_prompt_once_and_respects_choice_reuse() {
    let active = screen(RehearsalResponseSchemaV1::Matching {
        prompts: vec![choice("0001"), choice("0002")],
        choices: vec![choice("0003"), choice("0004")],
        reuse_choices: false,
    });
    let matches = vec![
        MatchPair {
            prompt: ChoiceId::new("0001"),
            choice: ChoiceId::new("0003"),
        },
        MatchPair {
            prompt: ChoiceId::new("0002"),
            choice: ChoiceId::new("0003"),
        },
    ];
    let request = RehearsalSubmissionRequestV1 {
        presentation_digest: active.presentation_digest.clone(),
        response: crate::StudentResponse::Matching { matches },
    };
    assert_eq!(
        request.validate_for_screen(&active),
        Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
    );
}

#[test]
fn ordering_requires_a_permutation_of_issued_items() {
    let active = screen(RehearsalResponseSchemaV1::Ordering {
        items: vec![choice("0001"), choice("0002")],
    });
    let request = RehearsalSubmissionRequestV1 {
        presentation_digest: active.presentation_digest.clone(),
        response: crate::StudentResponse::Ordering {
            order: vec![ChoiceId::new("0001"), ChoiceId::new("0001")],
        },
    };
    assert_eq!(
        request.validate_for_screen(&active),
        Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
    );
}

#[test]
fn hotspot_rejects_points_outside_regions_and_accepts_edges() {
    let surface = RehearsalHotspotSurfaceV1 {
        id: id("0001"),
        asset: RehearsalAssetReferenceV1::parse("RA-0123456789abcdef").expect("asset"),
        description: "surface".into(),
        regions: vec![RehearsalHotspotRegionV1 {
            id: id("0002"),
            label: vec![],
            x: 10,
            y: 10,
            width: 10,
            height: 10,
        }],
    };
    let active = screen(RehearsalResponseSchemaV1::Hotspot {
        surface,
        minimum: 1,
        maximum: 1,
    });
    let request = RehearsalSubmissionRequestV1 {
        presentation_digest: active.presentation_digest.clone(),
        response: crate::StudentResponse::Hotspot {
            points: vec![HotspotPoint { x: 99, y: 99 }],
        },
    };
    assert_eq!(
        request.validate_for_screen(&active),
        Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
    );
    let request = RehearsalSubmissionRequestV1 {
        presentation_digest: active.presentation_digest.clone(),
        response: crate::StudentResponse::Hotspot {
            points: vec![HotspotPoint { x: 20, y: 20 }],
        },
    };
    assert!(request.validate_for_screen(&active).is_ok());
}

#[test]
fn rendered_submission_boundary_admits_each_supported_family_and_rejects_unsupported_responses() {
    let matching_prompts = vec![choice("0001"), choice("0002")];
    let matching_choices = vec![choice("0003"), choice("0004")];
    let cases = vec![
        (
            "single choice",
            RehearsalResponseSchemaV1::SingleChoice {
                choices: vec![choice("0001"), choice("0002")],
            },
            crate::StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("0001")],
            },
        ),
        (
            "multiple choice",
            RehearsalResponseSchemaV1::MultipleAnswer {
                choices: vec![choice("0001"), choice("0002")],
                minimum: 1,
                maximum: 2,
            },
            crate::StudentResponse::MultipleChoice {
                selected: vec![ChoiceId::new("0001"), ChoiceId::new("0002")],
            },
        ),
        (
            "fill in",
            RehearsalResponseSchemaV1::FillIn { max_characters: 4 },
            crate::StudentResponse::ShortText {
                text: "text".into(),
            },
        ),
        (
            "numeric",
            RehearsalResponseSchemaV1::Numerical {
                max_characters: 4,
                displayed_unit: None,
            },
            crate::StudentResponse::Numeric { value: 12.0 },
        ),
        (
            "multi fill in",
            RehearsalResponseSchemaV1::MultiFillIn {
                blanks: vec![
                    RehearsalPresentedBlankV1 {
                        id: id("0001"),
                        label: vec![],
                        max_characters: 2,
                    },
                    RehearsalPresentedBlankV1 {
                        id: id("0002"),
                        label: vec![],
                        max_characters: 2,
                    },
                ],
            },
            crate::StudentResponse::MultiBlank {
                answers: vec![
                    TextEntryAnswer {
                        slot: ChoiceId::new("0001"),
                        text: "a".into(),
                    },
                    TextEntryAnswer {
                        slot: ChoiceId::new("0002"),
                        text: "b".into(),
                    },
                ],
            },
        ),
        (
            "matching without choice reuse",
            RehearsalResponseSchemaV1::Matching {
                prompts: matching_prompts.clone(),
                choices: matching_choices.clone(),
                reuse_choices: false,
            },
            crate::StudentResponse::Matching {
                matches: vec![
                    MatchPair {
                        prompt: ChoiceId::new("0001"),
                        choice: ChoiceId::new("0003"),
                    },
                    MatchPair {
                        prompt: ChoiceId::new("0002"),
                        choice: ChoiceId::new("0004"),
                    },
                ],
            },
        ),
        (
            "matching with choice reuse",
            RehearsalResponseSchemaV1::Matching {
                prompts: matching_prompts,
                choices: vec![choice("0003")],
                reuse_choices: true,
            },
            crate::StudentResponse::Matching {
                matches: vec![
                    MatchPair {
                        prompt: ChoiceId::new("0001"),
                        choice: ChoiceId::new("0003"),
                    },
                    MatchPair {
                        prompt: ChoiceId::new("0002"),
                        choice: ChoiceId::new("0003"),
                    },
                ],
            },
        ),
        (
            "ordering",
            RehearsalResponseSchemaV1::Ordering {
                items: vec![choice("0001"), choice("0002")],
            },
            crate::StudentResponse::Ordering {
                order: vec![ChoiceId::new("0002"), ChoiceId::new("0001")],
            },
        ),
        (
            "hotspot",
            RehearsalResponseSchemaV1::Hotspot {
                surface: hotspot_surface(),
                minimum: 1,
                maximum: 1,
            },
            crate::StudentResponse::Hotspot {
                points: vec![HotspotPoint { x: 20, y: 30 }],
            },
        ),
    ];

    for (name, schema, response) in cases {
        let active = screen(schema);
        let request = RehearsalSubmissionRequestV1 {
            presentation_digest: active.presentation_digest.clone(),
            response,
        };
        assert!(
            ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(request, &active)
                .is_ok(),
            "{name} must be admitted at the rendered boundary"
        );
    }

    let active = screen(RehearsalResponseSchemaV1::FillIn { max_characters: 4 });
    for response in [
        crate::StudentResponse::FileUpload {
            object_key: "student-records/not-a-rehearsal-response".into(),
        },
        crate::StudentResponse::ExternalTool {},
    ] {
        assert!(matches!(
            ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(
                RehearsalSubmissionRequestV1 {
                    presentation_digest: active.presentation_digest.clone(),
                    response,
                },
                &active,
            ),
            Err(RehearsalWireValidationError::UnsupportedResponseFamily)
        ));
    }
}

#[test]
fn rendered_submission_boundary_rejects_matching_identifier_role_swaps() {
    let active = screen(RehearsalResponseSchemaV1::Matching {
        prompts: vec![choice("0001"), choice("0002")],
        choices: vec![choice("0003"), choice("0004")],
        reuse_choices: false,
    });
    let request = RehearsalSubmissionRequestV1 {
        presentation_digest: active.presentation_digest.clone(),
        response: crate::StudentResponse::Matching {
            matches: vec![
                MatchPair {
                    prompt: ChoiceId::new("0003"),
                    choice: ChoiceId::new("0001"),
                },
                MatchPair {
                    prompt: ChoiceId::new("0004"),
                    choice: ChoiceId::new("0002"),
                },
            ],
        },
    };
    assert!(matches!(
        ValidatedRehearsalRenderedSubmissionV1::try_from_active_screen(request, &active),
        Err(RehearsalWireValidationError::ResponseDoesNotMatchScreen)
    ));
}

#[test]
fn empty_mutation_requires_an_exact_empty_object() {
    assert!(
        serde_json::from_value::<RehearsalEmptyMutationRequestV1>(serde_json::json!({})).is_ok()
    );
    assert!(
        serde_json::from_value::<RehearsalEmptyMutationRequestV1>(
            serde_json::json!({"revision": 3})
        )
        .is_err()
    );
}

#[test]
fn public_presentation_contains_no_uuid_or_private_answer_fields() {
    let encoded = serde_json::to_string(&screen(RehearsalResponseSchemaV1::FillIn {
        max_characters: 3,
    }))
    .expect("JSON")
    .to_ascii_lowercase();
    for forbidden in [
        "uuid",
        "answer",
        "learner",
        "attempt",
        "sourceartifact",
        "generatorseed",
    ] {
        assert!(!encoded.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn safe_progress_is_bounded_and_carries_no_attempt_identity() {
    assert_eq!(
        RehearsalProgressV1::new(3, 4),
        Ok(RehearsalProgressV1 {
            current: 3,
            total: 4
        })
    );
    assert_eq!(
        RehearsalProgressV1::new(5, 4),
        Err(RehearsalWireValidationError::InvalidProgress)
    );
    let encoded = serde_json::to_string(&RehearsalProgressV1::new(3, 4).expect("progress"))
        .expect("JSON")
        .to_ascii_lowercase();
    assert!(!encoded.contains("attempt"));
    assert!(!encoded.contains("uuid"));
}

#[test]
fn result_rejects_nonfinite_scores_and_unknown_feedback_fields() {
    let feedback = RehearsalDisclosedFeedbackV1 {
        correctness: None,
        points_earned: Some(f64::NAN),
        points_possible: Some(1.0),
        hint: None,
        correct_response: None,
        rationale: None,
    };
    let result = RehearsalSubmissionResultV1 {
        feedback,
        evidence: RehearsalEvidenceSummaryV1 {
            status: RehearsalEvidenceStatusV1::Recorded,
            recorded_at: crate::ActivityTimestamp::from_unix_millis(1),
        },
    };
    assert_eq!(
        result.validate(),
        Err(RehearsalWireValidationError::NonFiniteFeedback)
    );
    let invalid = serde_json::json!({"correctness":true,"unexpected":true});
    assert!(serde_json::from_value::<RehearsalDisclosedFeedbackV1>(invalid).is_err());
}
