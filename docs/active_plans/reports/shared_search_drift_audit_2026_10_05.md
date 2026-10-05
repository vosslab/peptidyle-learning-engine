# Shared-search drift audit, October 5, 2026

## Result

All six fresh independent passes completed: Plan, Test, Style, Docs, Legacy, and Comment.
The initial audit found three code/workflow issues. Follow-up changes correct those issues,
as recorded below. A second set of six independent passes also completed; runtime validation
passed. All 256 screenshots were subsequently published from a clean Demo.
The initial audit itself was not a clean merge approval.

The user requested this audit after the implementation added Instructor phone/square captures
and weakened the laptop-only gates to permit them. That specific drift was corrected before
this audit. Reviewers received the same raw change boundary, authorities, plan, and validation
receipts, without an expected conclusion. The initial audit authorized documentation corrections only. After seeing the findings, the
user explicitly required removal of the obsolete endpoint, challenged the fixture recommendation
against `PYTEST_STYLE.md`, and requested correction of drift before another independent audit.
Production and test-setup corrections were made under that follow-up authorization.

## Initial findings

### High: the Pool owner foreign key does not enforce Instructor ownership

**Evidence.** `schemas/base_schema/20_tables/question_pool.sql:13-14` references only
`account_id`. `schemas/base_schema/50_functions/question_pools.sql:439-443` checks account
existence in the private fork constructor, without checking User Role. Human Guidance's Pool
specification identifies the owner as an Instructor. `docs/DATABASE_STYLE.md:190-193` requires
role-typed composite foreign keys for this invariant; the Account table already has the
corresponding `(account_id, user_role)` unique key.

**Impact.** Trusted SQL callers or a future creation path can persist a non-Instructor owner.
Current API callers deriving the owner from an Instructor session do not make this a database
invariant. The review did not demonstrate a Student-accessible exploit.

**Smallest useful fix.** Add the existing repository's fixed Instructor role carrier and
composite foreign key to the Pool table, align create/fork writes and generated schema docs,
and verify rejection with a one-time connected database check. No new permanent test is
proposed. This is a schema change and remains for implementation approval.

### Medium: the unused standalone Pool discovery stack remains live

**Evidence.** `crates/server/src/question_pool_library.rs:58-60` still registers
`GET /api/question-pools` alongside the required detail route. Its list implementation retains
separate query parsing, filtering, cursors, and paging. `src/api/http_client.ts` still composes
`createQuestionPoolLibraryClient`; `src/api/http_client/question_pool_library.ts:80-105` and
`src/api/question_pool_library_filter.ts` retain the old discovery client and filter. Repository
call-site review found no production UI caller of the list client. Dedicated old discovery tests
still preserve the endpoint. The shared-search schematic calls for removal of duplicated search
machinery after migration.

**Impact.** Two Pool discovery contracts must be maintained even though the migrated Library
uses combined search. Their filters, cursor formats, and results can drift independently.
No current user-facing failure was demonstrated, so this is medium rather than the Legacy
reviewer's proposed high severity.

**Smallest useful fix.** Remove only obsolete list-specific route, client, query/store methods,
exports, and tests. Retain Pool detail, mutation, and any genuinely shared summary decoding.
This crosses server/client/store boundaries and remains for implementation approval.

### Medium: the browser-only Library check has an undeclared fixture prerequisite

**Evidence.** `tests/e2e/e2e_live_demo_question_library.sh:443-450` runs `prove_browser` alone
for `--browser`, while fixture creation occurs in `prove_api`. The browser script waits for
`Shared Search Live Verification Pool` and its chosen member at
`tests/playwright/e2e_live_demo_question_library_browser.mjs:73-78`. The API fixture helper at
`tests/e2e/e2e_live_demo_question_library.sh:177-184` treats any text-search result as proof the
fixture exists; it does not require an exact title or validate the expected membership.

**Impact.** On a fresh Demo without that Pool, the documented browser-only path cannot establish
its prerequisite. A partial-title match can also skip setup and leave the later browser check
waiting for a different object. This is a gate reproducibility defect, not an observed product
search failure.

**Smallest useful fix.** Keep required inputs and setup local to each test/capture, as mandated
by `PYTEST_STYLE.md` fixture policy. Remove duplicate assertions that do not justify their setup
instead of creating shared fixture infrastructure. Each retained path must establish its own
exact object/member prerequisite. Verify independent entry paths with one-time fresh-Demo
evidence. No new permanent test or production seed content is proposed.

**Coordinator correction.** The first recommendation proposed a shared setup step. The user
correctly challenged that recommendation: convenience does not justify a shared fixture.
The current `shared_search_fixture.ts` is also being removed rather than defended merely
because its setup is idempotent.

The Test reviewer also characterized persistent screenshot fixtures as an order-dependency
failure. That broader claim is not accepted: both screenshot scenarios call
`ensureSharedSearchFixture`, and its implementation creates an absent exact-title Pool before
capture. A fresh screenshot publication already succeeded. Reusable Demo mutation alone does
not establish a cleanup requirement. However, this does not justify the shared fixture module
under the fixture policy; local ownership or removal of unnecessary setup is required. No destructive reset or live fixture mutation was performed during this audit.

## Documentation findings corrected

- **Medium, stale durable contracts:** `docs/DATABASE_STRUCTURE.md`, `docs/CONTRACTS.md`, and
  `docs/DESIGN_DECISIONS.md` still described Author storage, per-Question uniqueness, calculated
  licensing, owner metadata, and/or combined search as pending. Corrected the delivered/current
  facts while retaining the genuine open handoff: unordered storage, sortable editor, mismatch
  warnings/release blocking, and classification re-checks. The newly identified owner-role and
  legacy-discovery gaps remain explicit.
- **Medium, acceptance wording:** implementation/schematic introductions presented historical
  M16 acceptance as current without clearly identifying later changes. They now distinguish the
  pre-pruning full run from post-pruning focused checks and disclose this audit's open findings.
  The Plan and Docs reviewers proposed high severity for absent full reruns. This audit treats
  that as an evidence limitation and misleading wording, not a demonstrated production defect
  or a reason to rerun every gate after a documentation/test deletion.
- **Low, historical visual language:** the visual report's top disclaimer was contradicted by
  sections calling removed phone/square evidence current. Renamed those sections as historical,
  withdrew narrow-width acceptance, and linked the later scope correction. No narrow recapture
  is required. Docs and Comment reported the same issue; it is counted once.

The Docs review also proposed moving the plan out of `docs/active_plans/`. That proposal is not
accepted here: the user supplied that exact existing path as the implementation source, and
repository policy preserves existing root-level plans absent an authorized move. Being new
relative to HEAD does not establish when or by whom it was placed there. The index is untouched.

## Pass coverage and disagreements

| Pass | Concrete results | Areas with no additional finding |
| --- | --- | --- |
| Plan | Stale durable contracts; acceptance evidence wording | Content-neutral shared architecture and explicit Pool-validation non-goals |
| Test | Browser fixture prerequisite and weak API lookup; broader lifecycle claim narrowed above | Retained session/picker/domain tests earn their place; removed duplicate harnesses remain removed |
| Style | Missing Instructor-role constraint on Pool owner | Shared state/definition ownership, inspected file sizes and formatting |
| Docs | Acceptance wording and visual evidence; plan-move proposal rejected | Updated Question model's distinction between delivered fields and deferred Pool validation |
| Legacy | Unused standalone Pool discovery stack | No other concrete compatibility shim, dead dependency, or duplicate decoder finding |
| Comment | Historical visual report wording, merged with Docs | No concrete production comment/docstring contradiction |

No entire pass returned zero findings. No reviewer was reused or given editing work. Coordinator
source inspection confirmed the accepted findings and narrowed the Test, Legacy, and acceptance
severity claims as described above. Independent pass reports are retained for this session at
`/tmp/ple_shared_search_audit_{plan,test,style,docs,legacy,comment}.md`.

## Boundary and verification

The audit reviewed the worktree against HEAD, including staged and unstaged changes: 38 Rust
area files, 13 schema files, 64 frontend files, 44 test files, Cargo.lock, and 283 documentation
and screenshot artifacts. Many artifacts are generated images; they were not all visually
re-inspected. Untracked `.agents/` and `skills-lock.json` were outside scope. The complete
changed-file inventory and evidence tails are at `/tmp/ple_shared_search_audit_files.txt` and
`/tmp/ple_shared_search_audit_evidence.md`.

Existing receipts checked:

- Historical M16 full acceptance: `/tmp/ple_shared_search_m16_final_acceptance.log`.
- Post-pruning fast aggregate: `/tmp/ple_test_pruning_fast_clean.log` (541 Node and 10,475 Python,
  with the aggregate's Rust/TypeScript/schema checks).
- Retained UI and live Library checks: `/tmp/ple_test_pruning_ui.log` and
  `/tmp/ple_test_pruning_live.log`.
- Restored laptop scope: static screenshot artifact verification, 20 Node corpus/definition
  tests, and 405 viewport/link checks at `/tmp/ple_staff_corpus_checks.log`.

During this audit the Test reviewer ran 53 focused Node cases successfully. The Style reviewer
ran Rust formatting, changed-file Prettier, and diff whitespace checks successfully. No new
permanent tests were added. The coordinator's final documentation gate is recorded in
`/tmp/ple_shared_search_drift_audit_docs.log`.

Limits of the initial audit: no fresh full `all_test`, full screenshot replay, connected non-Instructor
owner rejection probe, fresh browser-only run, external endpoint traffic inventory, or exhaustive
visual review was performed in this audit. Static source evidence establishes the constraint and
fixture gaps; the proposed implementation fixes still need their focused runtime verification.
Passing earlier gates does not establish Human Guidance alignment or close these findings.


## Follow-up implementation and explicit deferral

After the user required corrections, the obsolete standalone Pool-list endpoint, Rust/store/SQL
list path, grants, frontend client/filter/page decoder, and list-only tests were removed. Pool
detail, creation, mutation, Bloom correction, and Assessment-fork reads remain.

Pool ownership now uses an Instructor role carrier, composite Account foreign key, and covering
index. A clean disposable PostgreSQL installation and rollback-only probe accepted Instructor
ownership and rejected Student and Sysadmin ownership. Schema regeneration and style checks pass.

The shell API check no longer creates data for the browser check. Browser and screenshot
consumers use local inputs/setup near their assertions or captures. The shared fixture module
and fuzzy title reuse are removed. The browser-only check passed against the prior running Demo;
fresh-source browser and screenshot validation follow the rebuild.

The user explicitly deferred the broader Library Object discovery-contract redesign. No edits
were made for that proposed redesign. The existing mixed result wire shape is retained for
this shared-search-interface and screenshot task; the deferred ownership question is not
claimed as resolved.


## Second independent audit and corrections

Six fresh reviewers completed Plan, Test, Style, Docs, Legacy, and Comment passes against the
follow-up changes. Reports are retained at `/tmp/ple_shared_search_reaudit_{plan,test,style,docs,legacy,comment}.md`.

- Plan, Test, Style, and Legacy found stale Rust imports left by removal of standalone Pool
  discovery. These were removed; the complete Rust gate subsequently passed.
- Test identified a remaining shared synthetic fast-UI Question Library fixture. It was removed.
  Each retained consumer now declares only its own minimal inputs locally.
- Plan identified Question-only labels on combined Library results. Search labels, result-region
  names, and empty-state text now describe the Question Library; selectors were updated with them.
- Plan and Docs identified premature completion wording. Current status now separates historical
  M16 evidence from follow-up validation.
- Comment returned no concrete finding after review of the explicitly deferred Pool-validation
  boundary. Style's near-limit file-size observation is advisory, not a reason to create another
  shared fixture abstraction.

The user's subsequent screenshot review required names that identify actual PLE locations.
Picker filenames omit the redundant laptop suffix; Profile settings has a descriptive name;
Library result filenames and captions omit the implementation term shared. Question and Pool
previews belong to Browse, including their Ribbon parent and return links. Capture workflows open
previews from Browse categories. The initial Browse page shows category choices without falsely
claiming that no Library objects match before a category has been selected.

These corrections add no permanent tests. Existing navigation assertions were updated for Browse;
an obsolete empty-state wait was removed from the Bloom workflow check.


## Current validation receipts

- Complete Rust gate: `/tmp/ple_shared_search_drift_rust.log` (passed; no later Rust edits).
- Frontend contract, TypeScript, lint, and formatting gate:
  `/tmp/ple_shared_search_final_frontend.log` (534 checks passed).
- Retained isolated UI suite: `/tmp/ple_shared_search_final_ui.log` (passed).
- Full Python compliance suite: `/tmp/ple_shared_search_final_python.log`
  (10,468 passed, ten file-size warnings).
- Screenshot corpus definitions and artifacts: `/tmp/ple_shared_search_final_corpus_tests.log`
  (20 passed); publication and full live replay passed with 256 captures, including
  76 laptop-only Instructor captures.
- Independent live Library API and browser entry paths:
  `/tmp/ple_shared_search_final_live_library_api.log` and
  `/tmp/ple_shared_search_final_live_library_browser.log` (passed).

The live browser follow-up also removed two setup assumptions: the local Pool is created before
querying it, and checks search for that Pool explicitly rather than assume it appears on the first
unfiltered page. Member exclusion is checked against the exact member's title, followed by the
all-members filter. No product polling or new test infrastructure was introduced.

All three connected acceptance lanes passed at `/tmp/ple_shared_search_final_acceptance.log`:
disposable PostgreSQL schema/authority/persistence, installation-data provision/replay, and
Course-appearance PostgreSQL/MinIO coherence.
The full aggregate script has not been rerun; its component gates completed separately
so the unchanged Rust gate is not repeated merely for a second receipt.


The final Browse initial-state correction preserves the shared error/retry surface when category
loading fails. A temporary browser probe forced one failed request, retried against the real API,
and confirmed recovery to category choices without a false empty-state message. The probe passed
at `/tmp/ple_browse_error_probe.log`; it is one-time implementation evidence, not a permanent test.


Final clean-Demo publication is recorded at `/tmp/ple_shared_search_clean_capture.log`.
The published Instructor corpus contains 76 laptop captures. Nineteen asset paths were renamed;
the registry, manifest, receipt, atlas, galleries, README, and historical documentation links use
the new names. Representative native/WeBWorK previews, mixed results, Profile settings, and the
Assessment picker were inspected visually. The clean capture removes accumulated temporary Pool
examples from earlier attempts; it does not add production seed content or fixture infrastructure.
The full replay receipt precedes the final Browse error-state guard; that guard has its separate
one-time browser proof above. Final publication runs every capture workflow and privacy check.
