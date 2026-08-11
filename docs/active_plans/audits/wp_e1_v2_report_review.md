# WP-E1 schema-v2 report groundwork review

## Result

**ACCEPTED OFFLINE.** The isolated schema-v2 parser, renderer, child, and
prospective fixture satisfy the reviewed closed-contract, redaction, and
repository hygiene requirements. This is deliberately not final walkthrough
acceptance: the runner does not yet invoke the v2 child, and J5/J8 integration
and retained-stack evidence remain owned by WP-E1.

## Evidence reviewed

- `v2_visible_outcome_report.ts` accepts exactly nine `PASS` fragments in the
  fixed order J11, J12, J13, J1, J2, J3, J4, J5, J8. Each fragment has an exact
  key set and exact closed milestone list. J13 supplies the canonical public
  assignment ID; every later assignment-bearing fragment must match it, and all
  nine course IDs must match J13's course ID.
- The renderer accepts only an exact array or an exact `{ fragments }` wrapper,
  rejects inherited, hidden, symbol, accessor, score-bearing, reordered, and
  cross-bound input, restricts the seed to unsigned 32-bit range, and projects
  only journey, PASS, bounded elapsed time, closed codes, and empty diagnostics.
  It emits the one label-only `api-retry-corpus-publication` arrangement. No
  public identifier, title, learner, score, run detail, email, problem ID, or
  version ID survives rendering.
- The reader requires canonical single-line ASCII JSON no larger than 4096
  bytes. Canonical re-rendering rejects duplicate JSON members at every depth.
  Descriptor checks require a non-symlink 0700 parent and 0600 regular
  `journeys.json`; the parent inode/device is rechecked after opening the child.
  Hostile tests cover mode changes, symlink, non-ASCII, CR/multiline,
  noncanonical and duplicate JSON, parent replacement, and a failing child with
  empty stdout.
- The prospective fixture has explicit
  `recordType: "prospective-walked-journey-fixture"`; its duplicate-safe Python
  loader requires only the nine ordered PASS rows and the one corpus label. It
  has no path into the current runner or accepted static baseline.
- The existing schema-v1 state/report modules were not edited by this proposed
  set. The runner still terminates before the full WP-E1 path, so this review
  makes no live or final report claim.

## Focused gates

- PASS: `npx playwright test tests/playwright/simulator/v2_visible_outcome_report.spec.ts`
  - `7 passed (397ms)`.
- PASS: `python3 -m pytest -q tests/test_walked_journey_baseline_v2.py`
  - `8 passed in 0.01s`.
- PASS: `npx tsc --noEmit` and focused ESLint completed with no diagnostics.
- PASS: `npx prettier --check tests/playwright/simulator/v2_visible_outcome_report.ts tests/playwright/simulator/v2_visible_outcome_report.spec.ts tests/e2e/ui_walkthrough_v2_report.ts`
  - `All matched files use Prettier code style!`.
- PASS: focused `git diff --check` produced no diagnostic.

## Boundary retained for promotion

`wp_e1_corrected_v2_report.md` now expressly marks this work **NOT ACCEPTED
LIVE** and the fixture as prospective. Only a later integrated two-run
retained-stack can accept the final schema-v2 report. The historical
`wp_v2_visible_outcome_report` material remains schema-v1/J1 evidence and
cannot be read as acceptance of this schema-v2 fixture.
