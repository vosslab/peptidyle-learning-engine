# WP-G1 harness independence

## Charter addendum

The accepted schema-v1 scanner remains useful historical protection. The
corrected walkthrough excludes J9/J10 and canonical email onboarding entirely;
schema-v2 follow-up removes email-specific journey rules while retaining the
general prohibition on hidden identity, API, cookie, storage, answer, score,
and pointer shortcuts.

## Status

**Independently ACCEPTED.** This is the fast static simulator-boundary gate,
not live journey, scoring, onboarding, or final M7 acceptance. See the
[WP-G1 independent review](../audits/wp_g1_harness_independence_independent_review.md).

## Scope

This workstream adds a fast, static pytest gate for the simulator boundary. It
inspects only the owned simulator, UI-walkthrough runner, arranger/reporter,
live-config, and keyboard-journey sources; it does not inspect or alter
product code.

## Contract enforced

- Harness sources cannot import product internals, Rust crates, or generated
  private data, and cannot contain SQL, `psql`, or database-shaped setup.
- Keyboard journeys cannot use Playwright request/state/cookie/route
  shortcuts, aliases, `fetch`, private endpoints, non-focus `evaluate`,
  answer-bearing or body-text assertions, or a caught/promise failure converted
  to `PASS`.
- J1 through J5 cannot use pointer actions, direct focus, browser history,
  synthetic selection, direct non-root navigation, or any key press other than
  literal Tab, Shift+Tab, Space, and Enter in the platform path. The shared
  Tab helper emits the same literal Tab pair, and its `evaluate` reads the
  rendered active element exactly. After those narrow allowlists, every
  remaining `.goto`, `.click`, `.press`,
  `.focus`, `.type`, `$eval`, `$$eval`, `evaluate`, or synthetic-selection
  member is rejected, including member alias extraction.
- Future J9/J10 files cannot reuse a local development credential, local
  sign-in form, local-login file, or local session as a canonical-account
  fallback, including normalized or zero-padded J9/J10 filenames.

Dynamic imports and SQL CTE/PRAGMA/database-client forms are rejected along
with direct SQL. This keeps a renamed helper or a different database dialect
from evading the boundary.

The scanner intentionally permits the declaration-time live-mode skip and the
accepted J1-J5 root navigation plus rendered local credential form. It does
not count files or assert implementation names, so adding legitimate simulator
sources remains a behavior review rather than an exact-count update.

## Evidence

`tests/test_ui_walkthrough_harness_independence.py` exposes pure source
scanners and exercises hostile snippets for product/crate/generated imports,
dynamic imports, SQL/CTE/PRAGMA/database clients, private browser/API state,
non-root navigation, pointer/synthetic selection/non-platform keyboard
actions, residual member aliases, non-focus evaluation, body-text assertions
including an answer literal, multiline answer-bearing assertions, catch/promise
hidden pass conversion, and normalized J9/J10 local-identity fallback. Its
repository scan is fast and offline.

Focused validation is recorded with this workstream after the manager runs:

```bash
source source_me.sh && python -m pytest tests/test_ui_walkthrough_harness_independence.py
source source_me.sh && python -m pytest tests/test_pyflakes_code_lint.py
```

The independent review accepts this static gate. It remains evidence only and
does not turn blocked onboarding, unwalked journeys, or release gates into a
pass.
