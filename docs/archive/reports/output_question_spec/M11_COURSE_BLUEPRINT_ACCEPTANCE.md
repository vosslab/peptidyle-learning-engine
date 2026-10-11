# M11 Course-to-Blueprint Publication Acceptance

## Result

Added a registered connected PostgreSQL test for ordinary Course-to-Blueprint publication. The
test reaches `PostgresCourseBlueprintPublicationStore::create_blueprint_from_course_instance`,
reloads the persisted Blueprint, and verifies that its Assessment still references the source
ordinary Pool with the requested `selection_count`.

The source Pool is created through its ordinary Store, then referenced by an unreleased Course
Assessment. It contains one Question while the Assessment requests two selections. The test keeps
the Assessment unreleased so M12 release-time adequacy does not change this M11 identity check.
After publication, the test verifies the Pool ID and count, source Course provenance, unchanged
Instructor ownership, and no new source-linked Pool child.

## Registration and root selector

The test module is registered in `crates/learning-data-access/tests/blueprint_course_postgres.rs`
and belongs to the `blueprint_course_postgres` integration-test target. Its exact test target is
`blueprint_course_postgres_course_publication_pool_reference::course_publication_preserves_an_ordinary_pool_reference`.
The canonical fresh-database baseline now invokes this target serially under the existing
application-role acceptance environment, after the Manual WebWork Type and Sysadmin selectors.

```sh
source ./source_me.sh && cargo test -p learning-data-access --features postgres --test blueprint_course_postgres blueprint_course_postgres_course_publication_pool_reference::course_publication_preserves_an_ordinary_pool_reference -- --ignored --exact --test-threads=1
```

The baseline driver's result key is `Course publication Pool reference PostgreSQL acceptance`.
See the [canonical selector registration note](M11_CANONICAL_SELECTOR_REGISTRATION.md) for its
placement and focused review record.

## Review and focused checks

- Fresh SPEC review found no contract or path mismatch.
- The first fresh QUALITY review caught a Pool ID fixture that could not pass its constructor.
  A fresh owner corrected it to a per-run generated identifier, and a different fresh reviewer
  found no remaining issue.
- The new module passes `rustfmt --edition 2024 --check`.
- `source ./source_me.sh && cargo check -p learning-data-access --test blueprint_course_postgres --no-default-features --features postgres` passes. It reports the existing unused `question_pool_member_pins` warning in `blueprint_course_postgres/support.rs`.
- The parent integration-test driver had two import-order differences in its concurrently edited
  imports. The registration task made only the rustfmt-required import-order cleanup; the new
  registration compiles in the focused Cargo check.

The exact selector above was attempted after correction. The test target compiled, then execution
stopped at `AcceptanceRuntime::load()` with `acceptance runtime: Locator`. After sourcing
`source_me.sh`, `PLE_ACCEPTANCE_RUNTIME_MANIFEST`, `DATABASE_URL`, and
`PLE_MIGRATION_DATABASE_URL` were unset. The test stopped before connecting to a database or
changing data, and no container was started. Rerun the selector when the existing disposable
acceptance runtime is configured.
