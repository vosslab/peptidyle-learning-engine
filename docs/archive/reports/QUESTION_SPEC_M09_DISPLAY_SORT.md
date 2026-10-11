# M09 Pool display sorting

## Result

The ordinary Question Pool detail and its Owner/Sysadmin Add/Remove/Save editor
use the shared RecordSortControl and RecordTable. Both support local sorting by
Question Title, Published Question ID, Revision, and Question License. Text sorts
use natural, case-insensitive English comparison; Revision sorts numerically;
equal values retain their input order.

The read-only detail table sorts a copy of loaded rows. The editor sorts a copy of
draft members while Add, Remove, dirty-state comparison, and Save continue to use
the unchanged draft. Sorting leaves the exact Revision tuple set, random
selection, and Pool Edit Number unchanged. This is display state only. The UI
adds no Pool order field, default policy, or required choice. M09 owns the
ordinary Pool editor and display sorting; M10/M11 retain direct Pool reference
and forced-copy work.

The coordinator-approved ROOT_POOL_SORT_DECISION.md records the cross-lane
boundary. QUESTION_SPEC_M09_ORDER_CORRECTION.md tracks removal of the unsupported
persisted Assessment Pool selection-order policy and the focused evidence.

## Source and focused evidence

- Product authority: docs/HUMAN_GUIDANCE.md, Question Pool specifications,
  lines 1292-1303 (HG1295-96); docs/QUESTION_SPECS/QUESTION_POOL_SPEC.md,
  Membership.
- The read-only table is in src/pages/question_pool_detail.tsx; the editor is
  src/components/question_pool_members_editor.tsx and uses that page's
  Owner/Sysadmin gate.
- src/components/record_list/question_pool_member_sort.ts makes stable display
  copies. src/components/question_pool_members_model.ts compares membership as
  a set, and display sorting does not mutate the draft.
- Focused Node checks passed 40/40 across Pool creation, member model/client,
  sorting, metadata decoding, nested identity, and release validation. The
  selector model passed 3 tests for input-permutation independence and
  entropy-driven sampling. question_model passed 165/165 tests.
- Both local Playwright checks passed. The editable check removes a member,
  changes only visible row order, verifies the same exact draft tuple set and
  Edit Number 7, then confirms Save carries that set with Edit Number 7. The
  stubbed receipt advances the acknowledged number to 8.
- `source ./source_me.sh && cargo check -p learning-data-access --features postgres --lib`
  passes. `source ./source_me.sh && cargo check -p server_core --lib` passes with
  three warnings: unused `now`, dead `is_ple_question_json_request`, and dead
  `instructor`. Inline tests were split into `authoring_tests.rs` to keep
  `authoring.rs` below 1,000 lines (879 lines; test module 150 lines).
- `source ./source_me.sh && cargo test -p server_core --lib authoring::tests`
  passes 4/4. The broader `authoring` filter ran 17 tests: 16 passed and one
  unrelated publication hotspot test failed on stale fixture JSON `language`.
  This is not an M09 editor failure.
- `npx tsc --noEmit -p tsconfig.json` passes. Schema-style, Prettier, scoped
  rustfmt, and git diff --check passed.

## Limits

The browser checks use a controlled local harness and stubbed Save receipt. No
connected PostgreSQL, API, production-browser, or full database acceptance was
run. The SQL callable wrapper, ple_app grant, active Instructor-or-Sysadmin
check, and post-lock Pool-owner check exist in source; connected role and Save
behavior remain pending.
