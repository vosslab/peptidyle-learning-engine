# M5 shared report/state review

## Type Safety

- ACCEPTED from source inspection: the renderer requires the closed, ordered six-row sequence
  J1, J2, J3, J4, J5, J8; validates public UUID relationships; rejects duplicate, reordered, and
  incomplete ordinary JSON records; and retains the prior exact arrangement allowlist.
- ACCEPTED from source inspection: direct renderer input uses unknown-safe parsing and
  `Reflect.ownKeys` checks, so null, hidden, Symbol, and answer-like extra fields fail rather than
  serialize. J4 has its own exact key schema; its public Mastery and Exam IDs are deliberately
  separate from J1/J2/J3/J5/J8's Mastery assignment ID.
- ACCEPTED: the complete live-input fixture now includes the required Mastery-problem UUID. All five
  focused configuration tests pass, including their independent base-URL, seed, and private-artifact
  assertions.

## Module Boundaries

- ACCEPTED from source inspection: generic `appendJourneyState` validates a mode-0700 non-symlink
  parent, mode-0600 non-symlink state file, descriptor opened with `O_NOFOLLOW`, `fstat`, 4,096-byte
  limit, ASCII canonical JSON, fixed prefix ordering, and matching public IDs before descriptor-only
  truncate/write/fsync. It enforces the next journey sequence J1, J2, J3, J4, J5, J8.
- ACCEPTED from source inspection: J8 is derived only from parsed J4 and J5 records with matching
  course and Mastery assignment IDs. Its fragment has only fixed learner/instructor visibility
  codes and public IDs. J5 requires one exact visible gradebook row by the deterministic arranged
  title and fails when the first visible page does not contain exactly one matching row. The title
  does not enter the final fragment or report.
- ACCEPTED: `readJourneyStatePrefix` now checks the mode-0700 non-symlink parent and mode-0600
  non-symlink file, opens with `O_NOFOLLOW`, uses `fstat`, applies the size/ASCII/canonical-prefix
  checks, and always closes the descriptor. J8 obtains its J4/J5 input exclusively through that
  reader before generic append. The focused Node tests cover noncanonical state, unsafe parent mode,
  and symlink replacement/outside-file preservation.
- ACCEPTED from source inspection: the fixed final renderer child now exclusively obtains the state
  through `readJourneyStatePrefix` and then requires the exact six-row parse. Its prior bare
  `lstatSync`/`readFileSync` path has been removed, so the final report shares the hardened
  mode/descriptor/canonical boundary.
- ACCEPTED: a dedicated final-renderer child subprocess regression now proves canonical six-row
  state renders J8 and that a replacement-symlink state exits nonzero with exactly empty stdout.
  Combined with the shared reader's noncanonical and unsafe-parent tests, this protects the child
  boundary and its no-leak failure behavior.
- ACCEPTED from source inspection: the runner invokes a fixed serial sequence of J1, J2, J3, J4,
  J5, then fixed J8 derivation; it does not accept a caller-provided specification. It retains
  Python final-report ownership and sensitive Playwright artifact cleanup.

## Compile-Time Errors

- PASS: Node report/state suite passed 14 tests; focused Python runner suite passed 26 tests.
- PASS: focused Playwright configuration suite passed 5 tests; J3, J4, and J5 correctly skipped
  without the explicit live invocation. No live stack was run.
- PASS: TypeScript check, ESLint, Prettier, audit ASCII, Markdown links (136 tests), and diff check.

## Type-Level Tests

- Node coverage exercises six-row ordering, hostile renderer input, J2 replacement safety,
  J8 noncanonical/unsafe-parent/symlink state rejection, and the final renderer child's canonical
  success plus replacement-symlink no-output failure behavior.
- Offline acceptance permits the planned live gate only. A live report claim still requires a later
  real-stack inspection of the exact six redacted rows and confirmation that no private artifact or
  container is retained.

## Runner Stage Diagnostics

- ACCEPTED from source inspection: fixed runner stages are concise and redacted (`launcher_check`,
  `launcher_start`, `arrangement`, `playwright_smoke`, `playwright_arranged`, `playwright_j1`
  through `playwright_j5`, `cross_actor`, and `visible_outcome_report`). Smoke, arrangement, and
  J1 are separate fixed serial child commands; no caller-controlled specification enters the runner.
  `run_required` prints only the fixed stage and exit status; it never prints child stdout or stderr.
  Failure reports retain only status, seed, stage, and already-public arrangements.
- ACCEPTED: the new cross-actor failure regression checks its exact stage, child-output redaction,
  mode-0600 report, private-state removal, and one runner-owned `down --remove-orphans` cleanup
  without volumes.
- ACCEPTED: the visible-outcome-report failure regression now checks the exact public FAIL report
  (including arrangements and its fixed stage), mode-0600 report, private-state removal, child-output
  redaction, and exactly one runner-owned no-volume `down --remove-orphans` cleanup.
- ACCEPTED: the table-driven smoke, arranged, and J1 failure regression checks each exact stage,
  mode-0600 FAIL report, private-state removal, child-output redaction, and exactly one no-volume
  `down --remove-orphans` cleanup.

Result: ACCEPTED TO RERUN. The repaired config, generic state boundary, J8 derivation, final
renderer child, exact six-row report contract, fixed serial runner, and per-child failure diagnostics
are accepted by focused offline evidence. No live claim is made.

## Live M5 Evidence

- ACCEPTED LIVE: independently inspected the manager full-run artifact
  `test-results/ui_walkthrough/ui_walkthrough_seed_42.json` before another rerun. Its containing
  directory is mode 0700 and its report is mode 0600. It is one ASCII canonical JSON line with only
  the exact top-level schemaVersion/status/masterSeed/stage/elapsedMs/arrangements/journeys keys.
- ACCEPTED LIVE: the report is complete PASS for seed 42 with the exact five allowed public
  arrangements and the ordered PASS journey sequence J1, J2, J3, J4, J5, J8. Each journey has its
  fixed visible-code vocabulary, empty diagnostics, matching public IDs, and no answer, credential,
  secret, trace, screenshot, error-context, or other prohibited key.
- ACCEPTED LIVE: `test-results/.last-run.json` records `status` passed with an empty failed-test
  list. The walkthrough directory contains only the report; runner-private temp-root patterns are
  absent and `podman ps --all` is empty.

Result: ACCEPTED LIVE. The independently inspected full-run artifact satisfies the M5 public report,
permission, redaction, and cleanup contract. This verdict is limited to the observed artifact and
cleanup state.
