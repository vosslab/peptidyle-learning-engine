# Blueprint search fixes - 2026-10-04

This change implements HG-SRCH-02 and HG-SRCH-05 for Public Blueprint Course search.

## Delivered behavior

- Search can sort by Stars, Watches, Adoptions, number of students having taken the Course, and most recent edit. Name remains an additional default sort.
- A result shows its author, institution, classification, Stars, Watches, Adoptions, number of students having taken the Course, and last edit date.
- Student numbers remain a historical aggregate across adopted Course Instances. No Student identity or completion tracking is returned.
- Star counts are visible. Watch counts support comparison and sorting, while the query returns no watcher identity or watcher list.
- Cursors carry the selected sort and its matching count or timestamp so later pages remain ordered and cannot be reused with another sort.

## Ownership and evidence

- `ple_api.list_blueprint_courses` derives the counts and edit time from the existing Blueprint, Star, Watch, Revision, and metadata-event facts. It reads the owner affiliation through the new private Profile helper rather than copying that field into the Blueprint.
- `BlueprintCourseSummaryView` carries the projection through the Store, server, generated browser type, strict decoder, and result renderer.
- `tests/test_blueprint_course_client.mjs` covers author, institution, Star, and Watch result details and strict summary decoding.
- `crates/server/src/blueprint_course/list.rs` checks all required sort wire values and cursor kind matching.
- The connected Blueprint discovery test gives Stars and Watches different
  leaders. Opposing Star/Watch counts detect swapped aggregates,
  and a later save of an older Course separates edit order from creation order.

## Watch-count interpretation

Watches follow the GitHub repository model: an aggregate count supports comparison and sorting,
while watcher identities and lists stay private. The current Instructor keeps the separate
Watch/Unwatch control. This was confirmed during implementation.

## Validation

`cargo tsgen`, `cargo check -p learning-data-access -p question_model`, and
`node --import tsx --test tests/test_blueprint_course_client.mjs` passed (16 tests).

An isolated PostgreSQL 17 container installed the current base schema, real grants, and RLS
policies, then ran the exact ignored test
`blueprint_course_postgres_discovery::discovery_pages_return_250_rows_and_one_blueprint_lookahead`:
**1 passed, 0 failed**. It created 251 Blueprint rows and checked one-row lookahead paging,
Adoptions, Students, Stars, Watches, and most-recent-edit ordering with the projected counts.
The strengthened existing test passed again after the six-pass audit on a fresh
isolated PostgreSQL 17 schema (1 passed, 0 failed).

Connected validation exposed and fixed the schema access boundary for the new aggregate
projection. `ple_api_owner`, the trusted owner of `ple_api.list_blueprint_courses`, now has
SELECT-only access and matching RLS read policies for Star and Watch facts, plus execute access to
the two private Instructor display projections it calls. The browser-facing `ple_app` role still
has neither direct table access nor an RLS policy for individual Star or Watch records.

## Live seed integration

The first live Blueprint list returned 422 because the existing non-login PLE
Example Content publishing Account lacked the new required Instructor Profile.
Its Fall Genetics row had no owner name or affiliation, so summary decoding
rejected the list. The canonical bundled-curriculum context now creates that
Profile while keeping source authorship in the curriculum manifest.

The same idempotent owner-context block was applied to the running disposable
demo. An authenticated application-role query then returned both Blueprint
Courses with nonempty owner names and affiliations. Rust and browser code were
unchanged. A subsequent normal demo restart installed the corrected seed from
scratch, and the Blueprint list, detail, and Question-picker browser captures
passed (`hg_screenshots_blueprint_verified.log`).
