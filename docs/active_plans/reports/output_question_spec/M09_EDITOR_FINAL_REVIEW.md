# M09 Ordinary Pool Editor Final Review

## Acceptance

**Ordinary editor source acceptance: PASS.** A fresh specification reviewer and a
different fresh quality reviewer accepted the current ordinary Owner/Sysadmin Pool
Add/Remove/Save editor and display sorting on 2026-10-06. The SPEC reviewer returned
PASS after root decisions D1 and D2 closed the call-path and lock-order interpretations.
The QUALITY reviewer returned PASS with no editor findings.

This accepts the source editor slice. It does not accept connected PostgreSQL/API or
production-browser behavior, M05/M06 dependencies, or the complete M09 milestone.
Root retains those runtime and milestone gates.

## Scope and task boundary

The reviewed product scope is M09 from
[`question_spec_implementation_plan.md`](../../active/question_spec_implementation_plan.md#L181):
an ordinary Pool member editor for the Pool Owner or a Sysadmin, with Add/Remove/Save,
an unordered exact Revision Tuple set, no attestation or order input, and display-only
member sorting. M10/M11 retain direct Pool references and forced-copy removal. The
shared statistics panel's owner retains its mounting-only hunk. The editor lane owns
no SQL changes.

Root decision D1 accepts the current save call path:
`ple_app` calls SECURITY DEFINER `ple_api.save_question_pool_members(...)`, which
delegates to protected `ple_data.save_question_pool_members(...)`. `ple_app` has no
`ple_data` schema USAGE or direct data-function EXECUTE grant. The old direct-data
brief phrase and older unanswered-question text are stale implementation evidence,
not an open product question.

Root decision D2 treats SQL `ORDER BY ... FOR SHARE` at
`question_pools.sql:418-429` as deterministic lock acquisition for new metadata rows.
It does not determine stored/displayed Pool membership or sampler output. Preserve
this lock behavior; it does not create a Pool order policy.

## Requirement-to-source evidence

| M09 behavior | Current source evidence |
| --- | --- |
| Owner/Sysadmin editor visibility | [`question_pool_detail.tsx`](../../../../src/pages/question_pool_detail.tsx#L103) gates the editor; the server repeats the Owner/Sysadmin check in [`question_pool_library.rs`](../../../../crates/server/src/question_pool_library.rs#L172). |
| Complete exact tuple-set draft, Add/Remove/Save, failure retention, and reload protection | [`question_pool_members_editor.tsx`](../../../../src/components/question_pool_members_editor.tsx#L50) and [`question_pool_members_model.ts`](../../../../src/components/question_pool_members_model.ts#L48). |
| Add eligibility and complete closed request/receipt | The editor resolves current member tuples and Pool eligibility; [`question_pool_members.ts`](../../../../src/api/decoders/question_pool_members.ts#L18) enforces a bounded nonempty set with one Revision per Question and decodes the Edit Number receipt. |
| Type, Backend, classification, license, owner, and Edit Number safeguards | The server route and protected save function retain request, permission, tuple, classification/license, and CAS checks; the Postgres store calls the accepted `ple_api` wrapper in [`question_pool_library.rs`](../../../../crates/learning-data-access/src/postgres/question_pool_library.rs#L59). |
| Display sorting only | [`question_pool_member_sort.ts`](../../../../src/components/record_list/question_pool_member_sort.ts#L59) returns a sorted copy; the draft dirty check compares sets in [`question_pool_members_model.ts`](../../../../src/components/question_pool_members_model.ts#L126). The controlled browser harness checks sort stability, Save payload set, and Edit Number in [`question_pool_members_editor_sort_browser.mjs`](../../../../tests/playwright/question_pool_members_editor_sort_browser.mjs#L35). |
| No attestation or persisted order input | The browser save input contains Pool ID, expected Edit Number, and member tuples in [`question_pool_members.ts`](../../../../src/api/question_pool_members.ts#L7); the M09 source request and save path contain no certification field or order policy. |

The implementation also keeps canonical tuple ordering for input-permutation-independent
random sampling in [`question_pool_selection.rs`](../../../../crates/domain/src/question_pool_selection.rs#L58).

## Focused verification evidence

The implementation handoff records these results; the two fresh reviewers inspected
the current source and recorded evidence without rerunning broad suites:

- `node --import tsx --test tests/test_question_pool_members_client.mjs tests/test_question_pool_members_model.mjs tests/test_question_pool_member_sort.mjs`: 13/13 passed.
- `npx tsc --noEmit -p tsconfig.json`: passed.
- `source ./source_me.sh && cargo test -p question_model question_pool_library --lib`: 2 passed.
- `source ./source_me.sh && cargo check -p learning-data-access --features postgres --lib`: passed.
- `source ./source_me.sh && cargo test -p learning-data-access --features postgres member_replacement_requires_bounded_nonempty_one_revision_per_question --lib`: 1 passed.
- `node --import tsx tests/playwright/question_pool_members_editor_sort_browser.mjs`: passed in the controlled local harness with a stubbed Save receipt.
- `rustfmt --edition 2024 --config skip_children=true --check crates/question_model/src/question_pool_library.rs crates/learning-data-access/src/question_pool_library.rs crates/learning-data-access/src/postgres/question_pool_library.rs crates/server/src/question_pool_library.rs` and `git diff --check`: passed.
- All reviewed authored source and focused test files remain below 1,000 physical lines. The largest reviewed source files are `schemas/base_schema/50_functions/question_pools.sql` at 729 lines, `crates/learning-data-access/src/postgres/question_pool_library.rs` at 598 lines, and `crates/server/src/question_pool_library.rs` at 515 lines.

The handoff also records `source ./source_me.sh && cargo check -p server_core --lib`
passing with three warnings (`now`, `is_ple_question_json_request`, and `instructor`).
`source ./source_me.sh && cargo test -p server_core --lib authoring::tests` passed
4/4. A broader `authoring` filter ran 17 tests: 16 passed and one unrelated
publication HOTSPOT test failed on a stale fixture `language` field. These broader
authoring results are not M09 editor failures. No tsgen run was performed within this
editor lane; the handoff leaves final source generation to root.

## Remaining acceptance

Connected PostgreSQL owner/Sysadmin Save, connected API behavior, and production-stack
browser behavior remain pending root runtime acceptance. The local browser harness
does not replace those gates. M05/M06 dependency acceptance and complete M09 milestone
acceptance also remain pending.

The separate M09 order-correction report has historical unanswered-wrapper wording
under its separate owner. D1 and this boundary clarification govern the current call
path; this final review does not edit that lane's report.
