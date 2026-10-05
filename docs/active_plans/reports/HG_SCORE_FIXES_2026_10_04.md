# Human Guidance score and answer-release fixes - 2026-10-04

## Settled behavior implemented

- Automated Question scores and Assessment totals have no configurable release setting. A submitted Attempt exposes its current automated score as soon as grading finishes. The Student UI says `Score pending` only while automatic grading is incomplete or stale.
- New Quiz and Exam Assessments default Question Answers and Answer Explanations to `after_all_students_complete`. The Instructor can select another allowed answer timing, including `after_submit`, when a Student does not complete the Assessment.
- Issued Questions, responses, and grading stay pinned to the Attempt snapshot. The history read uses the Assessment's current disclosure rule only, so a later answer-release change applies to an existing Attempt.
- Optional Question feedback timing remains undecided and unchanged.

## Main changes

- Removed `feedback_score` from the PostgreSQL enum consumers, policy snapshot, creation, template copy, Blueprint transfer, recovery data, API decoder, generated contract, settings pages, and test fixtures.
- Added `after_all_students_complete` as a feedback timing. New Assessment creation and the Rust Template initializer choose it for Quiz and Exam answer material.
- Limited that timing to Question Answer and Answer Explanation in both browser controls and the PostgreSQL policy boundary. The database rejects a crafted request that applies it to correctness, responses, class statistics, hints, or worked solutions.
- Replaced the old Assessment-Type hard gate with a check that runs only when the current rule is `after_all_students_complete`.
- Aligned score calculations with the shared retired-entry helper supplied by the fairness workstream.
- Regenerated `docs/SCHEMA_TABLES.md` and `schemas/catalog_snapshot.json`.

## Security boundary

The trusted PostgreSQL policy function validates the limited timing use (ASVS 2.2.1 and 2.2.2). The current-cohort completion value comes from the authorized history read boundary (ASVS 2.3.1). Browser filtering is usability support; it is not the enforcement point.

## Checks run

- `cargo test -p question_model assessment_activity_rules::tests` - 10 passed, including the Quiz/Exam default.
- `cargo test -p domain student_feedback_release::tests` - 15 passed, including the completion hold and explicit `after_submit` override.
- `cargo test -p server_core assessment_delivery::history::tests` - 11 passed, including configured Quiz answer release after every current Student completes and the Instructor `after_submit` override.
- `node --import tsx --test tests/test_assessment_template_settings_model.mjs tests/test_assignment_workspace_questions.mjs tests/test_student_course_progress_presentation.mjs` - 13 passed.
- `git diff --check` - passed.
- `source ./source_me.sh && devel/generate_schema_tables_doc.py && schema_style/check_schema_style.py` - passed.

Integration validation also corrected Blueprint's closed-content validator: its
feedback object now omits the removed score-timing field. The WebWork answer-review
test now applies the selected completion rule and verifies the Instructor override.
The full Rust workspace gate passes, including both feature configurations and
strict Clippy.

Live acceptance found one missing Rust read-side mapping: PostgreSQL created a
Quiz correctly, but the workspace decoder rejected `after_all_students_complete`
and returned HTTP 422. The decoder now accepts that enum value; its focused
regression passed in the full Rust gate.

After the application rebuild, a disposable browser/API probe passed against
`https://localhost:8263/`. The actual Quiz Properties page showed the all-completed
default for correct answers and explanations, exposed no score-delay control,
and saved an `after_submit` answer override that the API returned on readback.
A separate unreleased Exam creation returned both all-completed defaults. The
receipt is `/private/tmp/hg_live_policy_probe_final.log`; the probe created no
released Assessment or Student work.
