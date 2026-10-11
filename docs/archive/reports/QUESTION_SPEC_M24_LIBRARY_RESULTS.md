# M24 Library Object results

## Result

The shared Library search now uses the Library Object request names and exposes common Question
fields at the browser row boundary while keeping the Question and Pool wire variants intact. Search
predicates and their Postgres binding use the same cross-kind semantics. The tentative initial
choice remains `kind=both` and `questions=inNoPool`; M24 does not settle that product default.

M06's `output_question_spec/M06_TYPE_SQL_READY` handoff appeared before the SQL edits. Its Type,
Bloom, and Pool projections remain intact. M11 final acceptance is still pending. The connected
PostgreSQL search matrix is expanded in source but was not run.

## Contract implemented

- The endpoint is `GET /api/library-objects/search`. Its `kind` filter is unchanged; its Question
  filter is `questions=all|inNoPool`, represented by `PublishedQuestionFilter`.
- `inNoPool` excludes a Question when any Pool contains it, including source-linked Pool forks.
  Pool rows are unaffected by the Question filter and remain in mixed results.
- Pools use their own title, description, owner, Type, Backend, Tags, license, classification, and
  Bloom fields. A positive Author or authored-by-current-account filter excludes Pools.
- Type text (`type:`) and structured Type filters apply to Questions and Pools. Backend capability
  filtering and capability facet counts use each row's common Backend.
- Category counts use domain-filtered candidates before kind, `questions`, and cursor/page
  selection. Other facets use kind- and `questions`-filtered results before paging and retain
  deterministic global ordering.
- Question and Pool stay distinct wire variants. Common Question fields reach the browser as
  `title`, `description`, `ownerAccountId`, `questionType`, `backend`, `tags`, and `license`; the
  Question-only exact Revision, authorship, format, and evidence fields remain available.
- The Assessment content picker reuses mixed search. Pool contents selection fixes the same search
  to `kind=questions` and `questions=all`.

## Source changes

- Rust public request/filter/page/sort names and the strict server query use `LibraryObjectSearch`
  names and `PublishedQuestionFilter`; the internal Library Store receives `questions`.
- The browser builds the shared Library Object search request in
  [`question_library_repository.ts`](../../../src/api/question_library_repository.ts#L212). The client,
  application query key, URL handoff, decoder, and repository use the Library Object search names
  and `/api/library-objects/search`. URL recovery drops the old `membership` option.
- The repository maps common fields for both result variants. The strict mixed-row decoder requires
  those common fields for Question rows while the Question-only picker decoder remains unchanged.
- Filter controls preserve Backend capability in Pools-only search and clear the Author predicate
  when Pools alone are selected. Type remains available for either kind.
- SQL changes in
  [`question_library_operations.sql`](../../../schemas/base_schema/50_functions/question_library_operations.sql)
  rename the filter argument to `p_questions`, keep Pool rows outside that Question-only
  restriction, match Type text on both row kinds, and remove the old capability flag that excluded
  Pools. The data-access bind and grants signature now match the reduced predicate list. Capability
  facets are derived from the filtered common Backend counts.
- The existing connected `question_library_search_filters_and_pages_in_postgresql` test now runs
  the mixed-search matrix against the same fixtures. It covers a source-linked Pool fork,
  `all`/`inNoPool`, Pool-row retention, category counts, Type text and structured Type across both
  kinds, positive and negative Author text terms, common Backend matching, and globally sorted Pool
  IDs.
- Current API contract docs, the active search specification, TODO naming item, changelog, and this
  report use the new request and route names. Historical reports retain the names they recorded at
  the time.

## Independent reviews

- Fresh SPEC review passed with no contract gaps.
- Fresh QUALITY review found that a negative `-author:` term also removed Pool rows before text
  matching. The SQL now excludes Pools only when an Author text term is positive; the connected
  matrix covers both signs for all three Pool rows, including the source-linked fork. The reviewer
  rechecked the correction and passed the owned source surface with no remaining findings.
- A fresh QUALITY recheck confirmed the mixed matrix is part of the existing ignored connected
  `question_library_search_filters_and_pages_in_postgresql` selector, runs before facet-boundary
  fixture edits, and has no duplicate standalone selector.

## Focused verification

| Check | Result |
| --- | --- |
| `source source_me.sh && cargo tsgen` | Passed; wrote 409 generated API types. |
| `source source_me.sh && node --import tsx --test tests/test_library_classification_search.mjs tests/test_question_pool_source_binding.mjs tests/test_question_picker.mjs tests/test_assessment_content_picker.mjs` | Passed, 30/30. |
| `source source_me.sh && npx tsc --noEmit` | Passed. |
| `source source_me.sh && cargo test -p question_model question_search --lib` | Passed, 6/6. |
| `rustfmt --edition 2024 --check` on changed Rust paths; `git diff --check` | Passed. The changed connected matrix source was rechecked after the Author-term addition. |
| Schema-table regeneration and schema-style check | The table document regenerated. Style check reported 8 findings in `schemas/base_schema/20_tables/question_authoring.sql` on `draft_question_authorship` and `draft_question_metadata`; these files are outside M24's search predicate changes and were left untouched. |
| `source source_me.sh && cargo test -p learning-data-access --test blueprint_course_postgres question_library_search_filters_and_pages_in_postgresql --no-run` | Blocked before the connected selector compiled by `E0599` at `crates/question_model/src/blueprint_course/fork_comparison.rs:196`: concurrent Pool changes removed `BlueprintQuestionPoolContent::question_pool_edit_number`. No database was started. |
| `source source_me.sh && cargo test -p server_core question_library --lib` | Blocked during dependency compilation by `E0609` at `crates/learning-data-access/src/postgres/assessment_delivery_start.rs:341`: that file reads `CurrentPoolEntry.random_selected_order`, a field absent from the current type. This is outside M24's search paths and remains with the concurrent Pool selection work. Server tests did not execute. |
| Connected `question_library_search_filters_and_pages_in_postgresql` selector and full database build | Not run. The existing selector now invokes the mixed matrix and is ready for the coordinated baseline. |

## Limits and handoff

No fresh database, connected PostgreSQL run, live browser capture, or deployment acceptance is claimed.
The connected selector needs the concurrent `BlueprintQuestionPoolContent` model mismatch resolved
before it can compile; the server parser and route unit tests also remain behind the
`CurrentPoolEntry.random_selected_order` mismatch. M11 final acceptance remains a separate plan
dependency. The initial filter candidate is still tentative under Human Guidance.

## Early checkpoint

The initial checkpoint recorded the then-current old route/`membership` request, browser loss of
Question common fields, and a hold on SQL edits while the M06 Type projection handoff was pending.
Those were implementation targets, not settled public names. The marker later appeared with M06's
revision-metadata projection handoff, after which M24 completed the search SQL changes described
above. The focused Graphify pass identified the search model, server mixed route, browser query
state, and picker seams; source and tests remain the behavior authority.

No human decision was required to proceed. The initial candidate remains explicitly tentative.

## Latest evidence (2026-10-07)

The canonical database baseline invoked the exact ignored
`question_library_search_filters_and_pages_in_postgresql` selector, labeled "Question Library
bounded PostgreSQL acceptance," in `local_stack_control/database_baseline_owner.py`. That selector
calls `assert_mixed_search_matrix` before its facet-boundary setup. The selector passed as part of
the canonical database baseline; the full `all_test` retry also exited 0. See the current status in
the [implementation ledger](question_spec_implementation_ledger.md) and
`output_question_spec/all_test_clean_retry_20261007.log`. This records the actual full filter/page
and mixed-matrix runtime evidence; earlier source-only/blocked checkpoints above describe their
state at that time.

The focused Library browser proof also passed, including Student Scores and Library search
discard/reopen. That proof does not establish Instructor Gradebook behavior. Broader browser
acceptance and final integration remain pending.
