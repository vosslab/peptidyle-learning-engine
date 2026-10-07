# M10 Direct Assessment Pool References

## Decision applied

**M10-D1:** `assessment_entry_pool.question_pool_id` remains a direct foreign key to an existing
ordinary Pool, and `assessment_entry_pool.selection_count` remains a positive Assessment-local
request. The Assessment input remains Pool ID plus requested count and ordinary entry policy. One
Pool can serve many Assessments. Saving an Assessment does not change Pool ownership or membership;
ordinary Pool edits affect later selections, while Attempts retain their selected exact Question
Revision Tuples.

The separate `assessment_question_pool_fork` association and its exclusivity, guard, grants, and
policies are removed. Fork-only Assessment routes, DTOs, stores, client code, and editor flows are
removed. The editor uses `QuestionPoolView` from the ordinary `GET /api/question-pools/{id}` route.
Pool Edit Number remains a live Pool concurrency/display value and Attempt selection evidence; the
Assessment does not persist it. The ignored `questionPoolEditNumber` member was removed from the
Assessment save JSON serializer.

## Implementation evidence

- **Storage and save:** `schemas/base_schema/20_tables/assessment.sql` keeps the direct Pool FK and
  `selection_count > 0`; `schemas/base_schema/30_constraints.sql` drops the association FK;
  `schemas/base_schema/50_functions/assessments.sql` validates the ordinary Pool and updates only
  Assessment entry policy. The association function/table files, trigger, grants, policy references,
  and install includes were removed. Course adoption and Blueprint update SQL no longer insert the
  removed association; their Pool copy behavior remains for M11.
- **Rust and API:** removed the Assessment Pool fork and count-only selection route/store/model
  modules. The ordinary Pool detail route and `QuestionPoolView` remain. The Assessment save
  serializer emits `questionPoolId` and `selectionCount` without the ignored Pool Edit Number.
- **Editor:** `assessment_workspace_questions_page.tsx` loads current Pool detail through the
  ordinary client method, then adds a reference with a local count. The entry editor and view use
  `QuestionPoolView`; saving changes Assessment entries without creating or changing a Pool.
- **Focused acceptance fixtures:** `tests/e2e/assessment_saved_response/03_course_pool_forks.sql`
  saves two Assessments against one Pool at counts 1 and 2, reads each through
  `ple_api.load_assessment_workspace_rows`, and asserts the Pool owner is unchanged.
  `07_assessment_fairness.sql` supplies the current Pool ID, Edit Number, and exact selected tuple
  for each Pool selection. It checks that earlier Attempts retain the original tuple after a Pool
  edit and that a later Attempt started before Entry retirement records the new Edit Number, exact
  replacement tuple, and issued Question. The mixed Library fixture uses ordinary Pools rather
  than a synthetic Assessment-owned association.
- **Contract docs:** `docs/CONTRACTS.md` now states the direct reference/count contract and links
  this report. `docs/SCHEMA_TABLES.md` was regenerated from the current schema sources.

## Checks

- `source ./source_me.sh && devel/generate_schema_tables_doc.py && source ./source_me.sh && python3 schema_style/check_schema_style.py` - passed; schema style reports `clean`.
- `node --import tsx --test tests/test_question_pool_metadata.mjs tests/test_nested_identity_contracts.mjs` - passed, 15 supporting Pool-metadata and nested-identity contract tests. The direct two-Assessment save/read behavior is covered by the SQL oracle listed above; that oracle was not executed.
- `npx prettier --check src/pages/assessment_workspace/assessment_pool_entry_editor.tsx src/pages/assessment_workspace/assessment_workspace_questions_page.tsx src/pages/assessment_workspace/assessment_workspace_questions_view.tsx src/pages/assessment_workspace/assessment_blueprint_update_review.tsx` - passed.
- `source ./source_me.sh && cargo check -p question_model` - passed.
- `source ./source_me.sh && cargo check -p question_model -p learning-data-access -p server_core` - blocked by a concurrent M09 source error: `crates/learning-data-access/src/postgres/assessment_delivery_start.rs:341` reads the removed `CurrentPoolEntry.random_selected_order` field. `cargo check -p question_model` passes; the M10 Pool-reference DTO itself remains unchanged.
- `npx tsc --noEmit -p tsconfig.json` - passed after concurrent TypeScript fixes landed; no M10 assessment-workspace diagnostics remain.
- `source ./source_me.sh && cargo fmt --all -- --check` - reports many unrelated formatting diffs in the heavily modified worktree; no workspace-wide formatting pass was applied.
- A source search for `assessment_question_pool_fork`, fork-only API names, and selection-count route names found no remaining matches under `schemas/`, `crates/`, `src/`, or `tests/`.
- Root ran `source ./source_me.sh && cargo tsgen` successfully and wrote 409 generated types. Generated TypeScript remains root-owned.
- Fresh specification review found no blocking mismatch. It identified two stale SQL comments,
  which were corrected to describe direct references and explicit ordinary Pool forks.
- Fresh quality review found that the original fairness fixture lacked a post-edit Pool selection.
  The fixture now checks an earlier Attempt at Pool Edit Number 1 and a later pre-retirement Attempt
  using the updated Edit Number and exact replacement tuple. A fresh quality re-review found no
  blocking issue.

## Acceptance limits and remaining work

The SQL acceptance fixtures were not executed against PostgreSQL; full database containers were out
of scope. Their save/read and Attempt assertions, including the before-and-after Pool edit
selection case, are source evidence, not runtime acceptance. The root database/runtime gate remains
required. Browser behavior and a live Assessment save/read journey were not run.

M11 owns preservation of Pool copy behavior and its provenance through Course operations. Existing
Blueprint-specific source-copy/materialization paths remain in place for that milestone; they were
not converted into ordinary direct references here. Ordinary Pool Watch producers remain intact.
