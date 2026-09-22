# Screenshot repair code audit

Date: 2026-09-21

Status: Read-only, six-pass review of the live working tree. This report assesses source
quality directly; it does not treat pre-built gate results as a substitute for code review.

## Scope

The coding manager's stated repair sequence was:

1. Finish the `plpgsql_check` cleanup.
2. Make `./launchers/all_test.sh` green.
3. Make the Live Demo build through `./launchers/run_live_demo.sh`.
4. Generate new screenshots through `devel/capture_screenshots.py`.

The review snapshot contained 78 modified files: cross-layer PostgreSQL, Rust, seed, E2E,
documentation, and screenshot-workflow repairs. Reviewers checked the current source against
`docs/HUMAN_GUIDANCE.md`, terminology, database, Rust, TypeScript, test, and repository style
authorities. The worktree was active during review; revisit findings after any material edit.

## Conclusion

The repair batch improves several boundary names, but it does not follow Human Guidance's
distinct-concept naming rule throughout. The foundational Published Question lineage identity
and exact Published Question Revision still use generic `QuestionId` and `QuestionRevisionTuple`
names despite the codebase's separate Draft Question, Published Question, Question Pool, and
Question Image Asset concepts. Do not merge the snapshot unchanged: that naming conflict needs a
planned direct pre-production rename, one check command writes generated source, and the
canonical screenshot workflow retains temporary debugging instrumentation.

## Findings

### High: Published Question lineage uses generic Question names

Human Guidance distinguishes a generic Question, a Draft Question, a Published Question, a
Question Revision, a Question Pool, and Question Image Assets. Its instruction is direct: give
distinct concepts distinct names, and prefer the clearest longer name. The current
`QuestionRevisionTuple { question_id, revision_number }` conceals that its first member is the
identity of a Published Question, not the identity of an arbitrary Question or a Draft Question.
The same generic name propagates across Rust, generated TypeScript, JSON, and the terminology
contract itself.

- Evidence: `docs/HUMAN_GUIDANCE.md:148-159,93-96`;
  `crates/question_model/src/question_library.rs:163-167,413-415`;
  `generated/api/QuestionRevisionTuple.ts:10-14`; and
  `docs/TERMINOLOGY_CONTRACT.md:35,137,238-249,777-789`.
- Impact: an identifier's owner must be inferred from surrounding context. That is exactly the
  ambiguity the Human Guidance rule rejects, and it will worsen as other Question classes gain
  identities and revision-like state.
- Smallest durable correction: first correct the terminology contract, then make one direct,
  cross-layer pre-production rename with no compatibility aliases:
  `QuestionId` to `PublishedQuestionId`, `QuestionRevisionTuple` to
  `PublishedQuestionRevisionTuple`, `question_id` to `published_question_id`, and JSON
  `questionRevisionTuple` / `questionId` to
  `publishedQuestionRevisionTuple` / `publishedQuestionId`. Keep Draft Question and every other
  Question-class identity equally explicit at the same boundary.

### High: Rust check command writes generated source

`check_rust.sh` runs `devel/generate_question_id_contract.py` without `--check`.
The generator's own docstring says its default operation refreshes the generated TypeScript
contract and `--check` detects staleness. A check command that rewrites source can silently
repair an outdated generated contract and leave a dirty worktree instead of reporting the
drift.

- Evidence: `check_rust.sh:72-73`; `devel/generate_question_id_contract.py:2-5`.
- Smallest correction: invoke the generator with `--check`. Keep deliberate regeneration as a
  separately named developer action.

### High: Changelog date conflicts with the diagnostic receipt

The current repair claims live under `## 2026-09-20`, while the new diagnostic says its
current-source cleanup rerun occurred on 2026-09-21. Changelog chronology is implementation
evidence, so this makes the historical record inaccurate.

- Evidence: `docs/CHANGELOG.md:9,33-63`; `docs/PLPGSQL_CHECK_DIAGNOSTIC.md:3-5,137-155`.
- Smallest correction: add a `## 2026-09-21` block and move the newly added repair bullets there;
  retain the actual September 20 warm-loop history in its existing block.

### Medium: Temporary tracing modifies the permanent screenshot workflow

The canonical HOTSPOT and question-type scenarios include 25 explicitly temporary `console.log`
traces. The changes also add a broad response listener and await a POST URL containing
`/publish`, without asserting its status. That creates incidental network and timing coupling
where the visible `Published` heading already expresses the user-facing completion condition.
It also conflicts with the screenshot contract's normal visible workflow boundary.

- Evidence: `tests/playwright/screenshot_corpus/hotspot_workflow.ts:129-164`;
  `tests/playwright/screenshot_corpus/scenarios_student_types.ts:183,246-296`.
- Smallest correction: remove the traces, listener, and endpoint-specific wait. If diagnosis is
  still needed, use an explicit ignored runner under `tests/_temp/`; retain only a concise,
  durable failure message if it helps the next maintainer.

### Medium: PL/pgSQL diagnostic needs reproducible evidence

The diagnostic describes how the temporary database was prepared and reports a clean ordinary
function pass, but it omits the exact image build/run invocation, checker query, and preserved
result that would let another maintainer inspect the claimed zero-error result.

- Evidence: `docs/PLPGSQL_CHECK_DIAGNOSTIC.md:7-60,137-155`.
- Smallest correction: include copyable build/run and checker commands plus a concise raw result
  artifact or digest. Keep the experiment disposable and outside the permanent image and normal
  acceptance path.

### Low: Workstream IDs leaked into permanent SQL commentary

The Course Blueprint adoption comment uses `C842/C843`, which are temporary planning labels,
instead of only explaining the durable persistence rule.

- Evidence: `schemas/base_schema/50_functions/course_blueprint_adoption.sql:126-128`.
- Smallest correction: remove the IDs and retain the existing semantic explanation of canonical
  Question-ID persistence.

### Low: An unowned generic trigger branch remains

`assert_assigned_instructor_membership()` has a `course_instance` branch, but its only installed
constraint trigger runs on `course_membership_event`. The new diagnostic already identifies this
as the source of its remaining checker finding. This code predates the current diff, but the
cleanup explicitly surfaced it.

- Evidence: `schemas/base_schema/50_functions/course_membership.sql:64-87`;
  `docs/PLPGSQL_CHECK_DIAGNOSTIC.md:144-151`.
- Smallest correction: remove the branch, or give it a real, documented trigger owner.

## Variable-name assessment

The current patch contains good local corrections, but the foundation is not good enough.

- `published_question_id` is clearer than an unqualified JSON `question_id` at an Issued Question
  boundary. It should become the consistent lineage name, not an isolated repair.
- `course_instance_id`, `assessment_attempt_id`, and `object_record_id` are appropriately
  specific across the reviewed SQL and Rust bindings.
- `question_revision_tuple` is still too generic. It should become
  `published_question_revision_tuple` so it cannot be mistaken for a Draft Question or a future
  non-published Question revision.
- `v_created_at` avoids collision with the table column in Question Pool SQL while keeping the
  value's role clear.

The Course Banner adapter also reads SQL `object_record_id` into a local `object_id`. That may be
the correct physical Object identity in the typed storage domain, but it must be documented and
kept at that conversion boundary. SQL, JSON, and public contracts should continue to name the
database identity `object_record_id`.

## Positive observations

- The reviewed seed repair moves from a direct private-table read toward a role-authorized API
  surface, consistent with Human Guidance's normal role-gated workflows.
- The sampled Rust, SQL, and TypeScript changes preserve language-native casing and use explicit
  public-ID, object-record, and revision-tuple names.
- The revised LDA PostgreSQL integration and SQL/shell E2E tests protect meaningful persistence,
  authorization, lifecycle, and Live Demo boundaries. No new permanent test is recommended.

## Review method

Six fresh, independent passes completed: Plan, Test, Style, Documentation, Legacy, and Comment.
The screenshot-corpus definition test passed 4/4 in this snapshot; that is structural evidence
only and is not the basis for the findings above. No code was changed by this audit.
