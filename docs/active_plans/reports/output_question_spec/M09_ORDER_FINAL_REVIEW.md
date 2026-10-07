# M09 Order Final Review

Review date: 2026-10-06

## Outcome

**ACCEPT: M09 unordered-Pool order-correction source slice.** A fresh SPEC review and a separate
fresh QUALITY review passed after one bounded SQL correction. This accepts the source slice only; it
does not accept the complete M09 milestone or connected runtime behavior.

## Authority and call path

The approved
[question_spec_implementation_plan.md](../../active/question_spec_implementation_plan.md)
and [HUMAN_GUIDANCE.md](../../../HUMAN_GUIDANCE.md) define Pool membership as an unordered set of
exact Question Revision Tuples. Display sorting does not change membership, random selection, or
the Pool Edit Number. Assessment-authored order and Attempt-local positions remain separate. The
[M09_ORDER_BOUNDARY.md](M09_ORDER_BOUNDARY.md) records the correction ownership split; M12 behavior
remains separately owned.

The accepted Save path is Store -> `ple_api.save_question_pool_members(text, bigint, text[], integer[])`
-> protected `ple_data.save_question_pool_members`. The Store calls the wrapper at
`crates/learning-data-access/src/postgres/question_pool_library.rs:83-91`; the `SECURITY DEFINER`
wrapper delegates by qualified name at `schemas/base_schema/50_functions/question_pools.sql:620-629`.
Grants give `ple_api_owner` access to the data function and give `ple_app` access to the Save wrapper
without direct Save data-function EXECUTE or `ple_data` schema USAGE at
`schemas/base_schema/70_grants/question_pools.sql:34-58`. The registered oracle asserts the exact
app callable, wrapper owner/search path, and absence of direct app access at
`tests/e2e/assessment_saved_response/09_pool_save_permissions.sql:19-55`.

This path is settled in [ROOT_POOL_SAVE_CALLABLE_DECISION.md](ROOT_POOL_SAVE_CALLABLE_DECISION.md).
The callable-path blocker in the boundary and the old direct-data task wording in the
[QUESTION_SPEC_M09_ORDER_CORRECTION.md](../QUESTION_SPEC_M09_ORDER_CORRECTION.md)
are superseded for source acceptance. Connected execution remains pending; no historical report was
edited in this review. Generated API contracts were inspected read-only; their regeneration remains
root-owned.

## Review sequence

1. **Fresh SPEC: ACCEPT.** No blocking findings. The reviewer confirmed unordered exact tuples,
   display-only sorting, random selection semantics, preserved ordered Assessment positions, and the
   accepted callable path.
2. **Fresh QUALITY: P2 finding.** The protected Save function did not enforce the established
   1,024-member maximum, although Create and the HTTP/store layers did.
3. **Bounded correction.** A fresh worker added
   `OR cardinality(p_member_question_ids) > 1024` to the protected Save guard at
   `schemas/base_schema/50_functions/question_pools.sql:343-352`, matching Create at lines 215-225.
   The SQL file is 729 lines. The worker changed no other file and ran no tests or runtime checks.
4. **Fresh SPEC re-review: ACCEPT.** The correction matches the existing 1,024-member limit and
   current M09 authority.
5. **Separate fresh QUALITY re-review: PASS.** No remaining findings.

## Source evidence

- `question_pool_member` has a primary key on `(question_pool_id, published_question_id)` and an
  exact Revision foreign key, with no member-position column:
  `schemas/base_schema/20_tables/question_pool.sql:79-90`.
- Pool read models and generated API contracts expose exact member tuples without an order field.
  The TypeScript decoder accepts arbitrary member order and rejects duplicate Question IDs at
  `src/api/decoders/question_pool_detail.ts:71-100`.
- Save compares current and requested membership in both directions as tuple sets, preserving the
  Edit Number for a no-op at `schemas/base_schema/50_functions/question_pools.sql:372-392`.
- SQL ordering used for license-array alignment and consistent row-lock acquisition is internal;
  it does not persist or expose Pool member order at
  `schemas/base_schema/50_functions/question_pools.sql:59-65,245-253,415-429`.
- The selector canonicalizes candidate tuples internally, then samples without replacement from
  server-provided entropy at `crates/domain/src/question_pool_selection.rs:58-115`. Its focused
  tests cover input-permutation independence and entropy-driven variation at lines 159-212.
- Read-only Pool detail and editable Pool draft tables sort derived display copies. Dirty-state
  comparison uses tuple sets, and Save serializes `draft.members`, not sorted display rows:
  `src/pages/question_pool_detail.tsx:58-62`,
  `src/components/question_pool_members_model.ts:48-60,126-164`, and
  `src/components/question_pool_members_editor.tsx:59-62,169-188,278-296`.
- Assessment entries retain authored positions; `issued_position` and Attempt-local
  `selection_position` remain in Student Work. No Pool member order policy remains in production
  source or generated contracts. A scoped legacy-name scan used:

  ```sh
  rg -n \
    -e 'QuestionPoolSelectedQuestionOrder|selected_question_order|QuestionPoolOrder|RandomOrder' \
    -e 'selectedQuestionOrder|questionPoolOrder|member_position|memberPosition' \
    schemas crates src generated content tests
  ```

  It found legacy names only in negative rejection or serialization-absence assertions at
  `tests/test_live_assignment_release_validation.mjs:234-238`,
  `tests/test_nested_identity_contracts.mjs:250-257`, and
  `crates/learning-data-access/src/postgres/assessment_workspace_save.rs:94`.
- The scoped source line-count check found no file at 1,000 lines; the longest M09 SQL source is
  729 lines. The adjacent `crates/server/src/authoring.rs` is 867 lines.

## Existing focused evidence

The producer's
[QUESTION_SPEC_M09_ORDER_CORRECTION.md](../QUESTION_SPEC_M09_ORDER_CORRECTION.md)
and `output_question_spec/m09_order_correction_cli.log` record seven focused Node suites
passing 40/40,
`question_model` passing 165 tests, selector tests passing 3/3, and both controlled local browser
sort checks passing. The editable browser harness preserves the draft tuple set, existing dirty
state, and Edit Number 7 through sorting; its stubbed Save preserves the tuple set and returns the
next acknowledged Edit Number. TypeScript, schema-style, formatting, and scoped Rust checks are also
recorded as passing. These checks were not rerun during this review.

The existing report also records a broader authoring filter with one unrelated stale publication
hotspot fixture failure and a server check with three warnings. Neither finding is in this M09
source slice; neither was rerun here.

## Remaining acceptance

Connected PostgreSQL Save, API behavior, and production-browser acceptance remain pending for the
root runtime pass. The registered permission oracle was not executed here. M05/M06 dependencies and
whole-M09 acceptance remain separate. No tests, containers, database commands, or browser runtime
were run by the review or correction workers.
