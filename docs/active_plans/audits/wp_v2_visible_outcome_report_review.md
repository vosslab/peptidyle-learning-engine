# WP-V2 visible-outcome report review

## Supersession addendum

This review remains accepted evidence for its bounded schema-v1 J1 report. It
is not the active acceptance baseline after the 2026-08-11 corrected local
no-email pilot passed the schema-v2 J11/J12/J13/J1/J2/J3/J4/J5/J8 sequence.
That pilot does not accept email/canonical onboarding, J6/J7, all-family,
multi-student, or release work.

## Type Safety

- ACCEPTED after re-review: `renderVisibleOutcomeReport` now owns the exact five arrangement
  labels and their exact public-ID key schemas. It rejects arbitrary answer-like labels or keys,
  duplicate labels, missing labels, and all out-of-schema arrangements rather than relying on the
  fixed child alone. The direct hostile probes rejected a `correctChoice` key, `correct-answer`
  label, missing row, duplicate label, and uppercase UUID spelling.
- ACCEPTED: UUID validation is now lowercase canonical ASCII; output cannot vary between equivalent
  uppercase and lowercase UUID inputs.
- ACCEPTED: J1 input has an exact key set; only PASS and FAIL are available at this
  milestone; PASS requires precisely the five J1 visible milestones with no diagnostic; FAIL
  requires one fixed redacted diagnostic and incomplete PASS evidence. Master seed and elapsed
  time are bounded integer values.

## Module Boundaries

- ACCEPTED: `ui_walkthrough_report.ts` is a fixed child with strict ASCII, single-line canonical
  JSON input; a non-symlink mode-0600 state file; exact arrangement labels and keys; bounded
  output; and no credential or answer-bearing input. It keeps the Python runner as the only final
  report writer.
- ACCEPTED: the final report preserves the prior top-level `status`, `masterSeed`, and
  `stage: "complete"` semantics. Arrangement and journey evidence remain separate; the current
  report does not reconstruct a score or call arrangement browser coverage.
- ACCEPTED: the runner passes only canonical, parsed arrangement IDs to the fixed renderer.
  `PLAYWRIGHT_NO_COPY_PROMPT=1` is added only to the Playwright child environment. The launcher,
  arranger, renderer, and report writer use fixed argv/configuration paths rather than caller
  supplied child commands.
- ACCEPTED: mode-0700 system-temporary J1 state is outside `test-results`; the state file is
  created mode 0600. Report writes use a directory descriptor, exclusive temporary file, atomic
  replacement, mode 0600, and symlink checks. The runner recreates the report directory after
  Playwright artifact cleanup and removes only its exact temporary state root. Cleanup failure
  downgrades an otherwise successful run to FAIL and nonzero status.

## Compile-Time Errors

- PASS (offline): `source source_me.sh && python3 -m pytest -q tests/test_ui_walkthrough_runner.py`
  returned `25 passed in 0.06s`.
- PASS (offline): `node --import tsx --test tests/test_visible_outcome_report.mjs` returned six
  passing tests, including direct-renderer rejection and no-pointer J1 source checks.
- PASS (offline): `npx tsc --noEmit`, focused ESLint, TypeScript Prettier, Python `py_compile`,
  Pyflakes, ASCII scans, Markdown-link suite (`136 passed`), and `git diff --check` completed
  without a relevant diagnostic. Prettier does not parse Python files, so it was run only on the
  TypeScript files; Python formatting was not claimed.
- PASS (independent artifact inspection after the manager's exact successful live M4 run):
  `test-results/ui_walkthrough/ui_walkthrough_seed_42.json` is a 1,065-byte, ASCII, single-line,
  canonical JSON record. Its parent is mode 0700 and the report is a regular mode-0600 file. It
  has exactly the expected seven top-level keys, `PASS`, seed `42`, `stage: "complete"`, the five
  ordered arrangement rows with exact allowed keys and lowercase UUID values, and one J1 PASS row
  with the five canonical visible codes and no diagnostics. `.last-run.json` records `passed` with
  no failed tests.
- PASS (final cleanup recheck): after a separate concurrent HCI run completed, `podman ps --all
--quiet` was empty. No `ple-ui-walkthrough-*` temporary directory and no trace, screenshot, or
  error-context artifact remained under the inspected output locations. The brief nonempty project
  state observed during this review belonged to that concurrent run, not to the completed M4 run.

## Type-Level Tests

- ACCEPTED: direct-renderer tests now reject answer-like keys, uppercase IDs, and duplicate
  arrangements. The exact five-label schema also rejects missing labels and arbitrary empty maps;
  the only empty public-ID map is the launcher-seeded enrollment row. The legitimate shared
  `courseId` between Mastery and Exam remains accepted; no incorrect broad UUID-uniqueness rule
  was introduced.
- Existing tests retain canonical arrangement sorting, answer text added to a journey fragment,
  J1 PASS/FAIL semantics, report compatibility, cleanup downgrades, symlink replacement, and
  Playwright artifact-directory recreation.
- The completed live inspection satisfied the required report checks: exact schema and public-only
  vocabulary, modes, five arrangement rows, J1 milestones, passed Playwright summary, private-state
  removal, artifact absence, and final empty Podman project state. The existing offline cleanup
  regressions continue to prove that a cleanup failure produces a mode-0600 FAIL report and
  nonzero status.

Result: ACCEPTED. WP-V2 has offline boundary acceptance and independently inspected live M4 report
and cleanup evidence. This review does not expand coverage beyond J1.
