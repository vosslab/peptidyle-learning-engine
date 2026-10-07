# M04 and M17 generated TypeScript contracts

`source ./source_me.sh && cargo tsgen` completed and wrote 414 contract types to `generated/api` from the canonical Rust contract roots.

The generated `QuestionMetadata` now declares `language: string | null`, matching the optional supplied language in the canonical model. The generated `AssessmentActivityRules` now includes `partialCreditEnabled: boolean`, matching the default-on partial-credit policy. These are the only contract fields this lane verified.

Focused TypeScript checks:

- `npx tsc --noEmit -p tsconfig.json`: passed.
- `npx tsc --noEmit -p tsconfig.lint.json`: failed at `tests/support/ribbon_shell_harness.tsx:610`. Its `PublishedQuestionSharedMetadata` fixture omits required `questionTitle` and `questionDescription` fields. The exact error was reported to the coordinator for routing to the owning test-support lane.

This is a generated-contract milestone only. It does not establish M04 or M17 integrated acceptance, runtime behavior, or dependent feature acceptance.
