use domain::validation::{
    StudentResponseFormatIssue, validate_presentation_response_format, validate_response_format,
};
use question_model::presentation::{
    PresentationResponseItemId, PresentedMatchingChoice, PresentedMatchingPrompt,
    QuestionPresentationResponseFormat,
};
use question_model::response::{
    MatchingChoice, MatchingPrompt, QuestionResponseFormat, ResponseItemId, StudentMatch,
    StudentResponse,
};

fn item_id(value: &str) -> ResponseItemId {
    ResponseItemId::new(value)
}

fn response_format() -> QuestionResponseFormat {
    QuestionResponseFormat::Matching {
        prompts: vec![
            MatchingPrompt {
                id: item_id("p1"),
                body: Vec::new(),
            },
            MatchingPrompt {
                id: item_id("p2"),
                body: Vec::new(),
            },
        ],
        choices: vec![
            MatchingChoice {
                id: item_id("c1"),
                body: Vec::new(),
            },
            MatchingChoice {
                id: item_id("c2"),
                body: Vec::new(),
            },
        ],
    }
}

fn presented_format() -> QuestionPresentationResponseFormat {
    let id = |value| PresentationResponseItemId::parse(value).expect("valid presentation ID");
    QuestionPresentationResponseFormat::Matching {
        prompts: vec![
            PresentedMatchingPrompt {
                id: id("a101"),
                body: Vec::new(),
            },
            PresentedMatchingPrompt {
                id: id("a102"),
                body: Vec::new(),
            },
        ],
        choices: vec![
            PresentedMatchingChoice {
                id: id("b101"),
                body: Vec::new(),
            },
            PresentedMatchingChoice {
                id: id("b102"),
                body: Vec::new(),
            },
        ],
        reuse_choices: false,
    }
}

#[test]
fn native_matching_allows_omissions_and_rejects_duplicate_or_unknown_ids() {
    let format = response_format();
    let partial = StudentResponse::Matching {
        matches: vec![StudentMatch {
            prompt: item_id("p1"),
            choice: item_id("c1"),
        }],
    };
    assert!(validate_response_format(&format, &partial).is_valid());
    assert!(
        validate_response_format(&format, &StudentResponse::Matching { matches: vec![] })
            .is_valid()
    );

    for invalid in [
        StudentResponse::Matching {
            matches: vec![
                StudentMatch {
                    prompt: item_id("p1"),
                    choice: item_id("c1"),
                },
                StudentMatch {
                    prompt: item_id("p1"),
                    choice: item_id("c2"),
                },
            ],
        },
        StudentResponse::Matching {
            matches: vec![StudentMatch {
                prompt: item_id("p3"),
                choice: item_id("c1"),
            }],
        },
    ] {
        assert!(
            validate_response_format(&format, &invalid)
                .issues
                .contains(&StudentResponseFormatIssue::MatchingPromptsMismatch)
        );
    }
    let unknown_choice = StudentResponse::Matching {
        matches: vec![StudentMatch {
            prompt: item_id("p1"),
            choice: item_id("c3"),
        }],
    };
    assert!(
        validate_response_format(&format, &unknown_choice)
            .issues
            .contains(&StudentResponseFormatIssue::UnknownMatchChoice {
                choice: item_id("c3")
            })
    );
}

#[test]
fn issued_matching_allows_omissions_and_retains_no_reuse_and_unknown_checks() {
    let partial = StudentResponse::Matching {
        matches: vec![StudentMatch {
            prompt: item_id("a101"),
            choice: item_id("b101"),
        }],
    };
    assert!(validate_presentation_response_format(&presented_format(), &partial).is_valid());
    assert!(
        validate_presentation_response_format(
            &presented_format(),
            &StudentResponse::Matching { matches: vec![] }
        )
        .is_valid()
    );

    let reused_choice = StudentResponse::Matching {
        matches: vec![
            StudentMatch {
                prompt: item_id("a101"),
                choice: item_id("b101"),
            },
            StudentMatch {
                prompt: item_id("a102"),
                choice: item_id("b101"),
            },
        ],
    };
    assert!(
        validate_presentation_response_format(&presented_format(), &reused_choice)
            .issues
            .contains(&StudentResponseFormatIssue::DuplicateMatchChoice {
                choice: item_id("b101")
            })
    );
    let unknown_prompt = StudentResponse::Matching {
        matches: vec![StudentMatch {
            prompt: item_id("a103"),
            choice: item_id("b101"),
        }],
    };
    assert!(
        validate_presentation_response_format(&presented_format(), &unknown_prompt)
            .issues
            .contains(&StudentResponseFormatIssue::MatchingPromptsMismatch)
    );
}
