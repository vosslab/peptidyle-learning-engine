# Implementation status and handoff

Last updated: 2026-08-08

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
- retention R1 through R4.2: pure policy, authorized persistence, worker cleanup, trusted due
  dispatch, schedule extension, archive-time assignment disposition, and the revisioned manager
  API.

These statements describe code-first acceptance. Environment-dependent live PostgreSQL, object
storage, and deployed worker/replica exercises remain one-time deployment gates where documented.

## Most recently accepted task: MOD-RETENTION R4.2

The safe instructor retention API is implemented and independently accepted. The Store owns
authorization, optimistic concurrency, replay, notification history, and closed worker dispatch.

### Store state implemented

- Forward migration: `../../schemas/migrations/20260808002400_retention_api.sql`.
- `RetentionRevision` accepts only `1..=i64::MAX`.
- The browser-safe retention view contains only state, assignment-definition disposition, and
  revision.
- Notification reads return only closed intent and authoritative creation time, including a notice
  created in an earlier retained generation.
- Archive/delete return `scheduled`, `inProgress`, or `completed` without exposing stage, job,
  lease, actor, student, object, or source identities.
- A broker-owned, forced-RLS action receipt binds tenant, course, actor, original expected revision,
  exact action/disposition, resulting generation, and stage.
- An original-ETag retry replays through that receipt. A current-revision repeat of the same bound
  scheduled action also replays without replacing its generation or job.
- PostgreSQL locks the active course row before reading the receipt, so concurrent identical
  requests converge after the first commit. Memory serializes the same behavior under its write
  lock.
- Conflicting actor, action, or archive disposition is rejected. Memory and PostgreSQL mutation
  authorization both use `Forbidden` rather than confusing authorization with stale state.

Focused Store gates reported green:

```text
cargo test -p store retention --lib --features postgres
cargo test -p store retention --lib --no-default-features
cargo test -p store --test conformance --features postgres --no-run
cargo clippy -p store --all-targets --features postgres -- -D warnings
cargo fmt --all --check
git diff --check -- crates/store/src/retention.rs crates/store/src/lib.rs \
  crates/store/src/memory.rs crates/store/src/postgres.rs \
  schemas/migrations/20260808002400_retention_api.sql
```

### HTTP contract implemented

All branches must send `Cache-Control: no-store`. Resolve session and course authority before
reading a body or parsing `If-Match` so hostile input cannot become an existence oracle.

| Route | Contract |
| --- | --- |
| `POST /api/courses/{course}/retention/end` | Instructor/admin; exact empty body; Store time; idempotent `200`; safe view and ETag. |
| `GET /api/courses/{course}/retention` | Instructor/admin; safe view plus optional fixed notification copy, intent, and creation time; ETag. |
| `POST /api/courses/{course}/retention/archive` | Instructor/admin; strict `{assignmentDefinitions}`; conditional request; `202` scheduled/in progress or `200` completed replay. |
| `POST /api/courses/{course}/retention/delete` | Instructor/admin; exact empty body; conditional request; `202` scheduled/in progress or `200` completed replay. |
| `PATCH /api/courses/{course}/retention/extend` | Tenant admin only; strict `{additionalDays}`; conditional request; `200`. |

Status conventions:

- no session: `401`;
- learner, outsider, foreign tenant, or missing course: concealed `404`;
- a verified course instructor attempting administrator-only extension: `403`;
- missing `If-Match`: `428`;
- malformed, weak, multiple, zero, or out-of-range `If-Match`: `422`;
- stale or conflicting CAS: `409`;
- unavailable Store: `503`.

Use a strong numeric ETag matching the body revision. R4.2 must leave lifecycle `Active`; archived
and deleted states become truthful only in R4.3 and R4.4.

### Acceptance evidence

- `crates/server/src/retention.rs` implements and permanently tests all five routes; composition
  mounts them with explicit `RetentionStore + RetentionApiStore` bounds.
- Mounted tests cover session and course authority before hostile input, strict bodies, bounded
  strong ETags, safe notification projection, original/current revision replay, conflicting
  actions, administrator-only extension, response status, and `no-store` on every branch.
- The test-support cleanup seed now includes the scheduler's required stage-to-job dispatch
  binding, so the real flaky-object worker retry exercises deletion and exact idempotent recovery.
- Fresh independent Spark review reported ACCEPT with no P0/P1. Report:
  `/tmp/ple-mod-adp-nat.RQcyoY/retention_r4_2_final_review_spark.md`.
- Focused results: 20 server retention tests, 16 PostgreSQL-feature Store retention tests, strict
  Store/server Clippy, rustfmt, ASCII compliance (334), Markdown links (44), PostgreSQL conformance
  compilation, and scoped diff checks all pass.
- Live PostgreSQL broker/RLS/concurrency tests remain compiled and ignored until
  `PLE_POSTGRES_TEST_URL` is provided; this is a deployment gate, not code-first acceptance debt.

## Current in-flight task: MOD-RETENTION R4.3

R4.3 is partially implemented but is **not accepted**. Do not start R4.4 until the access aliases,
worker-construction seam, full behavior gates, and fresh independent review are complete.

### Code currently present

- Forward migration `../../schemas/migrations/20260808002500_retention_archive_access.sql` and
  `CourseRecordsAccessStore` define a lifecycle-opaque course-record access predicate.
- The predicate returns false for a missing course, a foreign tenant, persisted archived/deleted
  state, or a current-generation archive stage already in `started` state.
- Memory ordinary run, attempt, submission, feedback, prefetch, summary, and gradebook paths have
  initial transactional access checks. PostgreSQL has matching RLS-policy replacements for the
  principal relational learner-record tables.
- Exact archive commit changes the stage to completed, lifecycle to archived, and the bound worker
  job to completed in one conditional Store transaction after external cleanup succeeds.
- Real `RetentionJobHandler` and `RetentionJobCommitter` components can be constructed from the
  production PostgreSQL and object-store dependencies without starting a queue drain or deployment
  worker.

The Store/schema implementation handoff is
`/tmp/ple-mod-adp-nat.RQcyoY/retention_r4_3_store_core_impl_fallback.md`. The worker-component
handoff is `/tmp/ple-mod-adp-nat.RQcyoY/retention_r4_3_worker_components_impl_fallback.md`.
These are implementation reports, not acceptance reports.

### Findings that must be resolved before acceptance

- Memory external-tool exchange and launch-session methods still need the central course fence
  before replay, provider-related state mutation, resolve, or revoke behavior. PostgreSQL paths
  rely on nested RLS and require an explicit SECURITY DEFINER/RLS audit.
- Export create/read/load/commit paths must refuse after the pre-cleanup fence so a concurrent
  export cannot recreate protected deliveries during archive cleanup.
- `StudentRecord` asset deliveries currently retain tenant and user ACLs but no direct course
  relationship. R4.3 needs a durable relational course binding and Store authorization check before
  a signed URL can be minted; do not infer ownership from opaque JSON. Public catalog assets must
  remain unaffected.
- Assignment definitions are retained by owner decision. Manager definition reads must remain
  available after archive, while learner assignment aliases must be concealed. A single
  tenant-only assignment RLS policy is not sufficient evidence for both audiences.
- The current worker-component constructor is private to binary composition and primarily proven by
  a unit test. Review whether a later worker entry point can consume it without duplicating private
  composition or accidentally starting the generic unfiltered queue worker.
- Migration `02500` needs fresh independent review of tenant-oracle behavior, actual policy/table
  names, broker grants, forced-RLS execution, nested-policy recursion, and query performance.

### Immediate next actions

1. Finish the Store/schema security audit and correct the protected StudentRecord course binding,
   export fence, external-tool fence, and learner-versus-manager assignment policy.
2. Apply the same predicate to all server aliases before backend, provider, cache, object-signing,
   replay, or healing work. Add counters proving archived requests make zero such calls.
3. Make the real retention handler/committer construction reusable by the future production worker
   entry point without activating a worker runtime or using refusing handlers for other queue
   families.
4. Run Memory/PostgreSQL parity, route alias, worker retry, strict Clippy, formatting, ASCII, diff,
   and PostgreSQL conformance-compilation gates.
5. Obtain a fresh independent P0/P1 review, update `../CHANGELOG.md`, and only then begin R4.4.

## Dependency-ordered future work

### R4.3: truthful archive boundary

- Atomically mark archive only after exact object cleanup succeeds.
- Add one Store/database course-retention access predicate to every learner-record alias: courses,
  assignments, enrollment, run/attempt/submission, summary/gradebook, feedback, prefetch, external
  tools, exports, and protected student-record assets.
- Keep manager retention status readable while learner-record routes become concealed `404`.
- Compose the real retention handler/committer in the production worker construction without
  activating deployment infrastructure yet.
- Add Memory/PostgreSQL parity, route alias, no-backend-call, and cross-replica tests.

### R4.4: permanent student-record purge

- Add indexed relational course ownership for records that currently carry ownership only inside
  opaque event payloads. Never infer purge scope from JSON.
- Freeze a durable exact purge/object-finalization ledger before deleting metadata.
- Terminalize export and external-tool resurrection paths, revoke deliveries, delete exact typed
  objects idempotently, then purge course-owned records in verified FK order.
- Preserve published content, drafts/workspaces, catalog metadata, and anonymous question
  statistics.
- Delete assignment definitions only when the archive-time disposition is `delete`; never traverse
  immutable published references.
- Keep an authorized course/retention tombstone so final coarse status remains truthful.

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
- A global `git diff --check` currently reports an unrelated blank line at the end of `.gitignore`.
  Do not edit it as part of retention unless its owner requests that cleanup; use scoped diff checks
  for R4.2 files and report the global blocker honestly.
- Live PostgreSQL retention fixtures compile but remain ignored when
  `PLE_POSTGRES_TEST_URL` is absent.
- Cargo artifacts were previously cleaned after `target/` exhausted disk space; rebuild time is
  expected, and source files were not removed.
- Finished agent records cannot be pruned from the current collaboration history. Avoid spawning
  redundant agents; a new conversation is the only way to obtain a short agent-history list.
- The quota display most recently showed the general weekly pool at about 1% and the separate
  Spark pool at about 87%, but an actual `gpt-5.3-codex-spark` launch was rejected as exhausted
  until 2026-08-15 08:52. Treat the launcher result as authoritative, keep new agents narrowly
  bounded, and continue critical integration locally when Spark cannot start.
