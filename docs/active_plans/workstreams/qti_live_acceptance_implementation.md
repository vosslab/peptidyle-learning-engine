# QTI live acceptance implementation

Status: WP-QTI-11 and its WP-QTI-12 independent review/documentation close-out are complete and
accepted on 2026-08-09. They are accepted history. Current authority is
[release_completion_plan.md](../active/release_completion_plan.md): WP-RC3 shipped upstream WeBWorK
is current, WP-ARCH1 follows it, then WP-RC4 owns the owner-supplied QTI-JSONL contract, WP-RC5 owns
families and Chapter 1 content, and WP-RC6 closes QTI export and H5P claims.

## Scope

WP-QTI-11 accepts the existing profile-import implementation through the real disposable
PostgreSQL 17 path. It does not widen the Canvas or Blackboard profiles, add a browser contract, or
change a production persistence contract.

## Accepted behavior

- A fresh database applies the exact six-file SQLx baseline, accepts a second no-op migration run,
  verifies the migration ledger, and exercises real least-privilege roles.
- The real upload route and worker process one minimized archive containing an accepted Canvas QTI
  1.2 static-single-choice item and a rejected sibling. The safe report preserves both outcomes.
- The accepted item converts through the native flat bridge, advances the workspace draft revision,
  remains editable, publishes as immutable native flat content, and grades both a correct and an
  incorrect response through the isolated PostgreSQL grader.
- The retained workspace archive, current origin, canonical source, published archive, published
  origin, and their checksums agree. Current private origin and choice-map state is cleaned with the
  workspace; published provenance remains immutable and retained.
- Application, student, grader, and foreign-tenant probes enforce the documented RLS and capability
  boundaries. Safe reports and DTOs contain no archive bytes, object keys, private choice maps,
  grader payloads, correct-choice material, or unreleased feedback.
- The disposable runner removes only its exact Compose project, network, volumes, and temporary
  database. It leaves the pre-existing developer PostgreSQL instance untouched.

## Gate corrections

The package-wide gate exposed two acceptance-fixture defects outside the production path:

- The QTI provenance SQL embedded a CJK scalar directly in tracked source. PostgreSQL `U&` escapes
  now preserve the same 1,024-scalar and 1,025-scalar boundary behavior while keeping maintained
  source ASCII/ISO-8859-1 compatible. The corresponding Rust URL fixture uses `\u{03b2}` for the
  same reason.
- The mounted feedback Playwright fixture submitted an `externalTool` marker to native
  multiple-choice attempts. It now submits the established native `carbonyl` choice, preserving the
  strict external-tool boundary and allowing the feedback state machine to reach its intended
  disclosure assertions.

## Validation

Focused gates:

- `cargo test -p adapter_qti`: 93 unit, 6 corpus, and 12 documentation tests passed.
- `cargo test -p adapter_native flat_question`: 13 passed.
- `cargo test -p learning-data-access --test conformance qti_`: 15 passed.
- `cargo test -p learning-data-access --test conformance flat_import`: 6 passed.
- `cargo test -p server_core qti_profile`: 16 passed; the one credentialed live test remained
  ignored in the ordinary suite and was invoked by the disposable runner.
- `node --import tsx --test tests/test_qti_profile_import.mjs`: 5 passed.
- `bash run_playwright_tests.sh tests/playwright/qti_profile_import.spec.ts --reporter=line`: 4
  passed.
- `bash run_playwright_tests.sh tests/playwright/feedback_submission_flow.spec.ts --reporter=line`:
  3 passed after the fixture correction.

Complete package gates:

- `bash tests/e2e/e2e_database_baseline.sh`: passed on a fresh isolated PostgreSQL 17 database,
  including the profile-to-native authoring, publication, correct/incorrect grading, provenance,
  role-denial, retention, and cleanup oracles.
- `cargo fmt --check`, strict workspace Clippy, and `cargo test --workspace`: passed.
- `./check_codebase.sh`: all 11 stages passed.
- `bash run_playwright_tests.sh --build --reporter=line`: 51 passed.
- `source source_me.sh && python3 -m pytest -q tests/`: 1,644 passed.
- TypeScript no-emit compilation plus staged and unstaged diff checks passed.

## Independent review

WP-QTI-12 ran six separate plan, test, style, documentation, legacy, and comment review passes.
Plan, test, legacy, and comment review found no issue. Style review initially requested clickable
evidence references; all actionable tracked-file references were corrected, while the deliberately
unstaged evidence file remains a code path because the repository link gate rejects untracked link
targets. The style re-review found no actionable issue.

Documentation review found two close-out blockers: the README still named WP-QTI-8 as the latest
accepted package, and the contracts, architecture, and file map omitted the completed route, worker,
conversion, author UI, protected-grader, and live-oracle ownership path. Those four owner documents
were corrected. Their Markdown link, ASCII, first-paragraph, whitespace, and Prettier gates passed;
the original documentation reviewer then confirmed both findings resolved. No P0/P1 finding remains.

## Deferred boundary

This acceptance does not claim optional WP-QTI-13 exporters, broader vendor compatibility, imported
media, or additional flat-question families.

## Historical successor

The owner-supplied QTI-JSONL artifacts are an assigned WP-RC4 package, not an unowned wait. WP-RC4
then provides the contract consumed by WP-RC5 and WP-RC6 in the release plan's dependency order.

## Repository state

The package started from clean `main` at `b297808`. It changes no Git index state. The current
working-tree changes remain unstaged for owner review.
