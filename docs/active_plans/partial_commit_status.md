# Partial commit status

Status recorded 2026-08-08 after the requested six-pass pre-commit audit.

## Commit decision

Do not commit the current index as-is.

The index contains an earlier version of migration `20260808002700` while required SQL fixes and
the final status text remain unstaged. The audit also reopened two high-impact design issues in the
permanent-purge transaction:

- it takes `EXCLUSIVE` locks on shared tables, so one course purge can block unrelated tenants; and
- it materializes all run and attempt identities into PostgreSQL arrays, making a large-course
  purge memory-bound.

These were architecture findings, not formatting defects. Both are corrected in the working tree,
the required disposable PostgreSQL check passed, and two focused independent rereviews report no
P0/P1 findings. The current mixed index still must be rebuilt before it is a safe commit boundary.

## Completed work

The working tree contains these coherent completed changes:

- R4.3 central archive access fencing and production-independent retention worker composition;
- R4.4 typed cleanup manifests, export/external resurrection closure, object deletion retry, and
  a course-scoped set-based relational purge;
- the SQLx directory-backed migration runner, including Cargo rebuild tracking for migration-file
  additions;
- fixture-policy cleanup that keeps only the explicitly approved published-problem corpus and
  inlines small QTI and WeBWorK behavior inputs; and
- removal of migration/source-string tests, an ignored credentialed database mega-test, private
  MemoryStore wiring tests, and compile-only composition tests.

The accepted database evolution decision is recorded in
[database_schema_evolution_plan.md](decisions/database_schema_evolution_plan.md). The six-file
pre-data baseline has not started; the current 34-file chain remains disposable implementation
history.

## Audit results

Six independent passes reviewed the full working tree against `HEAD`.

### Remaining commit gate

1. Rebuild a clean index from the final working tree. The current staged and unstaged migration
   hunks must not be committed separately.

The stale index is independently visible through `git diff --cached --check`: its staged copy of
`20260808002600_retention_cleanup_ledger.sql` still contains trailing whitespace at line 399. The
accepted working-tree migration is clean under `git diff --check`; rebuilding the index replaces
that obsolete staged snapshot rather than editing the accepted SQL again.

### Corrected architecture findings

- Learner-record producers acquire a shared lock on the exact course-retention row; retention
  prepare/commit hold the conflicting lock. A purge therefore freezes only its course.
- Private forced-RLS run, attempt, and export work-set tables replace whole-course UUID arrays and
  drive indexed set-based deletes and residual checks.
- Successful commit erases those educational-record identity work sets before it marks the cleanup
  manifest completed or writes the coarse deletion tombstone.

### Resolved findings

- Added `crates/store/build.rs` so Cargo rebuilds the embedded SQLx migrator when migration files
  change.
- Removed fragile private-state, compile-only, source-string, and credentialed ignored tests.
- Corrected [RETENTION_POLICY.md](../RETENTION_POLICY.md) to describe a distinct delete-stage
  manifest and removed an unsupported numeric backup-window promise.
- Converted retention-policy references to relative Markdown links and marked old gate counts as
  historical evidence rather than current results.
- Updated stale retention module and composition comments.
- Fresh scalability and security/atomicity rereviews accepted the course-scoped, set-based purge
  with no P0/P1 findings.

## Validation evidence

Permanent behavior and compilation evidence after test pruning:

- `cargo test -p store --lib --features postgres`: 30 passed.
- `cargo test -p server_core --lib`: 137 passed.
- `cargo test -p store --test conformance --no-default-features`: 11 passed.
- `cargo test -p store --test conformance --features postgres --no-run`: compiled.
- strict Store and server all-target Clippy: passed.
- `cargo fmt --all -- --check` and `git diff --check`: passed.
- ASCII compliance: 340 passed.

The Markdown-link gate currently reports only the two new untracked status/policy documents as
missing GitHub targets. This is an honest commit-preparation failure: GitHub would also return 404
unless those documents are included. Re-run the 46-link gate after rebuilding the intended index
with both documents.

One-time evidence, deliberately not retained as a test fixture:

- PostgreSQL 17 applied all 34 current migrations to an empty database and a second SQLx run was a
  no-op.
- The temporary Rust test, schema dump, database container, and credentials were removed.
- This proves current-chain migration execution only. It does not prove bounded purge behavior or
  unrelated-tenant availability.
- A second fresh PostgreSQL 17 database loaded one course with 50,000 attempts and a separate
  control course. Delete prepare persisted exactly 50,000 attempt-scope rows and rejected a new
  same-course learner membership after archive. While delete commit was visibly active, the control
  course inserted an enrollment within two seconds. Commit removed every target attempt, erased all
  private work-set rows, and wrote the deleted lifecycle.
- The temporary Rust test, both PostgreSQL containers, and their credentials were removed. This is
  scale/concurrency acceptance evidence, not permanent fixture infrastructure.

## Partial commit options

The current broad tree is not safe for one file-level retention commit until the mixed index is
rebuilt from the accepted working tree.

A small independent fixture-policy commit can include:

- `crates/adapters/qti/src/parser_stub.rs`;
- `crates/adapters/webwork/src/lib.rs`;
- `crates/server/src/qti_backend.rs`;
- `crates/server/src/qti_import.rs`;
- `crates/server/src/qti_publication.rs`;
- `crates/server/src/webwork_backend.rs`; and
- the deleted files under `tests/fixtures/qti/` and `tests/fixtures/webwork/`.

The SQLx seam can be a separate hunk-level commit containing `Cargo.toml`, `Cargo.lock`,
`crates/store/build.rs`, and only the migrator hunk in `crates/store/src/postgres.rs`. Do not stage
the whole PostgreSQL Store file for that commit because it also contains retention work.

When the retention implementation is committed, include the currently untracked
[RETENTION_POLICY.md](../RETENTION_POLICY.md). Exclude the unrelated untracked Blackboard instructor
guide PDF unless the owner intentionally chooses a separate documentation commit.

## Next implementation steps

1. Run ASCII and Markdown-link gates on the final tree; strict Clippy, formatting, and diff already
   pass.
2. Rebuild and inspect the intended Git index before committing.
3. After the partial commit, resume the six-file pre-data baseline, then M5 object reconciliation.
