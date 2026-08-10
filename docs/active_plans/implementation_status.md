# Implementation status and handoff

Last updated: 2026-08-10

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
  manager API, and truthful archive completion gates; and
- course appearance WP-CA1 through WP-CA7/WP-RC1: closed themes, Grass default, revisioned
  persistence, protected current-only banner objects, safe image normalization, all-seven-route
  Solid scope, keyboard-complete settings, live PostgreSQL/MinIO cleanup, visual evidence, and
  independent acceptance; and
- production-seam closure WP-RC2: implemented H5P/QTI/WeBWorK module names, no native renderer
  declaration, explicit catalog resolve/search Store capabilities, and durable feedback-release
  projection with independent no-P0/P1 review.

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
- `PLE_POSTGRES_TEST_URL` was absent at that handoff; WP-RC7 and WP-RC12 own the live forced-RLS,
  SDF, trigger, and query-plan production-activation evidence and require PASS before release.

## Accepted task: MOD-RETENTION R4.4

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

## Most recently accepted task: WP-QTI-12

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

The WP-QTI-11 live gate is complete:

- A fresh isolated PostgreSQL 17 database applied and verified the six-file SQLx baseline and ran
  the real profile upload, worker, conversion, publication, native grading, and cleanup path.
- One minimized Canvas archive produced an accepted item and a visible rejected sibling. The
  accepted item remained editable, published as native flat content, and graded one correct and one
  incorrect response through the isolated PostgreSQL grader.
- Workspace and published archive bytes, canonical source, current and published origins, and their
  checksums agreed. Workspace cleanup removed current private state while immutable published
  provenance remained.
- Application, student, grader, and foreign-tenant probes enforced the RLS and protected-capability
  boundaries. Safe DTO scans found no archive bytes, object keys, correct-choice material, private
  choice maps, grader payloads, or unreleased feedback.
- The full disposable database gate, focused Rust/Node/Playwright checks, strict workspace Clippy,
  workspace tests, all 11 repository checks, 51 built Playwright scenarios, and 1,644 Python tests
  passed. Detailed evidence is in
  `docs/active_plans/workstreams/qti_live_acceptance_implementation.md`.

The WP-QTI-12 independent review and documentation close-out is complete:

- Six separate plan, test, style, documentation, legacy, and comment review passes found no
  production or test defect.
- Documentation review found stale README status plus missing profile-to-native ownership evidence
  in the contracts, code architecture, and file map. The four owner documents were corrected and the
  original reviewer confirmed both findings resolved.
- The contract and architecture maps now name the profile parser, author upload/report route, worker,
  conversion bridge, Solid author workflow, protected grader boundary, and disposable PostgreSQL/RLS
  oracle. That historical closeout assigned future QTI-JSONL ownership externally; the current owner
  decision instead uses PLE flat JSON v2 as the internal all-family source contract.
- Focused Markdown link, ASCII, README first-paragraph, whitespace, and Prettier gates passed. No
  P0/P1 finding remains.

## Recent owner-requested support work

- The current PLE-owned student browser flow and all implemented response families passed a focused
  no-mouse audit. The primary route uses Tab, Shift+Tab, Space, explicit submission, and native link
  activation; Arrow, digit, Enter-to-submit, and Escape extensions have separately classified
  component scenarios. Representative VoiceOver and NVDA sessions remain a fall-pilot human gate.
- `launch_local_stack.sh` is the maintained all-in-one local test front door. It preflights the
  private configuration, generates ignored local identities and secrets, builds the code, migrates
  and seeds PostgreSQL before API/worker startup, provisions the distinct grader login, waits for
  the semantic gateway health response, and opens the browser without deleting persistent volumes.
  The accepted WeBWorK renderer remains an explicit optional profile using the pinned upstream
  `/render_rpc` integration.
- [docs/DATABASE_STRUCTURE.md](../DATABASE_STRUCTURE.md) maps implemented revision, assignment, and
  isolated-score relations. WP-RC8 now owns institutional OIDC; passkeys, local passwords, and
  email-code login are explicitly out of version 1 scope. The document records pilot and
  ten-million-question growth formulas without claiming institutional credentials or legal sign-off.

## Most recently accepted task: WP-RC1 course appearance

WP-CA1 through WP-CA7 and WP-RC1 are accepted on 2026-08-09. The production contract, Store/RLS,
object, server, Solid, and learner-entry owners now implement one of 15 measured themes and one
optional exact 1200 by 328 course-entry banner. The instructor settings page is keyboard complete,
preserves local state through validation/network/auth/permission/CAS failures, and supports explicit
reload, replacement, and removal. A bounded request-triggered cleanup executes the real claim,
tenant-owned MinIO deletion, and completion sequence without deleting the exact current object.

Acceptance evidence is recorded in
`docs/active_plans/workstreams/course_appearance_implementation.md`: the disposable PostgreSQL and
MinIO stack passed, including the `ple_app` cross-course pointer refusal and combined lifecycle;
`./check_codebase.sh` passed all 11 checks; the rebuilt browser passed 62 tests with the opt-in
visual generator separately passing 1/1; Python passed 1,743 tests; staged and unstaged diff checks
are clean; and three independent reviewers reported no P0/P1/P2.

## Dependency-ordered remaining work

### Current package order

The complete sequence is authoritative in
`docs/active_plans/active/release_completion_plan.md`:

1. WP-RC1 course appearance is accepted.
2. WP-RC2 production-seam closure is accepted. WP-RC3's pinned upstream WeBWorK `/render_rpc`
   integration is accepted after its live PLE/browser gate and final independent review.
3. WP-ARCH1 is accepted. Its dated 26-file maintained-source baseline now has zero maintained-code
   violations behind stable facades; the permanent size gate (582 tests), 2,451-test Python suite,
   eleven-stage codebase gate, and 72-pass browser suite are green. Its disposable PostgreSQL
   migration/RLS/conformance baseline also passes through the decomposed owners, and independent
   PostgreSQL, security, provider, TypeScript/HCI, test, size-policy, and architecture reviews found
   no unresolved P0/P1 issue.
4. WP-RC4's PLE flat JSON v2 implementation now covers the eight source/runtime families and awaits
   independent closeout; external QTI-JSONL is no longer a prerequisite.
5. WP-P1 through WP-P6 implement and accept the secure learner-payload boundary before WP-RC5
   acceptance. WP-P2 adds `2026080908_secure_question_grading_payloads.sql`; WP-RC5 then completes
   visual authoring, all-family Memory/PostgreSQL acceptance, and the two exact Chapter 1
   assignments, while WP-RC6 closes QTI export and H5P claims.
6. After WP-P2, WP-RC7 adds bounded inventory, object reconciliation,
   `2026080909_object_reconciliation.sql`, and the combined M2-M5 acceptance gate.
7. WP-RC8 implements institutional OIDC with `2026080910_oidc_identity.sql`; WP-RC9 implements LTI
   Advantage with `2026080911_lti_advantage.sql`.
8. WP-RC10 adds OpenTofu under `deploy/opentofu/`; WP-RC11 adds the measured bot-cost controls.
9. WP-RC12 runs working-codebase release acceptance and documentation closure after WP-ARCH1.

The accepted pre-data schema evolution is complete. SQLx owns the directory-backed migration ledger,
and `2026080907_course_appearance.sql` is the first forward migration. The release plan reserves
`2026080908_secure_question_grading_payloads.sql`, `2026080909_object_reconciliation.sql`,
`2026080910_oidc_identity.sql`, and `2026080911_lti_advantage.sql`; accepted filenames are not
renamed or reordered.

### Immediate packages: WP-RC4 closeout and secure learner payload

- WP-RC4 owner: native adapter/runtime owner plus an independent contract/security reviewer.
- Implemented behavior: PLE flat JSON v2 strictly compiles MC, MA, FIB, MULTI-FIB, NUM, MATCH,
  ORDER, and HOTSPOT into answer-free public definitions and bound grader-only keys. Exact browser
  decoders, key-free validation, learner controls, and all-or-nothing server grading cover the new
  response shapes; version 1 single choice remains compatible.
- Remaining RC4 acceptance: complete invalid-fixture review, secret-free projection scan, and
  independent contract/security verdict.
- Next dependency: accept WP-P1 through WP-P6 before WP-RC5's visual authoring, integrated storage,
  and pilot-content closeout.

## Known operational notes

- WP-QTI-11 started from clean `main` at `b297808`; its bounded implementation and later accepted
  work now share a mixed staged/unstaged worktree for owner review. Preserve unrelated user changes
  and do not alter the index.
- Global and scoped `git diff --check` were clean at the R4.4 handoff. Recheck the current shared
  tree before attributing any later formatting failure to this retention slice.
- No credentialed PostgreSQL retention fixture remains in the permanent suite. Fresh role/RLS,
  populated-graph, and migration-replay exercises are recorded as one-time gates and their temporary
  source is removed after execution.
- Cargo artifacts are cleaned as needed during implementation work; source files are never removed.
- Finished agent records cannot be pruned from the current collaboration history. Avoid spawning
  redundant agents; a new conversation is the only way to obtain a short agent-history list.
- Use Spark for simple, bounded independent work under
  [docs/CODEX_SPARK_SUBAGENTS.md](../CODEX_SPARK_SUBAGENTS.md); retain manager ownership
  for architecture, coordination, difficult cross-cutting decisions, and final integration.
