# Question metadata fixture alignment

## Correction

`PublishedQuestionSharedMetadata` now requires `questionTitle` and
`questionDescription`. The ribbon shell harness builds this fixture from the existing seeded
Question summary so its current shared metadata matches the displayed Question data.

## Scope

This changes only the `presentationApi` fixture construction in
`tests/support/ribbon_shell_harness.tsx`. It adds no production behavior or synthetic Question
metadata. Concurrent ActivityRules fixture changes in the same file remain preserved.

## Evidence

- `npx tsc --noEmit -p tsconfig.lint.json` passed.
- `npx tsc --noEmit -p tsconfig.json` passed.
- Prettier check passed for both assigned files.
- `source ./source_me.sh && ./launchers/run_fast_checks.sh` stopped at schema style with one
  `rule_09_null_meaning` finding. Its schema-doc generation also changed `docs/SCHEMA_TABLES.md`;
  that generated diff is outside this fixture correction and remains for integration review.
