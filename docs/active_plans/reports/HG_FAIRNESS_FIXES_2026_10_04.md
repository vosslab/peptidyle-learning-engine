# Assessment fairness fixes - 2026-10-04

## Implemented rule

Once any Student starts an Assessment, trusted writes keep the issued content
stable. They allow only Entry points and Entry order to change. A whole
Question Pool may be removed. It then contributes zero earned and zero
possible points to every existing Attempt, including finalization, history,
gradebook, and active-Attempt totals. A fixed Question cannot yet be removed
after issue because that product decision remains tentative.

An Assessment-owned Pool may remove only a member that has never been issued
to any Student in that Assessment. It cannot add, replace, or reorder members after an
Assessment has started.

## Boundary and concurrency

`ple_data.save_assessment` locks the Assessment root before it calls
`replace_assessment_entries`. Student Attempt start locks that same root before
creating an Attempt and Issued Questions. The Pool-member command also locks
the root and its owned Pool. The database therefore serializes a Student start
with an Instructor content change; a client cannot bypass the rule by calling a
trusted write directly or racing the editor.

## Changed files

- `schemas/base_schema/50_functions/assessments.sql`: post-issue content guard
  and conservative fixed-Question removal lock while the standalone-removal
  decision remains tentative.
- `schemas/base_schema/50_functions/assessment_pool_forks.sql`: allow only
  never-issued member removal after issue.
- `schemas/base_schema/50_functions/grading.sql`: one current-point resolver
  that makes retired Entries zero.
- `schemas/base_schema/50_functions/assessment_attempt_finalization.sql`,
  `assessment_attempt_history.sql`, `student_course_attempt_history.sql`, and
  `assessment_attempt_access.sql`, and `grading_access.sql`: use that resolver.
- `schemas/base_schema/70_grants/grading_access.sql`: keep the internal
  resolver private.
- `tests/e2e/assessment_saved_response/07_assessment_fairness.sql`: durable
  three-Student fairness receipt in the existing disposable PostgreSQL oracle.

## Validation

The historical A-04 probe and its `2 / 2` replay remain unchanged in
`docs/active_plans/reports/hg_compliance_2026_10_04/`. On 2026-10-04, a
temporary corrected-expectation version of the probe removed only its obsolete
`feedback_score` policy field, expected `1 / 1`, and then was restored. The
disposable PostgreSQL 17 replay installed the complete current schema, ran the
normal saved-response oracle, and printed
`a04_retired_pool_excludes_existing_attempt`: a submitted Attempt became
`1 / 1` after its Pool was removed. The same disposable probe also confirmed
that removal of an issued Pool member was rejected and removal of an unissued
member was accepted after issue.

`source ./source_me.sh && ./tests/e2e/e2e_assessment_saved_response.sh` now
installs a fresh PostgreSQL 17 schema and runs seven receipts. Its new fairness
receipt creates Student A (submitted), Student B (active before removal), and
Student C (starts after removal). It proves A's history is `1 / 1`, B's active
access total is one Question and one possible point, and C can start only the
remaining fixed Question with that same total. It also accepts a post-issue
points/order change and unissued-member removal, then rejects changed Pool
content, issued-member removal, and retired-Pool reactivation. The historical
pre-fix artifact remains unchanged.
