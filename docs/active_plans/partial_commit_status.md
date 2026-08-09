# Partial commit status

Status recorded 2026-08-08 at the audited database-evolution checkpoint through mutable assignment
timing and resolved student/group policy exceptions.

## Commit boundary

This checkpoint is one coherent cross-layer transition from the disposable 34-migration history to
the accepted six-file pre-data SQLx baseline. It includes the migrations, Store contracts and both
backends, scoring/timing workers, server integrations, generated browser contracts, fixtures,
tests, and documentation that use the new schema.

The separately staged `docs/how-to-reduce-impact-of-bot-traffic.md` article is unrelated user work
and is intentionally excluded from this database checkpoint.

## Completed work

The dependency-ordered database-plan packages now complete:

- exactly six domain-owned migrations for principals, catalog/authoring, courses/assignments,
  activity/feedback, operations/analytics, and retention;
- explicit `cargo xtask database status`, `migrate`, and `verify` operations, with verify-only
  application startup and exact SQLx ledger/checksum compatibility;
- human-readable catalog problem IDs and versions while UUID identity remains authoritative;
- normalized stable assignment items, pinned selection candidates, immutable delivered run order,
  exact decimal point values, and explicit attempt states;
- generation-fenced current scoring with private staging, atomic newest-generation publication,
  concurrent-submission restaging, and no scoring-history tables;
- revision-checked Delete and Regrade, future-run omission, protected submitted evidence, and
  recalculation;
- direct-instructor force-submit and clear with stable action IDs, minimal audit evidence,
  retry serialization, and no fabricated student response or grade;
- mutable visibility, availability, due/close boundaries, late policy, time limits, attempt limits,
  and generation-fenced durable auto-submit; and
- revisioned direct-student and course-group policy exceptions. Each dimension chooses the most
  permissive applicable value, issued attempts record the resolved policy and contributors, and
  exception/group/course-membership changes atomically re-resolve active work.

When a removed accommodation exposes an elapsed deadline, the active attempt auto-submits in the
same transaction. It records an authoritative submission time but creates no response, evaluation,
or score. Course roster replacement removes invalid group membership in both Store backends;
stable group identities cannot move between courses. Retention fences and purges group membership
and direct-student exception records.

## Independent audit

Six fresh reviewers independently audited plan conformance, tests, style, documentation,
legacy/dead code, and comments. The integrated fixes were:

- course membership removal/demotion now owns accommodation recomputation and active timing updates;
- direct-student exceptions and group memberships are covered by retention fences, broker policies,
  purge order, residual assertions, and MemoryStore cleanup;
- MemoryStore now matches PostgreSQL by rejecting movement of an existing group ID between courses;
- the PostgreSQL acceptance fixture now exercises combined student/group resolution, recorded
  attempt policy, membership-triggered immediate auto-submit, and exception cleanup; and
- the assignment advisory-lock and deterministic multi-assignment lock order are documented.

No fragile tests, additional dead code, or stale execution-workstream comments were found.

One high audit finding is deliberately not hidden: scoring and timing job handlers exist, but the
production composition does not yet start a queue drain loop. This is the same documented boundary
as the other worker families. The generic claim is not family-filtered, so starting a partial
registry could consume another family's job. Production worker claim filtering, complete registry
composition, process/container activation, and operational monitoring remain MOD-WORKER work.

## Migration safety evidence

PostgreSQL 17 acceptance was rerun after the final schema and Store changes on a newly created,
empty disposable database:

1. pre-migration status reported an absent ledger and exactly six pending migrations;
2. all six migrations applied successfully;
3. a second migration run completed without applying another migration;
4. status reported all six exact versions/checksums applied and compatible;
5. verify reported the application compatible;
6. the production PostgreSQL Store completed catalog, assignment, scoring-generation,
   concurrent-submission, Delete and Regrade, force-submit, clear, base timing, stale-generation
   rescheduling, student/group exception, membership-removal, and cleanup behavior; and
7. final verification remained compatible after live behavior.

Direct inspection of the exception case found one auto-submitted attempt with a submission
timestamp, zero submission rows, zero evaluation rows, zero score rows, only the direct-student
resolution after group membership disappeared, and zero remaining exception or group-member rows.

This is still a pre-data baseline. Once any environment accepts real durable data on this epoch,
these applied migrations must never be edited in place; every later change must be a new forward
migration. The disposable acceptance container/database was removed after final validation.

## Permanent validation evidence

- `./check_codebase.sh`: all 11 stages passed.
- Browser/Node contract suite: 148 passed.
- Store unit suite with PostgreSQL features: 40 passed.
- Store conformance suite: 13 passed.
- Server unit suite: 137 passed.
- Rust formatting, strict workspace Clippy, workspace tests, and doctests: passed.
- TypeScript generation, fixtures, type checking, linting, formatting, and Node tests: passed.
- Fresh migration, second no-op, status, verify, and live PostgreSQL behavior: passed.

## Remaining implementation order

1. Add manual grading and mixed automatic/manual assignment behavior.
2. Add course-local item-analysis recalculation after rescoring.
3. Complete import partial-failure, provenance, hostile-archive, and duplicate-warning acceptance.
4. Complete family-filtered production queue draining and the full worker registry/runtime.
5. Run the remaining real-role/RLS, partition-pruning, purge, backup/restore, and whole-system
   PostgreSQL acceptance gates from the database evolution plan.
6. Refresh the stale README body when the human explicitly authorizes the broader edit required by
   `docs/REPO_STYLE.md`; the first paragraph remains accurate.
