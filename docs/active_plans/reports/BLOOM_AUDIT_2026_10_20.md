# Bloom integration audit and test pruning

## Boundary and independent passes

Reviewed the uncommitted Bloom integration and its new permanent tests. Existing unrelated tests,
vendored tests, `.agents/`, and `skills-lock.json` were outside this change review. Six fresh,
independent reviewers completed the Plan, Test, Style, Docs, Legacy, and Comment passes.
The manager assessed findings and applied the corrections below.

## Findings and disposition

- Medium: current terminology and Question specification indexes still deferred initial Bloom
  assignment or described the former Pool rule. Updated these to post-publication JEV assignment,
  member modes on Pool membership changes, and inherited fork metadata.
- Medium: the HTML evidence test could pass after losing an answer-control value because the same
  value appeared elsewhere. Changed that input to a unique value and asserted its preservation.
- Low: the Pool queue hook was named as a publication event although membership saves also call it.
  Renamed it to `queue_pool_bloom_calculation` without changing behavior.
- Low: public Bloom data fields lacked intent documentation and an embedded-rubric `unwrap`
  lacked a stated invariant. Added concise documentation and a descriptive `expect`.
- Low: two decoder comments retained the old AI deferral, the Bloom specification retained stale
  database acceptance wording, and Compose credential wording obscured its environment-only
  wiring. Corrected the comments and documentation.

No architecture-level findings remain. The Plan pass had no findings. Other passes found the
bounded corrections above; no permanent test additions were proposed.

## Test pruning result

The boundary contains three focused Rust tests and one SQL oracle using the existing disposable
database baseline. All earn retention: they protect external label validation, answer-bearing
Native evidence, readable WeBWorK evidence, and atomic database writes and Pool mode calculation.
The SQL oracle also exercises application-role access and rejection of stale metadata/membership
results. No new service, runner, or shared fixture infrastructure exists solely for these checks.

No tests or support files were deleted; no maintenance machinery was removed. The existing HTML
test now protects its stated behavior more effectively. No protection was intentionally lost.

## Validation and limits

Before this audit, `launchers/all_test.sh` passed, including the disposable PostgreSQL oracle and
9431 Python tests. Live Native JEV probes completed in 222-262 ms; these remain one-time evidence.
Audit corrections are documentation, naming, and one existing assertion improvement. Fresh fast
checks passed through `launchers/run_fast_checks.sh`: Rust, TypeScript, and 9432 Python checks.
The connected database gate was not rerun because its SQL and behavior were unchanged.

Live WeBWorK renderer-to-JEV classification remains unverified. Focused evidence extraction is
covered. Accepted best-effort queue loss, NULL metadata after service failure, and revision
inheritance remain deliberate behavior, not unresolved audit findings.
