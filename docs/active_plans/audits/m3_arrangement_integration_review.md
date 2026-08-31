# M3 arrangement integration review

## Scope

- Reviewer: independent offline integration review.
- Reviewed: `tests/e2e/e2e_ui_walkthrough.py`,
  `tests/e2e/ui_walkthrough_arrange.ts`, live-input/configuration modules,
  arranged-browser spec, direct-arranger spec, focused runner tests, and the
  accepted WP-A1/WP-A2 workstreams and audits.
- Initial decision: **NOT ACCEPTED OFFLINE**. The following findings were
  repaired and independently re-reviewed on the current tree.

## Findings

### Resolved P1 - The runner rejected the normal repository-local `tsx` executable

- Location: `tests/e2e/e2e_ui_walkthrough.py:475-477`.
- Evidence: the installed npm executable is the normal shim
  `node_modules/.bin/tsx -> ../tsx/dist/cli.mjs`. The current `is_symlink()`
  rejection therefore raises `fixed walkthrough arranger is unavailable`
  before the fixed child can arrange anything. The focused test creates an
  artificial regular executable, so it does not represent the actual npm
  layout.
- Impact: a valid M3 live run cannot reach the supported-API arranger. This is
  a live feasibility failure, not a cosmetic hardening detail.
- Required correction: validate a repository-owned resolved executable rather
  than requiring the npm bin shim itself to be non-symlinked. Resolve it
  without permitting escape from the repository or an arbitrary script, then
  require the resolved target to be a regular executable before invoking the
  fixed `tests/e2e/ui_walkthrough_arrange.ts`. Add a focused test using the
  normal in-repository symlink shape and a rejection case for an escaping or
  nonregular target.

### Resolved P1 - `parse_arrangement_output()` accepted noncanonical multi-line and CR output

- Location: `tests/e2e/e2e_ui_walkthrough.py:426-437`.
- Evidence: the parser checks only ASCII byte size and a final `\n`, then gives
  the whole string to `json.loads()`. Python accepts JSON whitespace around a
  document. The following all incorrectly parse as success when applied to the
  otherwise valid five-record object: a leading blank line, leading spaces,
  a trailing blank line, trailing spaces before the final newline, and a final
  CRLF. The direct adversarial probe produced `ACCEPTED` for all five cases.
- Impact: the runner does not enforce its intended fixed-child protocol of
  exactly one ASCII JSON line. Extra child output can be accepted and carried
  into the private report, weakening the redaction and bounded-output boundary.
- Required correction: require precisely one terminal LF and reject every
  other CR, LF, or leading/trailing whitespace byte before decoding. Then
  decode the one line and retain the existing exact top-level keys, five record
  order, UUID, and per-record-key checks. Extend
  `tests/test_ui_walkthrough_runner.py` with all five adversarial cases above.

### Resolved P2 - The private launcher manifest parser was not closed to extra keys

- Location: `tests/e2e/ui_walkthrough_arrange.ts:62-70`.
- Evidence: `launcherManifest()` validates that `assignmentId` is UUID-shaped
  but accepts every other JSON member. The M3 integration contract requires a
  strict bounded ASCII JSON/UUID boundary with no extra keys; the manifest is
  also the only private file that supplies the baseline reference.
- Impact: a malformed or unexpectedly expanded private manifest is silently
  accepted rather than being rejected at the arrangement boundary. The value
  currently does not reach Playwright, but this leaves the private-file
  contract weaker than the report/child boundary.
- Required correction: reject non-ASCII or oversized input, require exactly
  the sole `assignmentId` key, and add direct-parser rejection tests for an
  extra key and malformed/non-ASCII input. Preserve the current mode-0600,
  non-symlink checks and the rule that only the public assignment ID is
  returned.

## Confirmed behavior

- The runner invokes one fixed TypeScript child and fixed smoke/arranged specs;
  it does not accept a caller-selected script or Playwright spec.
- The child uses isolated instructor API contexts for WP-A1 and WP-A2 and
  disposes them on its normal and failure paths. Its returned arrangement
  object carries only five separately labelled public arrangement records.
- The report retains only status, master seed, stage, and validated public
  arrangement IDs; raw child stdout/stderr, API bodies, cookies, credentials,
  private source, and manifest contents are not reported. Later failure still
  follows the existing private atomic-report recreation and no-volume cleanup
  path.
- The visible spec uses the rendered local credential form, then visible course
  and assignment links. It does not use API login, `addCookies`, storage state,
  or a direct assignment-route shortcut. UI strings and routes match
  `src/pages/course_list_page.tsx` and `src/pages/course_assignments_page.tsx`.
- AUTO/`--build`/`--skip-build`, IPv4-only `127.0.0.1` live origin, fixed
  Compose ownership, and cleanup behavior remain intact in the reviewed code.

## Offline validation

- PASS: `source source_me.sh && python3 -m pytest tests/test_ui_walkthrough_runner.py -q`
  - 20 passed. This baseline does not cover the P1 whitespace variants or
    actual npm `tsx` symlink layout.
- PASS: `npx playwright test tests/playwright/ui_walkthrough_live_config.spec.ts tests/playwright/simulator/ui_walkthrough_arrange.spec.ts --reporter=line`
  - 7 passed.
- PASS: `npx tsc --noEmit`, ESLint on the M3 TypeScript files, and
  `git diff --check`.
- PASS: focused source-size and ASCII pytest gates - 1,638 passed.
- NOT APPLICABLE: Prettier has no Python parser, so Python source was not sent
  to Prettier; TypeScript formatting is covered by its normal Prettier check.
- NOT RUN: live Podman/Chromium arrangement, as required for this offline
  review.

## Required live acceptance after repair

Run twice from a clean selected Compose project with the same seed:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

For each run, verify the fixed child succeeds, the student visibly signs in and
opens both `Peptide mastery retry` and `Peptide exam contrast`, the private
report is mode 0600 in a mode-0700 directory with exactly five arrangement
records, no mock server is used, and selected containers are absent after
cleanup. The two reports must have the same arrangement labels and same seed;
new API-created UUIDs need not be identical across clean runs.

## Repair re-review

### Fixed-child executable boundary

- PASS: The runner now resolves the exact repository-owned
  `node_modules/tsx/dist/cli.mjs` target and invokes it through an executable
  `node` binary. It separately requires the normal
  `node_modules/.bin/tsx -> ../tsx/dist/cli.mjs` npm shim to resolve to that
  exact target. This preserves a fixed child while supporting the actual npm
  installation shape.
- PASS: The direct normal-shim test now creates the same in-repository npm
  symlink layout and reaches the parsed-output handoff. The escape test points
  the shim at a different in-repository file and is rejected. The actual
  repository symlink resolves to `node_modules/tsx/dist/cli.mjs`, and
  `node node_modules/tsx/dist/cli.mjs --version` succeeds.

### Canonical output and private-file boundaries

- PASS: `parse_arrangement_output()` now requires ASCII output of at most 2048
  bytes, exactly one terminal LF, no CR, and byte-for-byte equality to compact
  canonical JSON plus that LF. It retains exact top-level keys, five fixed
  ordered records, exact per-record keys, and UUID validation.
- PASS: The focused runner tests reject leading blank output, trailing blank
  output, CRLF, trailing whitespace, and JSON formatting whitespace. An
  independent direct probe additionally rejected leading spaces; the canonical
  one-line output was accepted.
- PASS: private files remain regular, non-symlink, mode-0600 files. Reads are
  bounded to 4096 bytes and reject non-ASCII bytes. The launcher manifest now
  requires exactly its current four UUID keys (`assignmentId`, `enrollmentId`,
  `problemId`, `versionId`) and returns only the baseline assignment ID. The
  current launcher-generated manifest has exactly that shape; it is never put
  in the Playwright environment.

### Regression boundary

- PASS: only the fixed arranger and fixed smoke/arranged specs are invoked;
  no caller-selected script or spec exists. The report remains private and
  redacted, retains validated arrangements on later failure, and uses atomic
  recreation after Playwright artifact cleanup. The visible browser flow still
  signs in through the rendered student form and selects the visible course,
  Mastery, and Exam links; it uses no API, cookie, storage-state, or direct
  assignment-route shortcut.
- PASS: AUTO/`--build`/`--skip-build`, IPv4-only local origin, mock disabling,
  exact Compose-project ownership, and no-volume cleanup are unchanged.

## Final offline verdict

**ACCEPTED OFFLINE.** This acceptance is limited to static and focused-test
evidence. It makes no live M3 claim. Live acceptance remains the two clean,
same-seed invocations and evidence checklist above.

## Selector-fix re-review

### Visible selection contract

- PASS: `tests/playwright/ui_walkthrough_arranged.spec.ts` now selects the
  Mastery card only when it contains the exact visible public href
  `/courses/${courseId}/assignments/${masteryAssignmentId}`, and likewise
  selects the Exam card with its exact current public course-and-assignment
  href. The subsequent action still clicks the rendered `Review assignment`
  link inside that selected card.
- PASS: title assertions remain confirmation of the visible outcome, rather
  than selection authority. This eliminates the discovered duplicate-title
  ambiguity when retained volumes contain an older arrangement. The flow still
  reaches the course through the visible `Open course` link and does not use a
  direct route, API arrangement/login, cookies, or storage state.
- PASS: current `src/pages/course_assignments_page.tsx` renders exactly this
  course-scoped assignment href shape and `Review assignment` label, so the
  fixed selector matches the public UI contract.

### Failed-replay evidence and limits

- PASS: the M3 workstream records that the prior retained-volume replay wrote
  five safe arrangement records, failed at the duplicate-title selector, and
  left zero selected containers after no-volume cleanup. The runner's report
  writer is still constrained to status, seed, stage, and already validated
  public arrangement IDs; it does not retain raw child stdout/stderr, API body,
  credential, cookie, private source, or manifest content.
- LIMIT: the ignored prior report/Playwright artifact is no longer present in
  `test-results/`, because later focused Playwright runs clear that directory.
  I therefore verified the retained evidence record and the current redaction
  implementation, not a preserved raw failed-replay file. The required fresh
  reruns below must inspect each newly created report before any subsequent
  Playwright command clears it.

### Re-validation

- PASS: focused runner and Markdown-link pytest gates - 157 passed.
- PASS: focused live-config/direct-arranger Playwright gates - 7 passed.
- PASS: strict TypeScript check, ESLint, TypeScript Prettier, and
  `git diff --check`.
- PASS: `podman ps --all --quiet` is empty before fresh reruns.

## Verdict for fresh live reruns

**ACCEPTED TO RERUN LIVE.** This is not a replacement live acceptance. Run the
two exact same-seed commands in the prior checklist from a clean selected
Compose project. After each command, inspect the report before running another
Playwright command: it must be private (directory 0700, file 0600), contain
only the allowed redacted fields and five arrangement records, show the
expected result, and leave `podman ps --all --quiet` empty. Confirm in the
browser that the student uses the rendered form, opens the visible course, and
opens the exact current Mastery and Exam cards selected by their hrefs.

## Final live re-review

### Consecutive replay evidence

- PASS: two consecutive corrected runs supplied for independent review both
  exited 0 with master seed 42. Their new arrangement IDs differed as expected:
  first problem/Mastery/Exam IDs were
  `019fef80-3da6-7452-9cdc-b517f795e62b`,
  `019fef80-3db9-79a1-89e7-7fbd8451f6a2`, and
  `019fef80-3dc2-7611-951c-ba4b777ec473`; second IDs were
  `019fef81-b551-7d60-9b30-1586bb5ef7a8`,
  `019fef81-b565-7532-8d2b-074c4d17e881`, and
  `019fef81-b56f-7651-ba65-fb0032339eca`. This is correct replay behavior:
  stable seed and labels, not an incorrect requirement to reuse API-created
  UUIDs.
- PASS: I inspected the retained second report. It was a 0700 directory with a
  0600 report file, and contained exactly `PASS`, seed 42, `complete`, and the
  five fixed public arrangement records. It contained no credential, cookie,
  secret, answer, source, raw stdout, or raw stderr field. `.last-run.json`
  reported `passed` with no failed tests, and the selected Podman project was
  empty after the run.

### Independent corrected run

- PASS: I ran the exact elevated command:

  ```bash
  bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
  ```

  The live browser phase completed and produced a fresh report with status
  `PASS`, seed 42, stage `complete`, and five records. Its fresh public IDs
  include problem `019fef83-57d3-7840-9b75-75ec3974cefb`, Mastery
  `019fef83-57e7-7ad3-8e44-31e2f4393367`, and Exam
  `019fef83-57f0-7450-8d86-50a2d859b28c`. The report directory/file modes were
  again 0700/0600, `.last-run.json` again reported passed/no failures, its
  redaction scan found none of the prohibited classes, and final
  `podman ps --all --quiet` was empty.
- NOTE: the execution harness surfaced the running containers before its
  completion notification had reliably delivered the runner's final output. I
  issued a no-volume `down --remove-orphans` for only that exact runner-created
  `containers` project while checking cleanup; it raced with normal shutdown
  and printed already-removed-resource messages. The final report and clean
  project state are verified, but the two preceding clean consecutive runs are
  the unambiguous automatic-cleanup evidence.

## Final verdict

**ACCEPTED.** M3 now has two clean same-seed live replay passes, an independent
fresh live PASS report, visible current-ID card selection rather than title
ambiguity, private/redacted evidence, and final empty-container state. This
accepts M3 arrangement evidence only; it does not claim any later keyboard
journey, canonical onboarding, all-family coverage, or release gate.

## UUID-derived title re-review

### Finding

### P1 - The changed supported-API title contract has a stale focused expectation

- Location: `tests/playwright/simulator/assignment_arrangement.spec.ts:129-160`.
- Evidence: `assignment_arrangement.ts` now correctly derives both visible
  assignment titles from the validated public problem UUID, and the arranged
  browser spec uses the same helpers while selecting exact current public
  hrefs. However, the focused contract test still expects the old bare titles.
  Its actual failure shows the correct received values:
  `Peptide mastery retry 123e4567-e89b-12d3-a456-426614174002` and
  `Peptide exam contrast 123e4567-e89b-12d3-a456-426614174002`.
- Impact: the changed arrangement request contract is not fully validated;
  the required focused suite is red, so another live rerun must not be claimed
  yet.
- Required correction: update the exact expected request titles to use the
  same title helpers (or their exact UUID-derived output) and add/retain a
  direct deterministic helper test for valid and invalid UUID input.

### Confirmed design boundary

- PASS: Mastery and Exam cards are selected by exact current visible hrefs
  containing validated public course and assignment IDs. The shared public
  corpus problem UUID derives their expected headings; it is handed through the
  strict runner record parser and live input validator, not read from a private
  file by the browser spec.
- PASS: the student still visits `/`, fills the rendered local credential form,
  clicks `Sign in locally`, opens the rendered course card, then clicks the
  rendered `Review assignment` links. No API login, private manifest/source,
  cookie injection, storage state, or direct-route shortcut was added.

### Re-validation

- FAIL: `npx playwright test tests/playwright/simulator/assignment_arrangement.spec.ts tests/playwright/simulator/ui_walkthrough_arrange.spec.ts tests/playwright/ui_walkthrough_live_config.spec.ts --reporter=line`
  - 19 passed, 1 failed at the stale title expectation above.
- PASS: strict TypeScript check, ESLint, Prettier, focused Python runner tests,
  and `git diff --check`.

## Current verdict

**NOT ACCEPTED TO RERUN LIVE** until the focused arrangement test is corrected
and passes. The browser selector design itself is sound, but a red owning
contract suite blocks another live claim.

## UUID-title repair re-review

- PASS: `assignment_arrangement.spec.ts` now imports the same
  `masteryRetryTitle()` and `examContrastTitle()` helpers as the supported-API
  arranger. Its exact request-body expectation therefore follows the sole
  title owner rather than retaining a stale bare string.
- PASS: the same focused spec directly asserts both complete strings from the
  full public problem UUID and their 58-character bound. This covers the
  visible-heading contract while retaining UUID validation in the helper.
- PASS: the combined focused suite reports 21 passed with the live arranged
  spec honestly skipped outside explicit live mode. Strict TypeScript, ESLint,
  Prettier, focused Python runner checks, and `git diff --check` also pass.

## Final rerun verdict

**ACCEPTED TO RERUN LIVE.** The stale-title blocker is resolved. A live rerun
may now verify the unchanged visible local-login flow, exact current href card
selection, and the shared UUID-derived headings; this offline decision itself
makes no new live claim.
