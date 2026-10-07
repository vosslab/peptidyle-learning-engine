# M19: Grade FIB and MULTI-FIB

## Result

Source-level FIB and MULTI-FIB support the four declared text-match modes: `exact`,
`caseInsensitive`, `normalized`, and `regex`. Regex uses the existing `regex-lite` matcher with
ordinary `is_match` behavior; authors choose anchors and case handling. Empty and whitespace-only
Student responses earn zero.

MULTI-FIB grades each authored blank independently and divides correct blanks by all authored
blanks. Wrong, blank, and omitted answers earn zero. Response validation accepts empty and known
subsets while rejecting duplicate or unknown blank IDs. Source compilation rejects invalid regex;
publication requires valid complete source. Parsing remains separate from compilation.

## Source inventory

- Model rule: [`answer.rs`](../../../crates/question_model/src/answer.rs) defines and serializes
  `TextResponseMatchRule`.
- Native adapter: [`source_document.rs`](../../../crates/adapters/ple/src/question_json/source_document.rs)
  parses source; [`source_compile.rs`](../../../crates/adapters/ple/src/question_json/source_compile.rs)
  maps it to response and grading models; [`tests.rs`](../../../crates/adapters/ple/src/question_json/tests.rs)
  covers regex compilation and invalid syntax.
- Grading and domain: [`ple_question_json.rs`](../../../crates/grading/src/ple_question_json.rs)
  evaluates answers; [`fill_in_tests.rs`](../../../crates/grading/src/ple_question_json/fill_in_tests.rs)
  covers regex and all-authored-blank scoring; [`validation.rs`](../../../crates/domain/src/validation.rs)
  validates partial and empty MULTI-FIB responses.
- Browser and API: [`question_json_source.ts`](../../../src/features/ple_question_json_authoring/question_json_source.ts),
  [`question_json_codec.ts`](../../../src/features/ple_question_json_authoring/question_json_codec.ts),
  [`question_json_match_modes.ts`](../../../src/features/ple_question_json_authoring/question_json_match_modes.ts),
  [`question_json_fill_in_editor.tsx`](../../../src/features/ple_question_json_authoring/question_json_fill_in_editor.tsx),
  and [`question_json_multi_fill_in_editor.tsx`](../../../src/features/ple_question_json_authoring/question_json_multi_fill_in_editor.tsx)
  expose and preserve regex. The generated
  The local generated output `generated/api/TextResponseMatchRule.ts` includes `regex`; the authored
  enum declaration is the [TextResponseMatchRule enum](../../../crates/question_model/src/answer.rs#L54).

## Verification

- `source ./source_me.sh && cargo test -p grading --lib` passed all 19 tests. Relevant cases include
  `fib_regex_uses_authored_anchors_and_case_and_rejects_blank_input`,
  `fib_invalid_regex_is_a_validation_error`,
  `multi_fib_scores_correct_blanks_over_all_authored_blanks`, and
  `multi_fib_invalid_blank_regex_is_a_validation_error`.
- `source ./source_me.sh && cargo test -p domain multi_blank_responses_allow_subsets_and_reject_duplicate_or_unknown_slots --lib`
  passed 1 test for empty/partial durable and issued responses, duplicate slots, and unknown slots.
- `source ./source_me.sh && cargo test -p adapter_ple fib_regex_mode_compiles_patterns_and_reports_invalid_syntax --lib`
  passed 1 test for source conversion, serialized `regex`, correct grading, and invalid-pattern
  rejection during compilation.
- `source ./source_me.sh && node --import tsx --test tests/test_ple_question_json_text_numeric_authoring.mjs tests/test_question_response_format.mjs tests/test_ple_question_json_multi_fill_ordering_authoring.mjs`
  passed all 19 tests. This includes source round trips, FIB/MULTI-FIB selectors and response-format
  decoding, plus MULTI-FIB pattern editing above the Student response limit. An earlier 10-test
  result covered the initial subset and is superseded by this 19-test run.
- `source ./source_me.sh && cargo test -p question_model text_match_modes_use_camel_case_names --lib`
  passed the focused serialization test. A prior full model run executed 165 tests: 164 passed,
  including six search tests, and one failed on the M09 issued-Question identity fixture. That fixture
  is now corrected and its focused test
  `student_work::identifiers::tests::issued_question_identity_is_stable_and_distinguishes_frozen_content`
  passes; the full model suite has not been rerun after the correction. See the
  [M25 search report](QUESTION_SPEC_M25_SEARCH_STORAGE.md) for the earlier package result and the
  [M09 report](QUESTION_SPEC_M09_POOL_STORAGE.md) for the corrected identity test. The earlier
  compile blocker was in [M27 Blueprint Theme fixtures](QUESTION_SPEC_M27_THEME.md), not M05.
- The coordinator regenerated 415 declarations into 416 files. The generated text-match rule now
  includes `regex`. The broader TypeScript batch still reports 43 source and 46 lint errors in
  concurrent Pool, Bloom, and Theme code; no full M19 TypeScript type acceptance is recorded.

## Current boundaries

The browser authoring preview is local and answer-free. It can validate response-format shape, but
it does not invoke Native JSON source compilation or send Student answers for backend grading.
[M16](../active/question_spec_implementation_plan.md#m16-complete-draft-preview-and-testing) will
connect preview and testing to shared validators. [M15](../active/question_spec_implementation_plan.md#m15-autosave-unfinished-drafts)'s
raw broken-Draft save remains pending; this report does not claim completed backend preview,
testing, or raw-save behavior.

[M29](../active/question_spec_implementation_plan.md#m29-verify-integrated-completion) connected
response persistence, Student grading presentation, and Assessment point calculation remain
pending. No database, container, connected-storage, or live acceptance checks were run for this
milestone.
