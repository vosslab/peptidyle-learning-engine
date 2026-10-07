# M04 Native adapter and exact-title report

Status: adapter implementation and focused checks are complete. Exact SQL behavior, integrated
server compilation, and milestone acceptance remain pending the coordinated fresh-database run and
core/producer handoff.

## Implementation

The Native JSON document no longer accepts or compiles `questionTitle`, `questionDescription`,
`tags`, `questionLicense`, `questionCitation`, or `language`. Its compiled presentation contains
content and response behavior only; it exposes neither `metadata()` nor `question_title()`. The
trusted import constructor likewise accepts only prompt, choices, and the correct choice. No
language value is created by the adapter.

`preview_question_json` and `issue_question_json` now receive the record title as an explicit
argument. `NativePleIssuanceSource` and the Native variant of `InstructorStudentViewSource` carry
that title from an exact Question Revision read. The Assessment issuance, Instructor Student View,
and completed-history source projections join `question_revision_metadata` on both Question ID and
Revision number, and their Rust decoders use the `question_title` output field. The generic Backend
trait is unchanged.

## Checks

- `cargo test -p adapter_ple`: 20 unit tests and 6 compile-fail documentation tests passed.
- `cargo check -p learning-data-access`: passed with existing unrelated dead-code warnings.
- The coordinated `cargo check -p question_model -p learning-data-access -p server_core -p project-tools` passed, including the exact-title server callsites and content-only import constructor.
- `cargo fmt --package adapter_ple`, scoped Rust formatting, and `git diff --check` passed.
- A focused adapter test verifies that all six record-owned fields are absent from the Native
  fixture and rejected when supplied with valid JSON value shapes. The adapter source test also
  verifies the supplied title in both preview and issued presentations.

## Remaining evidence

The focused adapter tests and coordinated crate check establish source parsing, title injection,
and Rust projection/decoder type agreement. They do not exercise the SQL output records against
PostgreSQL. The exact metadata joins still need fresh-database verification. No full-stack build,
schema generation, TypeScript generation, or database runtime was run in this lane.
