# Final question-spec evidence alignment

## Authority and status

[Human Guidance](../../../HUMAN_GUIDANCE.md) and the approved
[Question-spec implementation plan](../../active/question_spec_implementation_plan.md) define
intended behavior. The Question specifications describe that model; current source and tests provide
implementation evidence.

All 29 source milestones are implemented or in final bounded corrections. The M15 WebWork exact-save
acknowledgement and M16 correction-publication metadata carry-forward fixes remain active, with
public interfaces stable. This documentation and source review does not claim connected database,
browser, screenshot, or integrated M29 acceptance. Those checks remain pending the central ledger.

## Corrected implementation statements

### Native JSON preview and testing

[NATIVE_JSON_SPEC.md](../../../QUESTION_SPECS/NATIVE_JSON_SPEC.md) now says the Native Draft preview
parses and compiles source before producing an answer-free presentation. Draft testing separately
evaluates a transient response. It removes the claims that preview skips compilation and raw Draft
saves remain pending. It also removes the claims that serialized source still duplicates Question
metadata and that the decoder requires record metadata fields.

Evidence: [draft_preview.rs](../../../../crates/server/src/draft_preview.rs),
[preview tests](../../../../crates/server/src/draft_preview/tests.rs),
[Native adapter tests](../../../../crates/adapters/ple/src/question_json/tests.rs), and
[WebWork preview tests](../../../../crates/adapters/webwork/src/lib/draft_preview_tests.rs).

### Draft creation and imports

[QUESTION_IMPORT_SPEC.md](../../../QUESTION_SPECS/QUESTION_IMPORT_SPEC.md) now says the ordinary
Draft API and browser creation page support Native JSON, WebWork PG, and PGML. The converter
handoff remains deferred; authoring support does not establish it.

Evidence: [create.rs](../../../../crates/server/src/authoring/create.rs),
[question_drafts_page.tsx](../../../../src/pages/question_drafts_page.tsx),
[creation client test](../../../../tests/test_question_draft_creation.mjs), and
[Draft source client test](../../../../tests/test_draft_source_client.mjs).

### Bloom metadata

[QUESTION_BLOOM_CLASSIFICATION_SPEC.md](../../../QUESTION_SPECS/QUESTION_BLOOM_CLASSIFICATION_SPEC.md)
now says Bloom uses ordinary Question and Pool metadata saves. Both dimensions may be NULL; a full
Question metadata replacement sends both values and can preserve the unchanged one.

Evidence: [question_metadata.rs](../../../../crates/server/src/question_metadata.rs),
[Question metadata editor](../../../../src/features/question_metadata/question_metadata_editor.tsx),
[Pool metadata editor](../../../../src/components/question_pool_metadata_editor.tsx),
[Question metadata test](../../../../tests/test_question_metadata_client.mjs), and
[Pool metadata test](../../../../tests/test_question_pool_metadata_client.mjs).

### Optional Question language

[QUESTION_LIBRARY_METADATA_SPEC.md](../../../QUESTION_SPECS/QUESTION_LIBRARY_METADATA_SPEC.md)
now says `language` is nullable in the current schema and model; its length check applies when a
value is present.

Evidence:
[published_question.sql](../../../../schemas/base_schema/20_tables/published_question.sql),
[question_content.rs](../../../../crates/question_model/src/question_content.rs), and
[authoring tests](../../../../crates/server/src/authoring_tests.rs).

## Settled behavior retained

These corrections describe current implementation only. They do not alter the settled model: direct
Pool references and explicit reusable forks; complete Question Revision records and exact Revision
Tuples; nullable Bloom dimensions; content-only Native JSON; ordinary archive, restore, and fork
behavior; stored credit fractions with current Assessment point settings; or delivery and credit
statistics based on actual deliveries and outcomes.

Deferred behavior remains deferred: AI assignment, automatic WebWork Type detection, Instructor bulk
editing, optional feedback timing, NC/ND licenses, abandoned-Draft cleanup, converter handoff,
answer-key regrading, and the tentative initial Library search filter.

## Evidence and acceptance boundary

The cited tests were inspected as source evidence and were not run in this documentation lane.
Static source and document consistency do not establish connected database or browser acceptance.
No source, test, API-contract, schema-table, central ledger, TODO, changelog, or plan files were
changed here.

## Reviews and checks

- Fresh read-only Luna SPEC review completed with no concrete findings.
- Separate fresh read-only Luna QUALITY review completed with no concrete findings.
- ASCII compliance passed for all seven assigned Question-spec files and this report.
- Local Markdown link-target check passed for all eight files; local targets exist within the repo.
- `git diff --check -- docs/QUESTION_SPECS` passed; the report trailing-whitespace check passed.
- Implementation tests were inspected but not run. Builds, containers, browser checks, and connected
  runtime acceptance were not run; integrated M29 acceptance remains pending the central ledger.
