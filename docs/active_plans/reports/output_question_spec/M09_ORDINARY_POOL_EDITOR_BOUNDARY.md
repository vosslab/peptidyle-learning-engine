# M09 ordinary Pool Questions editor

## Result

M09 adds one ordinary Pool member editor to `src/pages/question_pool_detail.tsx`.
The Pool Owner and Sysadmin see Add, Remove, and Save controls. The browser keeps a
local complete unordered set of exact Published Question Revision tuples and sends
that set with the acknowledged Pool Edit Number to
`PUT /api/question-pools/{question_pool_id}/members`.

Retained members keep their exact Revision tuples. Added Questions are discovered
through the existing Question Picker, then resolved to current exact tuples after
checking the Pool's Discipline, Subject, Type, Backend, and license eligibility.
The client, local model, data-access validation, and existing SQL function enforce
one Revision per Question ID. The editable draft table uses the shared sort helper,
sort control, and record table on a display copy of draft members. The persisted
detail table remains a separate read-only view. Sorting does not change the exact
draft tuple set, dirty state, or acknowledged Edit Number.

An accepted save adopts the receipt's Pool Edit Number, clears the dirty state, and
refreshes Pool detail. Failed saves retain the complete local draft and display an
error. The editor prevents removal of the last member because saved Pools remain
nonempty. No Assessment ownership, automatic fork, member attestation, persisted
ordering, or ordering option was added.

Add is disabled while reload is pending, and `addSelection` also guards against
reloading. Remove remains available during reload except while saving or adding,
and when only the last member remains. If Remove changes the draft during reload,
the draft identity guard rejects the stale reload and preserves the exact tuple set
and Edit Number. UI evidence covers Remove-during-reload preservation;
synthetic pending-Add/reload order cases are model-only and are not reachable
through the current UI controls.

## Source boundaries

- `crates/question_model/src/question_pool_library.rs` defines the closed complete
  member request and Edit Number receipt.
- `crates/learning-data-access/src/question_pool_library.rs` owns the store method;
  `crates/learning-data-access/src/postgres/question_pool_library.rs` validates the
  bounded tuple set and makes a parameterized transaction call to
  `ple_api.save_question_pool_members(...)`.
- `crates/server/src/question_pool_library.rs` exposes the PUT route, conceals
  writes by non-owners, permits the Pool Owner or Sysadmin at the HTTP boundary,
  bounds the JSON body, and returns no-store success or generic error responses.
- `src/api/question_pool_members.ts`, its decoder, and HTTP client provide the
  strict browser command and receipt.
- `src/components/question_pool_members_model.ts` owns the exact local tuple-set
  draft and Save/CAS state. `question_pool_members_editor.tsx` resolves additions,
  renders Add/Remove/Save controls and the locally sorted draft display, and
  preserves the draft after failure.
- `src/pages/question_pool_detail.tsx` shows editing controls only to the Pool
  Owner or Sysadmin and leaves the existing M05 metadata editor, M22 evidence,
  shared sort helper, `RecordSortControl`, and `RecordTable` in place.
- `docs/API_CONTRACTS.md` lists the new route. This lane did not hand-edit generated contracts; root owns regeneration.

## SQL boundary and connected acceptance

### Exact SQL hunk ownership

This ordinary editor slice did not edit
`schemas/base_schema/50_functions/question_pools.sql` or its grants. It consumes
the current store call path through the `ple_api` wrapper to the protected
`ple_data.save_question_pool_members` function. The whole-workspace SQL diff
belongs to the separate approved M09 storage/order and M12 release-gating lanes;
preserve those existing hunks when reviewing or integrating this editor slice.

The HTTP source and current SQL function source include the Owner/Sysadmin
predicates. The callable boundary is the exact `ple_api` wrapper and focused
catalog/app-invocation assertion; see
[ROOT_POOL_SAVE_CALLABLE_DECISION.md](ROOT_POOL_SAVE_CALLABLE_DECISION.md).
Connected database and API/browser acceptance remain pending:

- `PostgresQuestionPoolLibraryStore::begin` in
  `crates/learning-data-access/src/postgres/question_pool_library.rs` switches to
  `ple_app` and calls `ple_api.save_question_pool_members(...)`.
- The wrapper is a `SECURITY DEFINER` function owned by `ple_api_owner` with
  `search_path = pg_catalog`; it delegates by qualified name to the protected data
  function. `ple_app` has no `ple_data` USAGE and no direct data-function EXECUTE grant.
- `schemas/base_schema/50_functions/question_pools.sql` defines the API wrapper
  and retains the protected data function's active-role check, Pool lock, and
  Instructor-owner or Sysadmin predicate. Its member, license, classification,
  and Edit Number checks remain in the existing function.

The existing `ple_data` function keeps its owner and all authorization and save
checks. The callable-boundary source now includes the `ple_api` wrapper, exact
`ple_app` wrapper grant, and catalog/app-call oracle assertion. Root owns the
disposable PostgreSQL run. Source generation completed, but no connected
database Save or Sysadmin Save is claimed until the disposable run passes. HTTP
authorization, decoder, local draft, and source gates do not replace database
acceptance.

## Focused evidence

- `node --import tsx --test tests/test_question_pool_members_client.mjs
  tests/test_question_pool_members_model.mjs
  tests/test_question_pool_member_sort.mjs` passed 13/13 tests.
- `npx tsc --noEmit -p tsconfig.json` passed.
- `source ./source_me.sh && cargo test -p question_model
  question_pool_library --lib` passed 2 tests.
- `source ./source_me.sh && cargo check -p learning-data-access --features
  postgres --lib` passed.
- `source ./source_me.sh && cargo test -p learning-data-access --features
  postgres member_replacement_requires_bounded_nonempty_one_revision_per_question
  --lib` passed 1 test.
- `node --import tsx
  tests/playwright/question_pool_members_editor_sort_browser.mjs` passed in
  the controlled local/stubbed harness.
- `rustfmt --edition 2024 --config skip_children=true --check
  crates/question_model/src/question_pool_library.rs
  crates/learning-data-access/src/question_pool_library.rs
  crates/learning-data-access/src/postgres/question_pool_library.rs
  crates/server/src/question_pool_library.rs` and `git diff --check` passed.
- The Prettier check on the edited TS, test, boundary, root, and changelog
  files passed.
- `source ./source_me.sh && cargo check -p server_core --lib` passes with three
  warnings: unused `now`, dead `is_ple_question_json_request`, and dead
  `instructor`. Inline authoring tests were split into
  `crates/server/src/authoring_tests.rs` to keep source files below 1,000 lines;
  `authoring.rs` is 879 lines and `authoring_tests.rs` is 150 lines.
- `source ./source_me.sh && cargo test -p server_core --lib authoring::tests`
  passes 4/4. The broader `authoring` filter compiled and ran 17 tests: 16
  passed and one unrelated `question_publication::tests::hotspot::native_hotspot_without_a_real_authoring_context_fails_before_any_publication`
  failed on stale fixture JSON `language`. This is not an M09 editor failure.
  Connected
  PostgreSQL/API/production-browser acceptance and M05/M06 remain pending; the
  local browser harness does not establish connected acceptance. No tsgen run
  was performed for this editor task.

Review handoff: the latest fresh SPEC pass is `m09_spec_sql_hunk_recheck`
(2026-10-06). It confirmed the ordinary editor contract, this exact SQL hunk
ownership note, and the current store -> `ple_api` wrapper -> protected
`ple_data` function call path. The older QUALITY rejection concerned
documentation issues and the source line-limit violation; those corrections are
applied, and that rejection is prior history. A separate fresh QUALITY
re-review remains pending. The question about a direct `ple_data` grant versus
the current `ple_api` wrapper has been asked and remains unanswered. Connected
database/API/production-browser acceptance, M05/M06, and full milestone
acceptance remain pending.


ASVS Level 2/3-relevant source controls include a bounded body, closed serde request,
positive typed Revision/Edit Number decoding, parameterized SQL, transactional CAS,
Owner/Sysadmin route checks, no-store responses, and generic failure messages. The
source grant and focused catalog oracle are implemented; connected role tests and
database/API acceptance remain pending.

## Manager boundary clarification (2026-10-06)

The previous handoff sentence saying the `ple_data` versus `ple_api` call-path question
remained unanswered is superseded. Root decision D1 accepts the current path:
`ple_app` calls the `SECURITY DEFINER` `ple_api.save_question_pool_members(...)`
wrapper, which delegates to the protected `ple_data.save_question_pool_members(...)`.
`ple_app` has no `ple_data` schema USAGE or direct data-function EXECUTE grant. This
resolves an implementation boundary; it is not an open product question. The M09
editor lane owns no SQL changes. Source evidence is in the Postgres store call at
`crates/learning-data-access/src/postgres/question_pool_library.rs:85-90` and the
wrapper/data function at `schemas/base_schema/50_functions/question_pools.sql:333-370,620-629`.

Root decision D2 classifies the `ORDER BY ... FOR SHARE` at
`schemas/base_schema/50_functions/question_pools.sql:418-429` as deterministic lock
acquisition for newly added metadata rows. It does not order stored or displayed Pool
membership or affect sampler selection. Preserve this internal lock order; no editor
SQL change is required.

Propagation: the fresh M09 editor SPEC reviewer restated D1 and D2, verified the
current call/lock paths, and returned PASS. A different fresh QUALITY reviewer
returned PASS with no editor findings. Both reviews relied on the recorded focused
check results; neither reran broad suites. The separate M09 order-correction report
retains its separate owner; this clarification does not modify it. Root still owns
connected PostgreSQL/API/browser runtime acceptance, which remains pending.
