# Implementation status and handoff

Last updated: 2026-08-09

This file is a durable execution handoff. The architecture, scope, milestone order, security
boundaries, and acceptance criteria remain authoritative in `implementation_plan.md`. Durable owner
decisions remain authoritative in `../HUMAN_GUIDANCE.md`. If this status disagrees with either
source, follow those sources and correct this file.

## Working rules

- Keep the learning engine question agnostic. Biology examples are fixtures, not product policy.
- Keep answer-bearing content, grading keys, and grading logic server-side.
- Preserve tenant isolation, immutable published content, draft-versus-publication identity, and
  stateless API replicas.
- Preserve the shared dirty worktree. Do not reset, stage, commit, or discard unrelated changes.
- Complete one dependency-ordered slice, run its behavior gates, obtain an independent review, and
  update `../CHANGELOG.md` before advancing.
- Use `source source_me.sh && python3 ...` for repository Python commands.
- Keep maintained source ASCII/ISO-8859-1 compatible. Write non-Latin Unicode in Rust fixtures with
  escapes such as `\u{1F9EC}`.
- `../CLAUDE_HOOK_USAGE_GUIDE.md` is intentionally removed and does not govern Codex work.
- Use `../CODEX_SPARK_SUBAGENTS.md` for bounded Spark delegation.

## Completed and accepted code-first work

The shared tree contains independently reviewed, behavior-tested verticals for:

- immutable catalog publication, search, filters, safe details, and aggregate statistics;
- private workspace ownership/collaboration, revision CAS, safe publication diff, and author
  preview;
- revisioned assignment create/edit with server-owned capability validation;
- generic run, submission, feedback disclosure/release, summary pagination, and next-question
  prefetch;
- server-only native, WeBWorK, iMathAS, and QTI grading/source boundaries;
- private QTI import, publication, published grading, and opt-in production QTI composition;
- assignment export production with four requester-private, prompt-only artifacts;
- identity-free question statistics aggregation, persistence, catalog disclosure, and deletion
  survival;
- retention R1 through R4.4: pure policy, authorized persistence, worker cleanup,
  trusted due dispatch, schedule extension, archive-time assignment disposition, a revisioned
  manager API, and truthful archive completion gates.

These statements describe code-first acceptance. Environment-dependent live PostgreSQL, object
storage, and deployed worker/replica exercises remain one-time deployment gates where documented.

## Previously accepted task: MOD-RETENTION R4.3

The truthful archive boundary is independently accepted. Learner routes now use a central
retention access fence; worker construction is reusable; and archival lifecycle is
truthful only after exact cleanup and idempotent replay.

### Acceptance summary

- Central learner-record archive predicate now fences all learner-facing aliases (courses,
  assignments, enrollments, run/attempt/submission, summary/gradebook, feedback, prefetch,
  external-tool, exports, and StudentRecord-bound assets) via Store access checks and
  PostgreSQL RLS.
- Manager routes retain retained-definition read visibility, while learner rows use the
  central course-fence and return concealed `404` for archived/deleted learners.
- Archive completion is now strictly after exact cleanup, and both Store and server replay are
  exact/idempotent across retry/replay.
- Export, StudentRecord, and external-tool resurrection paths now terminate at the same closed
  archive gate.
- Public catalog asset routes remain unchanged and remain deliverable.
- Real `RetentionJobHandler`, `RetentionJobCommitter`, and `RetentionWorkerComponents` are now
  constructible from production PostgreSQL/object-store dependencies without starting queue
  activation.
- Independent review result: ACCEPT, no P0/P1 findings.
- Current gates are stable: server lib 140, Store lib postgres 44, Store conformance 16,
  Store retention postgres 18, object conformance 2, strict Store/server all-target Clippy,
  rustfmt, ASCII 337, Markdown links 45, PG conformance no-run, global/scoped diff clean.
- `PLE_POSTGRES_TEST_URL` is absent; live forced-RLS, SDF, trigger, and query-plan tests are
  deferred deployment gates.

## Most recently accepted task: MOD-RETENTION R4.4

R4.4A remains independently accepted. R4.4B had reached an accepted functional handoff, but the
later partial-commit audit correctly reopened its transaction-scale design. The non-destructive
R4.4A package froze the boundary needed before permanent deletion:

- persist the exact typed cleanup-object set before the worker deletes any object;
- replay that immutable manifest for a renewed lease on the same bound job;
- require the persisted manifest before cleanup completion;
- add indexed relational course ownership to student-record audit events without parsing JSON; and
- leave relational learner rows and the `deleted` lifecycle untouched until R4.4B supplies the
  complete purge transaction.

R4.4A evidence:

- Memory and PostgreSQL reject `deleteStudentRecords` preparation before mutation while still
  allowing the durable scheduler to dispatch and terminalize the unavailable work explicitly.
- Archive cleanup stores the exact normalized manifest before delivery revocation, replays it only
  for the same bound job under a renewed lease, and requires the prepared manifest plus current
  lease at commit.
- The manifest contains tenant-owned export and external-transcript objects; it never includes
  shared source objects or derives ownership from JSON payloads.
- Student-record audit events now carry indexed relational course ownership; catalog audit events
  remain course-free.
- Independent R4.4A review result: ACCEPT, no P0/P1 findings.
- Current offline gates are green: Store lib postgres 52, Store retention postgres 26, Store
  conformance 16, PostgreSQL conformance no-run, server lib 140, server retention worker 3, strict
  Store/server all-target Clippy, rustfmt, ASCII 337, and diff checks.
- Live PostgreSQL migration/RLS execution remains an environment-backed deployment gate.

### R4.4 test permanence classification

The repository test policy in `../PYTEST_STYLE.md` applies to the Rust suites as a maintenance
standard. R4.4 keeps only deterministic, offline behavior tests that protect durable authorization,
lease/replay, lifecycle, typed-object, and frozen assignment-disposition behavior. Those tests use
inline inputs or same-module helpers; R4.4 adds no committed `tests/fixtures/` data.

The committed fixture inventory now contains only `tests/fixtures/published_problem/`, which
[../HUMAN_GUIDANCE.md](../HUMAN_GUIDANCE.md) explicitly approves as reviewed cross-layer
infrastructure. The small QTI ZIP/base64 and WeBWorK PG inputs are inline beside their parser and
backend tests rather than stored as separate fixture files. New committed fixture directories or
files require explicit human approval; temporary database, object-store, migration, and
reconstruction inputs are removed after their one-time evidence is recorded.

The following are one-time implementation or deployment checks, not permanent suite residents:

- rebuilding an empty PostgreSQL database through every migration, including migration replay;
- loading a representative relational purge graph and exercising broker roles, RLS, foreign-tenant
  concealment, malformed cross-course dependencies, lock ordering, and exact object manifests;
- inspecting migration SQL names, text fragments, statement order, or policy-name lengths; and
- live object-storage, multi-replica, worker-soak, query-plan, and deployment exercises.

Scratch SQL and ignored live-database reconstruction tests used while building R4.4B are removed
after their evidence is recorded. The one-time populated PostgreSQL 17 gate passed on 2026-08-08:
it rebuilt and replayed every migration, exercised malformed but FK-valid cross-course prefetch,
successor, and statistics-receipt links in both endpoint directions, purged every identifying link,
and preserved the control course plus the shared anonymous aggregate. The temporary Rust test, SQL
seed, database, and container were removed afterward; this evidence must not be replaced by a
committed fixture file or an implementation-string test.

R4.4B now materializes the exact course-owned relational purge graph while the archived course is
write-fenced, deletes typed objects idempotently, removes learner rows in verified foreign-key
order, applies the frozen assignment disposition, and sets the coarse retention tombstone to
`studentRecordsDeleted` only after every required effect succeeds. At the original R4.4B handoff,
before the later permanent-test pruning, the focused gates were green: Store lib 51, server lib 140,
Store retention 25, server retention worker 3, Memory conformance 16, PostgreSQL conformance
compile, strict Store/server Clippy, rustfmt, ASCII 340, Markdown links 46, and diff checks. The
independent R4.4B security review at that boundary reported no P0/P1 findings; the later partial-
commit audit findings are tracked separately and supersede that readiness claim.

The 2026-08-08 partial-commit audit reopened two permanent-purge design blockers: global
`EXCLUSIVE` table locks blocked unrelated tenants, and whole-course UUID arrays made the transaction
memory-bound. The working tree now replaces both with a course-retention-row writer fence and
private indexed run/attempt/export work sets. A removed one-time PostgreSQL 17 probe applied the
fresh migration chain, purged 50,000 attempts, overlapped a control-course enrollment write that
completed within two seconds, refused a same-course learner insert after prepare, removed every
attempt, and erased the private work sets before the tombstone. Permanent offline gates pass: Store
lib postgres 30, server lib 137, Memory conformance 11, server retention worker 3, and PostgreSQL
conformance no-run. Fresh independent scalability and security/atomicity rereviews both report
ACCEPT with no P0/P1 findings. Current commit readiness and safe partial-commit options are recorded in
[partial_commit_status.md](partial_commit_status.md); do not commit the mixed index before that
index is rebuilt from the accepted working tree.

## Most recently accepted task: WP-QTI-10

The provenance-aware Memory and PostgreSQL conversion boundary is complete and independently
accepted:

- A closed, non-serializable staged profile-evidence value closes H2 while an import is prepared.
  Exact replay is idempotent; divergent evidence and staging after commit refuse without mutation.
- Conversion requires the committed accepted result to bind both its source identifier and exact
  `itemId` to the selected item, then revalidates the profile tuple and every integrity digest.
- Memory and PostgreSQL atomically advance the draft CAS revision and persist the draft, canonical
  source, current private grading, and current origin under the frozen lock order.
- Ordinary saves atomically stage current private grading and preserve imported origin. Publication
  accepts no grading payload from the caller; it promotes only the locked stored value after origin
  promotion.
- PostgreSQL uses the forced-RLS provenance and grading brokers. The Store implementation performs
  no direct reads of private grading, choice-map, or provenance secret tables.
- `Sha256Digest` JSON is exactly lowercase 64-character hexadecimal text and rejects uppercase,
  wrong-width, and non-hex input.
- Shared Memory/PostgreSQL conformance, PostgreSQL feature coverage, the full fresh baseline, and
  independent review passed. The review reported no P0/P1 finding.
- Detailed evidence is in
  `docs/active_plans/workstreams/qti_memory_postgres_implementation.md`.

The WP-QTI-9 server boundary is complete and independently accepted:

- An author upload stores the exact bounded ZIP as a deterministic private workspace object and
  enqueues one deterministic `qtiImport` job. Exact replay is stable; divergent replay refuses.
- The safe report exposes recognized package/item defaults, diagnostics, and acknowledgement digests
  without answer material, source bytes, object keys, or private mappings.
- The profile worker uses strict Canvas and Blackboard detection ahead of generic parsing, commits
  complete accepted-item evidence, and keeps mixed vendor evidence or all-rejected results from
  producing a conversion candidate.
- Conversion requires a current strong draft ETag plus report revision and acknowledgement tokens,
  rereads and reparses the archive, recompiles through the native bridge, and calls the WP-QTI-8
  atomic Store command. Refusals do not mutate a draft.
- Publication copies the source archive to deterministic non-signable `PublishedImportArchive`.
  Memory and PostgreSQL serialize prepared import work with draft deletion, preventing orphaned
  prepared evidence and unsafe identity reuse.
- Every upload, report, and conversion response is answer-free and `Cache-Control: no-store`; denied
  and absent resources have the same external response. Evidence is in
  `docs/active_plans/workstreams/qti_server_routes_implementation.md`.

The WP-QTI-10 author UI is complete and independently accepted:

- A feature-local browser client sends the exact ZIP as opaque bytes to the existing same-origin
  author route, accepts only strict bounded answer-free DTOs, and requires `no-store` responses.
  Browser code does not parse ZIP/XML or persist archives, reports, mappings, or private answers.
- The existing workspace route shows queued and processing states, manual refresh, accepted and
  rejected cards, defaults, warnings, explicit acknowledgement, all-rejected and unsupported
  recovery, and exact retry after an ambiguous upload. It does not add a product route.
- Conversion is guarded by the current displayed clean strong draft revision. It refetches the same
  route and opens the existing flat editor. The old editor is inert across the committed replacement;
  if refetch fails it stays locked behind an explicit repeatable reload action, with no repeat
  conversion or new import. The converted editor unlocks and receives focus only after a successful
  load.
- Permanent offline Node and real-route Playwright tests passed. The Playwright suite has four
  Chromium scenarios covering retry identity, safe mixed reports, all-rejected and unsupported
  recovery, stale/dirty/revision conflicts, refetch recovery, keyboard behavior, and 375 px reflow.
  `./check_codebase.sh` passed all 11 checks with 173 Node and 184 server tests. Independent security
  and HCI re-reviews reported no P0/P1 finding. Detailed evidence is in
  `docs/active_plans/workstreams/qti_author_ui_implementation.md`.

## Dependency-ordered future work

### Current package order

1. Complete the unstarted WP-QTI-11 live PostgreSQL/RLS/profile-to-native acceptance and WP-QTI-12
   independent documentation close-out in
   [the QTI profile plan](decisions/qti_profile_mapping_plan.md).
2. Implement the compartmentalized M3 course appearance package in
   `docs/active_plans/decisions/course_appearance_plan.md`: 15 measured three-color
   themes, one revisioned centered entry banner, protected object/persistence/API behavior, all-
   course-route theming, and live/browser/visual acceptance.
3. Continue the remaining M5 integration/recovery work below.

The course appearance contract is frozen for execution but is not implemented. It remains separate
and follows WP-QTI-11 and WP-QTI-12 in the QTI dependency order. Its pure theme-scope and settings
workstreams become parallel only at the explicit CA4 boundary; shared object, asset-delivery,
schema, and Store owners remain dependency ordered.

### Pre-M5 database baseline

- Apply the accepted pre-data evolution decision in
  [decisions/database_schema_evolution_plan.md](decisions/database_schema_evolution_plan.md):
  consolidate the working migration diary into the six-file initial epoch and replace the manual
  migration registry with SQLx's checksummed, locking migrator.
- The migration execution seam now uses SQLx's directory-backed migrator, so new migration files
  cannot be omitted from a handwritten Rust registry. Store behavior tests and PostgreSQL
  conformance compilation remain permanent; exact SQL/source-string checks were removed under the
  fixture and permanent-test policy. The ignored credentialed PostgreSQL mega-test was also removed:
  it required an external database, carried complex mutable setup, and never belonged in the fast
  permanent suite. Fresh-install, no-op replay, checksum, missing-version, role/RLS, and concurrent-
  runner exercises remain one-time environment gates for the consolidated six-file baseline rather
  than committed fixtures.
- Run the fresh-install/no-op replay, catalog, forced-RLS, grants, constraint, assignment-CAS,
  run-snapshot, partition, and payload-upcast gates before declaring the first-data boundary.
- Keep this as a deliberate schema task. Do not append new M5 persistence to the 27-file disposable
  history and then preserve another intermediate design.

### M5 integration hardening after retention

- Audit the remaining M5 deliverables against current source: cross-cutting E2E, orphaned-object
  reconciliation, asynchronous analytics, and the retention/security documentation set.
- Object reconciliation currently lacks a bounded object-store inventory API and a database record
  for deterministic WeBWorK/iMathAS render-cache objects. The durable design needs a database-
  authoritative object record, typed domain references, first-observed orphan quarantine, and a
  separate broken-reference alert path that never deletes the database record. Quarantine duration
  remains injected policy rather than a hardcoded permanent-test default.
- Run the plan's combined hostile-input, tenant-isolation, answer-key, partition-pruning, renderer
  outage, course-deletion, and below-k statistics gates together rather than relying on lane-local
  green results.
- Keep environment-backed database, object-storage, replica, and provider exercises one-time unless
  they satisfy the permanent-test checklist.

### Cross-cutting completion and deployment

- Re-audit every implementation-plan requirement against current source and behavior evidence.
- Complete remaining documentation and changelog synchronization without restoring Claude-specific
  guidance.
- Run environment-backed PostgreSQL role/RLS/concurrency tests, object-storage deletion/retry tests,
  multi-replica and worker soak tests, and container/browser deployment checks as one-time gates.
- Add actual server/worker deployment only in the later deployment milestone; code-first readiness
  does not imply operational activation.

## Known operational notes

- The worktree is intentionally broad and dirty from the implementation program; unrelated changes
  belong to the user and other accepted slices.
- Global and scoped `git diff --check` were clean at the R4.4 handoff. Recheck the current shared
  tree before attributing any later formatting failure to this retention slice.
- No credentialed PostgreSQL retention fixture remains in the permanent suite. Fresh role/RLS,
  populated-graph, and migration-replay exercises are recorded as one-time gates and their temporary
  source is removed after execution.
- Cargo artifacts were previously cleaned after `target/` exhausted disk space; rebuild time is
  expected, and source files were not removed.
- Finished agent records cannot be pruned from the current collaboration history. Avoid spawning
  redundant agents; a new conversation is the only way to obtain a short agent-history list.
- GPT-5.3-Codex-Spark is exhausted. Use GPT-5.6 agents for new delegated work and keep each task
  narrowly bounded with one owner, one outcome, and one verification step.
