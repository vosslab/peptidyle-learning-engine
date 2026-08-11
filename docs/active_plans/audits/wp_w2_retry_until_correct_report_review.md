# WP-W2 retry report review

## Type Safety

- ACCEPTED after re-review: the exported renderer now accepts `unknown` collections and is total
  for hostile records: null/non-record arrangements return `undefined`, not an exception.
- ACCEPTED: `Reflect.ownKeys` plus enumerable data-property checks now reject Symbol and
  non-enumerable extra fields on J1/J2 fragments, arrangements, and `publicIds`. The direct suite
  verifies null, hidden, Symbol, answer-like, and uppercase hostile inputs without a throw.
- ACCEPTED otherwise: ordinary JSON fragments require exactly ordered J1 then J2, have matching
  lowercase public course/assignment UUIDs, reject duplicate/missing/extra rows, and use separate
  allowlisted milestone vocabularies. J2 adds only `visible_retry`; it carries no answer, grading,
  score, selector, credential, or feedback-body field.

## Module Boundaries

- ACCEPTED after re-review: J2's append path now checks the mode-0700 non-symlink parent and the
  named mode-0600 non-symlink file, opens the final state component with `O_NOFOLLOW`, validates
  the opened descriptor using `fstat`, caps it at 4,096 bytes, requires ASCII canonical one-J1
  JSON, validates matching public IDs, then truncates and writes only through that descriptor.
  It restores mode 0600 and fsyncs before close. The symlink-replacement regression confirms that
  an outside file is neither read as state nor overwritten.
- ACCEPTED: Python remains the final mode-0600 atomic report writer and creates the state directory
  mode 0700 outside `test-results`; normal finalization removes only the exact state root and a
  cleanup failure downgrades PASS to FAIL/nonzero. The runner invokes a fixed J1 lane then a fixed
  J2 spec, and only the sensitive Playwright child receives `PLAYWRIGHT_NO_COPY_PROMPT=1`.
- ACCEPTED: the fixed TypeScript report child bounds the aggregate state input, requires ASCII canonical
  one-line JSON, validates the two-fragment array before rendering, and does not write the final
  retained report.
- ACCEPTED after artifact-failure repair: explicit live configuration revalidates an absolute,
  non-symlink mode-0600 state file under a non-symlink mode-0700 parent before Chromium can start.
  It derives `journey-artifacts` only as that parent sibling; no environment setting can override
  or traverse it. Walkthrough Playwright uses that private output directory, while normal offline
  tests retain `test-results`. Python removes the exact private parent, including any error context,
  trace, screenshot, or video, on every finalization path. `.last-run.json` is not evidence.

## Compile-Time Errors

- PASS (offline): Node report tests: 11 passed. Focused Python runner tests: 26 passed. Focused
  config tests passed 5; live J1/J2 Playwright specs were honestly skipped outside explicit live
  mode (2 skipped).
- PASS (offline): `npx tsc --noEmit`, focused ESLint, TypeScript Prettier, Python `py_compile`,
  Pyflakes, ASCII scans, Markdown-link suite (`136 passed`), and `git diff --check` completed
  without a diagnostic. Prettier was applied only to TypeScript, which it parses.
- PASS (independent forced-build artifact inspection): the stable seed-42 report is a 1,386-byte
  ASCII, single-line canonical JSON file, mode 0600 under a mode-0700 report directory. It has the
  exact top-level schema; exact five allowed arrangement rows with only lowercase UUID public IDs;
  ordered J1 then J2 PASS fragments with matching course/assignment IDs; exact J1/J2 milestone
  vocabularies; no diagnostics; and a top-level duration equal to the fragment-duration sum.
- PASS (cleanup inspection): no selected Podman container, private walkthrough temporary directory,
  trace, screenshot, video, or error-context artifact remained. `.last-run.json` was not used as
  walkthrough evidence.

## Type-Level Tests

- Existing tests pin ordinary JSON ordering, J1/J2 exact count/order, matching IDs, answer-like
  enumerable fields, uppercase UUIDs, keyboard-source policy, null/non-record arrangements, hidden
  and Symbol keys, and the J2 state append's symlink, mode, parent, size, ASCII, and canonical
  input failures. The legitimate shared course ID remains allowed.
- The artifact tests pin state-path metadata validation before browser creation, a derived sibling
  output directory, ignored artifact-directory environment input, offline `test-results` behavior,
  and private artifact deletion with the state root.
- The completed forced-build live inspection satisfied the required report and cleanup checks. It
  establishes only the visible J1/J2 retry contract; it does not claim later journeys, score
  reconstruction, or answer-key knowledge.

Result: ACCEPTED. WP-W2 has independently inspected forced-build live report and cleanup evidence.
