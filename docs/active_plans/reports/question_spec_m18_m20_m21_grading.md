# M18, M20, and M21 grading integration

## Implementation

M18 now accepts Matching responses containing any subset of authored prompts while retaining
duplicate-prompt, unknown-ID, and choice-reuse validation. Native grading awards the number of
correct answered pairs divided by all authored prompts; omitted prompts therefore earn zero.
Saved-response history renders every authored prompt and labels omitted answers `Unanswered`.

M20 registers and calls the prepared Multiple Answer helper after the existing selection and key
checks. M21 maps the complete submitted stable-ID permutation to answer-key ranks and calls the
prepared ORDER helper. The PLE Question JSON evaluator now carries normalized credit through
`QuestionEvaluation`; MC and HOTSPOT remain binary, and NUM retains its existing tolerance check.

## Scope

- `crates/grading/src/ple_question_json.rs`
- `crates/grading/src/ple_question_json_validate.rs`
- `crates/grading/src/ple_question_json/multiple_answer.rs`
- `crates/grading/src/ple_question_json/ordering.rs`
- `crates/domain/src/validation.rs`
- `crates/domain/tests/matching_partial_responses.rs`
- `crates/server/src/assessment_delivery/history_response.rs`

## Verification

- `source ./source_me.sh && cargo test -p grading --lib` - 14 passed.
- `source ./source_me.sh && cargo test -p domain --lib validation::tests` - 11 passed, including the then-local MATCH partial-response unit cases.
- `source ./source_me.sh && cargo test -p domain --test matching_partial_responses` - 2 passed after moving those MATCH cases to an integration test so `validation.rs` stays below 1,000 lines.
- Scoped `rustfmt --edition 2024` and `git diff --check` - passed.
- `source ./source_me.sh && cargo test -p server_core --lib assessment_delivery::history_response::tests::matching_history_shows_authored_prompts_with_omitted_answers_labeled` - 1 passed after M05 Server/data-access integration compiled. No database or full-stack command was run.

## Acceptance boundary

The grading and domain tests demonstrate source-level scoring and validation. Stored Assessment
points, connected response persistence, and Student browser rendering remain pending the assigned
M29/runtime acceptance. M17 fresh-runtime acceptance and independent specification/quality reviews
also remain pending.
