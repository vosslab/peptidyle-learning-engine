# WP-G1 harness independence

## Charter addendum

The accepted schema-v1 scanner remains useful historical protection. The
corrected walkthrough excludes J9/J10 and canonical email onboarding entirely;
schema-v2 follow-up removes email-specific journey rules while retaining the
general prohibition on hidden identity, API, cookie, storage, answer, score,
and pointer shortcuts.

## Status

**Independently ACCEPTED AS ONE-TIME DESIGN EVIDENCE.** The source scanner used
during implementation was retired after the repository test-policy review: it
encoded Playwright member names and helper layout rather than product behavior
and broke on harmless refactors. The visible keyboard journeys remain the
permanent evidence. See the
[WP-G1 independent review](../audits/wp_g1_harness_independence_independent_review.md).

## Scope

This workstream records the one-time static inspection that shaped the
simulator boundary. It did not inspect or alter product code and is not a
permanent executable test contract.

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

The historical review records the scanner's conclusions. Permanent coverage
now comes from the built Playwright keyboard journeys and the runner's public
input/output contract tests. Re-run source inventories only as untracked
implementation probes; do not recreate a source-member allowlist in pytest.
This evidence does not turn blocked onboarding, unwalked journeys, or release
gates into a pass.
