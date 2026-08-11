# WP-V3 advisory failure triage

## Status

**ACCEPTED OFFLINE.** The independent
[WP-V3 review](../audits/wp_v3_failure_triage_review.md) accepts this pure helper. Report
integration and M5 live evidence remain pending. The helper does not modify the accepted J1
renderer, Python runner, journey state, or report outcome.

## Scope

- `tests/playwright/simulator/failure_triage.ts` owns a closed stage and diagnostic vocabulary.
- It maps only exact stage-diagnostic pairs to `configuration`, `gateway`, `selector`, `keyboard`,
  or `visible-outcome-mismatch`; all other values yield `unclassified`.
- Its result contains only `{ category }`. It cannot retain or emit raw errors, selectors, URLs,
  credentials, answer text, feedback text, or scores.
- The helper is advisory. It cannot synthesize PASS or change PASS, BLOCKED, NOT_APPLICABLE, or
  FAIL evidence.

## Evidence

- Table-driven Node tests cover every fixed category, mismatched fixed inputs, unbounded input,
  an extra selector field, non-enumerable or symbol raw-detail properties, and hidden required
  fields.
- The accepted offline review records passing strict TypeScript, focused ESLint, Prettier, ASCII,
  Markdown-link, and whitespace checks. Integration and M5 live evidence are not claimed.

## Handoff

Integrate only after the report contract expands beyond the accepted J1 renderer. The future
consumer must pass one already-redacted stage-diagnostic pair, retain the existing journey and
report outcome unchanged, and publish only this helper's category. Re-review the report boundary
when that consumer is added.
