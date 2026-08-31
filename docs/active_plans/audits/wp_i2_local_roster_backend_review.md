# WP-I2 local roster backend review

## Final verdict

**ACCEPTED.** WP-I2 now meets the local-only roster backend contract. The
migration makes the local-development source's null-email/null-roster shape a
database invariant; identity selection remains alias-only and server-derived;
both Store implementations preserve atomic/idempotent membership and enrollment
behavior; and the required live PostgreSQL path is permanently part of the
isolated baseline.

## Verified contract

- `schemas/migrations/2026080913_local_development_roster.sql:9-21` has three
  explicit source branches. `legacy` preserves its compatibility allowance,
  `invitation` requires all managed identity fields, and
  `local_development` requires normalized email, delivery email, and roster ID
  all to be null.
- Local identity configuration validates a unique 1--128 byte lowercase ASCII
  alias per record. The route accepts only that alias, then derives the
  user, display name, and role from server-owned local configuration
  (`crates/server/src/composition/local_identity.rs:79-168` and
  `crates/server/src/course/roster.rs:437-485`).
- The directory only resolves a configured record with exactly the student
  role. Manager authorization is rechecked at both HTTP and Store boundaries;
  unknown, nonmanager, unaffiliated, and conflicting-role paths fail closed.
- The internal source type is closed and PostgreSQL decoding fails closed on an
  unknown stored source. The repaired roster query selects `source`, so live
  local-development records retain their honest projection
  (`crates/learning-data-access/src/postgres/course_roster.rs:44-73`).
- Memory performs the operation under one write lock and restores state on
  error. PostgreSQL uses one database transaction, locks the course roster
  cross-product, rechecks manager authority, and commits only after roster,
  course-membership, enrollment, and summary work succeeds.
- Production passes no local roster directory while exact local composition
  supplies it; the production-style route test still confirms endpoint absence.
  No inspected local path creates or mutates an account, invitation, email
  challenge, email field, or canonical identity.

## Required evidence now present

- The focused server test creates two existing assignments, concurrently
  activates the configured student, proves a single redacted local-development
  member, checks unknown/nonmanager refusal and conflict rollback, and observes
  both assignment rows (`crates/server/src/course/tests/roster.rs:396-628`).
- The ignored live PostgreSQL test seeds two existing assignments before
  activation; concurrent requests yield exactly one student membership, two
  student enrollments for those assignment IDs, and two joined summary rows. It
  also covers source/null fields, manager/nonmanager/foreign boundaries, and
  rollback of conflict side effects
  (`crates/learning-data-access/tests/postgres_enrollment_live.rs:414-620`).
- `tests/e2e/e2e_database_baseline.sh:223-239` invokes the exact ignored test
  immediately after the established passwordless roster/role lane. Reported
  isolated-Podman evidence migrated 0913 and passed both lanes while leaving
  retained volumes untouched.

## Independent commands this re-review ran

- `cargo test -p server_core course::tests::roster -- --nocapture` -- 4 passed.
- `cargo test -p learning-data-access --test conformance -- enrollment --nocapture`
  -- 9 passed.
- `cargo clippy -p learning-data-access --features postgres --all-targets -- -D warnings`
  -- passed.
- `npx prettier --check docs/active_plans/audits/wp_i2_local_roster_backend_review.md`
  and `bash -n tests/e2e/e2e_database_baseline.sh` -- passed.

The ignored PostgreSQL test deliberately requires `PLE_TEST_DATABASE_URL`; a
direct invocation without the repository's isolated database launcher refused
that missing precondition, as designed. Its permanent baseline wiring and the
reported disposable run provide the live evidence for this acceptance.
