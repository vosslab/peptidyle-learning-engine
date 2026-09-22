# Plan: Screenshot recovery cleanup

## Context

The 2026-09-21 independent screenshot-recovery audit reviewed a working cross-layer repair
batch. The product and current screenshot workflow were functioning, but the audit identified
provenance gaps, an under-specified security exception, misleading historical documentation, and
two SQL key names that described a relationship the schema did not implement. This plan records
the bounded cleanup and its explicit non-goals.

## Objectives

- Make current screenshot publication receipts bind a required manifest and only the active
  public, Instructor, and Student corpus while Sysadmin capture remains deferred.
- Make the public-ID reservation exception prove its ACL and immutable-trigger substitutes.
- Align presentation child-key names with their existing shared-key foreign keys.
- Preserve the meaningful HOTSPOT pointer, keyboard, reload, submission, and grading proof.
- Leave a reproducible, current-source `plpgsql_check` receipt and accurate documentation.

## Design philosophy

Use the smallest direct repair at each boundary, following KISS and the repository test policy.
Keep permanent checks for provenance and security contracts; keep the HOTSPOT interaction as
uncaptured workflow evidence rather than adding a duplicate permanent test. Use distinct source
names for Published Question and Question Pool identities.

## Scope

- Require `current_capture_manifest.json` in replay staging and bind the active corpus receipt and
  atlas to the public, Instructor, and Student roles.
- Add catalog assertions for PUBLIC/application-role ACLs and the enabled public-ID permanence
  trigger.
- Rename the two presentation child shared-key columns to `question_attempt_id` across schema
  readers and regenerate schema reference documentation.
- Preserve HOTSPOT interaction workflow checks and remove stale screenshot links and current-status
  claims.
- Refresh the disposable `plpgsql_check` receipt against the final base schema.
- Preserve Live Demo stop diagnostics and report failure when owned resources are not released.
- Keep the Live Demo launcher single-purpose by removing its acceptance-only
  `--without-live-demo` mode; retain the lower-level installation-data command's deployment
  option as a separate workflow.
- Give Published Question and Question Pool identities distinct source types and name the exact
  Published Question pin `PublishedQuestionRevisionTuple` with `publishedQuestionId` at the JSON boundary.

## Non-goals

- Do not add `plpgsql_check` to the permanent image or `all_test.sh`.
- Do not refresh or require Sysadmin screenshots in this Student/Instructor corpus pass.
- Do not add implementation-detail tests for the screenshot helper or recreate removed SQLFluff
  style checks.
- Do not preserve a Live Demo launcher mode solely to exercise a negative acceptance fixture.

## Current state summary

- The manifest contains 97 captures; the current receipt and atlas contain 92 active public,
  Instructor, and Student images. Five Sysadmin files remain available but deferred.
- The final disposable PostgreSQL 17 checker pass inspected 272 ordinary PLE PL/pgSQL functions
  and 105 installed trigger functions with zero errors and zero warnings.
- The current source builds the base schema successfully, and the security catalog's first
  authorization block passes under `ple_migrator`.
- The Live Demo launcher now always provisions the complete ordinary graph; the separate Cargo
  installation-data command remains the documented deployment boundary for omitting it.

## Approach

1. Repair publication provenance and role scope, then regenerate receipt and atlas from the
   existing PNGs.
2. Repair security assertions and shared-key SQL names, then regenerate `docs/SCHEMA_TABLES.md`
   and `schemas/catalog_snapshot.json`.
3. Keep the existing uncaptured HOTSPOT behavior proof beside the visual captures.
4. Run focused publication, Markdown-link, static screenshot, shell, schema-style, and repository
   fast checks.

## Files to modify

- `tests/playwright/screenshot_corpus/publication.ts`
- `tests/test_screenshot_corpus.mjs`
- `tests/playwright/screenshot_corpus/scenarios_student_types.ts`
- `tests/e2e/database_baseline_security_catalog.sql`
- `tests/e2e/e2e_installation_data.sh`
- `schemas/base_schema/20_tables/assessment_attempt.sql`
- `schemas/base_schema/50_functions/*` presentation readers
- `docs/PLPGSQL_CHECK_DIAGNOSTIC.md`
- generated screenshot, schema, and compliance documentation

## Verification

- `node --import tsx --test tests/test_screenshot_corpus.mjs tests/test_screenshot_corpus_definition.mjs`
- `source source_me.sh && python3 -m pytest tests/test_markdown_links.py -q`
- `node --import tsx tests/playwright/capture_live_demo_screenshots.mjs --verify-static`
- `source source_me.sh && python3 devel/generate_schema_tables_doc.py && python3 schema_style/check_schema_style.py`
- `bash -n tests/e2e/e2e_installation_data.sh`
- Disposable PostgreSQL 17 base-schema install plus the catalog security block and current-source
  `plpgsql_check` query.
- `git diff --check`, followed by `source ./source_me.sh && ./launchers/run_fast_checks.sh`.
- `source ./source_me.sh && ./launchers/all_test.sh` after the launcher-surface cleanup.

Final evidence on 2026-09-21: the focused lifecycle cleanup tests passed (32 tests), the fast
gate passed with 448 Node tests and 9,069 Python tests, and the definitive `all_test.sh` run
passed its complete Rust, Node, Python, disposable PostgreSQL, ordinary Live Demo
provision/replay, and Course Appearance PostgreSQL/MinIO acceptance lanes. The same run also
verified the completed Published Question/Question Pool identity cleanup and its updated
acceptance fixtures.

## Resolved decisions

- Sysadmin artifacts are explicitly deferred rather than silently sharing the active duplicate-byte
  gate. Their manifest records and files remain available for a later capture lane.
- The shared presentation relationship is a Question Attempt key, so both child columns use
  `question_attempt_id`.
- Published Question and Question Pool identities are now distinct in source as
  `PublishedQuestionId` and `QuestionPoolId`; exact Published Question pins use
  `PublishedQuestionRevisionTuple` and `publishedQuestionId`.
- The Live Demo launcher is single-purpose; the installation-data command retains its explicit
  deployment option because that is a distinct provisioning workflow.

## Documentation close-out requirements

- Keep this plan as the causal map for the cleanup patch and mark acceptance evidence in the
  changelog.
- Keep the independent audit read-only; update generated artifacts through their generators.
- Archive this plan after the final fast and acceptance gates are recorded.
