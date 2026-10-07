# M25 search storage verification

## Scope

This report records source verification for M25 in the approved
[Question-spec implementation plan](../active/question_spec_implementation_plan.md#m25-remove-persistent-search-storage).
The search specification says prompts and results are temporary page state and are never permanently
stored ([Question Library search specification](../../QUESTION_SPECS/QUESTION_LIBRARY_SEARCH_SPEC.md#navigation-and-search-lifetime)).

The schema and `question_search.rs` edits were already present in the shared working tree. This slice
verifies those edits and their focused gate; it makes no additional product or schema design change.
The M09 model/domain removed-position fixtures and M27 Blueprint Theme model fixtures were handed off
coherent before the package-wide model run.

## Source evidence

- The current diff removes `ple_private.saved_question_search` and its table comment from
  [question_authoring.sql](../../../schemas/base_schema/20_tables/question_authoring.sql).
- The current diff removes its owner index from
  [40_indexes.sql](../../../schemas/base_schema/40_indexes.sql), plus its owner policy and two RLS
  statements from
  [question_authoring_state.sql](../../../schemas/base_schema/60_policies/question_authoring_state.sql).
- A targeted search of `schemas/base_schema`, `src`, `crates`, and `tests` finds no remaining
  `saved_question_search`, saved-search, or `Used in my Courses` source references. No Rust, UI, or
  SQL-function caller remains.
- The generated [schema table catalog](../../SCHEMA_TABLES.md) has 137 table headings and no entry
  for the removed table.
- In [question_search.rs](../../../crates/question_model/src/question_search.rs), two
  `QuestionSearchFilter` comments and two test labels now describe ordinary filter conversion. The
  existing assertions remain: filter normalization and JSON round-trip, classification identity,
  and resetting cursor/page size for a fresh first-page query.

## Verification

- `source ./source_me.sh && python3 schema_style/check_schema_style.py` - passed (`clean`).
- `source ./source_me.sh && cargo check -p question_model` - passed.
- `source ./source_me.sh && rustfmt --check --edition 2024 crates/question_model/src/question_search.rs`
  - passed.
- `source source_me.sh && cargo test -p question_model --lib` - compiled and ran 165 tests: 164
  passed and 1 failed. Six search tests passed (the normalization test and five
  `question_search::tests`), as did
  `answer::tests::text_match_modes_use_camel_case_names`. The package gate remains red on
  `student_work::identifiers::tests::issued_question_identity_is_stable_and_distinguishes_frozen_content`;
  its supposed-valid Question ID is rejected because its random characters are not exact uppercase
  Crockford Base32. The earlier M27 Theme fixture compile blocker is cleared.
- `source ./source_me.sh && cargo fmt --check -p question_model` - reports formatting in
  [canonical_exchange.rs](../../../crates/question_model/src/blueprint_course/canonical_exchange.rs)
  and [question_metadata.rs](../../../crates/question_model/src/question_metadata.rs); neither is
  part of this slice.
- `source ./source_me.sh && python3 tests/test_markdown_links.py` - passed.
- Scoped `git diff --check` passes, and the report trailing-whitespace scan is clean.

No database build or connected PostgreSQL run was performed.

## Limits and review

The source removal and M25 search-model tests are verified. The package-wide model gate remains
pending because of the issued-Question identity test failure above. Fresh-database filtering and
pagination, M29 runtime pagination acceptance, and integrated acceptance remain open. M24 dependency
and final live acceptance are also pending.

The M25 changelog entry records the passing search-model tests and the package-wide failure.

The fresh SPEC reviewer is invited to challenge the source trace and the retained normalization,
round-trip, and first-page assertions. The separate QUALITY reviewer is invited to challenge the
schema-object removal evidence and the limits of the current verification. Neither independent
review has run in this slice.

## Latest evidence (2026-10-07)

M25's plan check is fresh-database filtering and pagination with temporary search state. The
canonical database baseline's exact bounded Question Library selector ran and passed, including
the full filter/page selector and mixed-search matrix; the canonical `all_test` retry then exited 0.
The focused Library browser proof passed search discard/reopen. See the current status and log
references in the [implementation ledger](question_spec_implementation_ledger.md). This updates the
earlier package-test and connected-runtime checkpoint above; those results remain historical for
their original runs. Source-removal review remains the authority for removal details.

The focused browser proof is not final integrated/browser acceptance. The broader browser suite
and final integration remain pending.
