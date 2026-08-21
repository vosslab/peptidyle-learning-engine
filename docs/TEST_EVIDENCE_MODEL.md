# Test evidence model

Peptidyle uses several kinds of evidence. They answer different questions and
must not be promoted into stronger claims than they support. This distinction
keeps the normal development loop fast while preserving the real-service,
security, accessibility, and visual evidence that an educational platform
needs.

This document explains how to classify evidence. It does not replace the
acceptance gates in the active implementation plan. Read
[PYTEST_STYLE.md](PYTEST_STYLE.md), [PLAYWRIGHT_TEST_STYLE.md](PLAYWRIGHT_TEST_STYLE.md),
[E2E_TESTS.md](E2E_TESTS.md), and [HUMAN_GUIDANCE.md](HUMAN_GUIDANCE.md) before adding or
changing a test.

## Validation test suite

Every active goal or work package must name its Validation test suite before implementation reaches
completion review. The suite is the complete set of permanent gates, scope-specific integration or
live gates, one-time acceptance evidence, and required independent review that proves that goal's
claims. The active plan owns the exact scope; a focused test used while editing does not replace it.

A goal may be marked complete only when the whole named suite is green on the final material tree:

- Every required command exits zero and reports no failed test.
- A required environment-backed or release gate passes; `SKIP`, unavailable, or unrun is not green.
- Any material change after a passing run triggers the affected gate again.
- Every required independent review has no unresolved blocking finding.
- The handoff and `docs/CHANGELOG.md` record the commands, results, intentional optional skips, and
  any acceptance evidence that is deliberately one-time.

The repository aggregate Validation front door for executable behavior is:

```bash
./all_test.sh
```

It fails fast in this order: repository environment and pytest, a distinct
production `dist/` build receipt, Rust checks, codebase checks, one
`local_stack.py acceptance` invocation, the cached diff check, and the
working-tree diff check. Run `./check_rust.sh` before `./check_codebase.sh`:
the Rust gate owns the ignored `generated/` TypeScript API and fixture
projections that the TypeScript gate consumes.

`./run_playwright_tests.sh --build` remains the focused selector for the production browser suite.
It creates a fresh disposable HTTPS stack and exercises production `dist/` through the real PLE
gateway and services. `source source_me.sh && python3 local_stack.py acceptance` owns that focused
browser lane once inside its complete connected validation. Its two retained visual-fixture lanes
are transitional evidence until the screenshot migration supplies real-origin provenance; they do
not establish canonical screenshot provenance. A required skip is red. Add a named
`tests/e2e/` runner only for a PostgreSQL, MinIO, renderer, migration, restart, or other real-service
claim that the aggregate does not already own. Documentation-only goals may name focused repository
hygiene modules plus both diff checks when no executable, generated, configuration, or runtime
contract changed.

One-time probes may be required completion evidence without becoming permanent tests. Classify them
under this document and apply the permanent-test checklist before retaining anything in the suite.
When evidence is missing or a required gate is red, keep the goal active; use blocked status only
under the repository's blocking rules. Never report "complete except for validation."

## The five evidence classes

| Evidence class                                 | Kept in the repository?                        | Normal execution                                    | Answers                                                                                           | Does not answer                                                                                    |
| ---------------------------------------------- | ---------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Permanent behavior, contract, or security test | Yes                                            | Regular automated gate                              | Does a stable, maintained behavior remain true?                                                   | Does a production-like dependent service work when the test uses a fake or memory backend?         |
| Permanent architecture or hygiene gate         | Yes                                            | Regular automated gate                              | Does a durable repository rule remain true?                                                       | Does the application deliver the intended student or instructor experience?                        |
| One-time implementation probe                  | No, except a concise result record when useful | Run only while investigating or rebuilding a slice  | Did this particular implementation, migration, or reconstruction behave as expected at that time? | Does the behavior remain protected from future regressions?                                        |
| Opt-in disposable or live acceptance           | Yes, when it is a repeatable boundary oracle   | Explicit command and disposable/private environment | Does the named real boundary work under the declared environment?                                 | Does a different deployment, upstream version, browser, or institution configuration work?         |
| Independent automated or agent review          | A concise review record may be kept            | Independently scoped automated or agent review      | Does the reviewed artifact meet its stated semantic, security, architecture, or visual criterion? | Does one review make every future change correct, or replace an optional human usability judgment? |

The word _test_ is therefore not enough. A passing test report must identify
its class, backend or environment, and the exact claim it supports.

## 1. Permanent behavior, contract, and security tests

Keep a test permanently only when it protects behavior that will remain part
of PLE. The [permanent-test checklist](PYTEST_STYLE.md#is-this-a-good-pytest)
is the default rule: test logic that could plausibly be wrong; make it
deterministic, offline, and quick; assert meaningful behavior rather than a
count, field list, default value, or implementation layout. When uncertain,
delete the proposed permanent test.

### What this class proves

- A public or internal contract continues to reject invalid input or preserve
  a stated outcome.
- A security boundary continues to refuse an unauthorized, cross-tenant, or
  answer-bearing action in the environment the test actually uses.
- Two interchangeable implementations preserve the same documented behavior
  when a conformance suite is deliberately shared between them.
- A production browser built from the current source provides the user-visible
  journey exercised through its declared real-stack scenario.

### What this class cannot prove

- A memory-backed test cannot prove PostgreSQL roles, forced RLS, migrations,
  transactions, restart behavior, or an object-store delivery policy.
- An isolated decoder, serialization, or error-mapping test cannot prove the API, database, renderer,
  or network integration beyond its declared local contract.
- A recorded provider response cannot prove that the provider is currently
  reachable, authenticates PLE, or has not changed its behavior.
- A focused test does not prove an unrelated product workflow merely because
  it uses the same type or helper.

### Location, naming, and collection

- Fast Python tests belong in `tests/test_*.py` and are collected by
  `pytest tests/`. They use no network, no real CLI round trip, no sleep, and
  no filesystem beyond `tmp_path`.
- Pure Node tests belong in the repository's `tests/test_*.mjs` lane and run
  through the documented Node/check gate, not through pytest.
- Rust unit, integration, and conformance tests live with their owning crate
  and run through the focused Cargo command named by the work package.
- Browser tests live only under `tests/playwright/`. The runner loads production
  `dist/` through the suite-owned HTTPS gateway and uses visible, accessible
  controls. They are excluded from the fast pytest collection.
- Non-browser end-to-end orchestration lives only under `tests/e2e/`, with
  `e2e_*.sh` or `e2e_*.py` names. It is also excluded from pytest.

Do not use an ordinary `test_*` name for a temporary experiment, and do not
put a slow or environment-dependent check in the pytest fast lane. The
`collect_ignore` boundary is intentional, not a loophole for poorly located
tests.

### PLE examples

| Example                                                   | Claim it permanently protects                                                                                                             | Limit of the claim                                                                                                       |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `tests/test_source_file_line_limit.py`                    | Maintained authored source stays below the repository's reviewed size boundary, except for narrowly approved immutable/history overrides. | It does not show that a refactor preserved runtime behavior. It is a durable architecture gate, not a learner-flow test. |
| `crates/learning-data-access/tests/conformance.rs`        | The maintained `Store` behavior exercised by its cases is consistent for the configured conformance driver.                               | It does not by itself prove a live PostgreSQL role/RLS/migration boundary.                                               |
| Rust and TypeScript response/decoder tests                | Wire values, response-shape handling, and invalid-input refusal keep their stated contract.                                               | They do not prove a full browser-to-server round trip.                                                                   |
| `tests/playwright/student_keyboard_accessibility.spec.ts` | The built PLE browser surface preserves the specified keyboard journey and separately scoped shortcuts.                                   | It does not replace screen-reader, assistive-technology, or human usability review.                                      |
| `tests/playwright/e2e/live_demo.spec.ts` through `run_playwright_tests.sh` | A visible fictional instructor journey uses the production browser, real authentication and authorization, and the disposable connected PLE stack. | It proves only the declared local seeded scenario; reload, a new session, or an authorized observer supplies persistence evidence for each behavior. |

## 2. Permanent architecture and hygiene gates

Some permanent checks enforce an engineering boundary rather than a runtime
story. Keep them only when the boundary itself is durable and inexpensive to
check. They follow the same preference for meaningful behavior: a source-size
gate protects modular ownership; an import boundary protects answer secrecy;
a formatting gate protects a shared compilation surface. They must not turn
into a long list of arbitrary tastes.

The source-file line-limit gate is the model. Its rule comes from the owner
decision to move complete capabilities into focused modules before a file
becomes an implementation warehouse. It is not an invitation to shuffle
lines mechanically or add a test for every incidental file arrangement.

For hygiene reports, run the actual pytest module. A failure may write a
`report_*.txt` at the repository root; a clean run removes stale reports.
Those reports are diagnostics, not tracked test artifacts and not a substitute
for examining the source change.

## 3. One-time implementation probes

A one-time probe reduces uncertainty while a feature is being built,
reconstructed, or investigated. It is useful when it answers a question that
is too environment-specific, too expensive, or too tied to the current
implementation step to justify permanent maintenance.

Examples include:

- an untracked symbol or line inventory used during a module decomposition;
- a temporary SQL script that reconstructs one populated deletion graph;
- a one-off migration checksum mutation or recovery rehearsal;
- a short command that compares a newly generated payload to a known fixture;
- a local browser network trace used to inspect a newly added secrecy boundary.

### Where a probe belongs

Run a probe from a temporary directory, an ignored scratch path, or an
untracked `_temp.*` artifact. Do not put it in `tests/` merely because it has
an assertion. Do not add fixture files, snapshots, test helpers, or permanent
dependencies solely to support a probe.

If the result matters later, record the conclusion, scope, date, command or
environment, and limitation in the relevant workstream report or durable
policy document. Record enough to explain the decision, not a large command
transcript. [RETENTION_POLICY.md](RETENTION_POLICY.md) is an example: it keeps
the conclusion of a one-time populated purge and recovery rehearsal while
making clear that the temporary SQL, helper, and shell harness were removed.

### Delete rule

Delete a temporary test or probe as soon as its implementation question is
resolved unless it independently satisfies every permanent-test criterion.
In particular, delete it when it depends on exact current file layout, exact
counts, a private local service, an unrepeatable fixture, wall-clock timing,
or a migration shape that will naturally change. Do not preserve a probe just
because it caught a defect once.

The 2026-08 test-policy review retired three representative anti-patterns:

- pytests that sliced shell, Compose, Containerfile, or Caddy source and
  asserted exact fragments;
- a repository-wide lexical Rust/SQL scanner for `OFFSET` that took longer
  than the complete fast-test budget for a single case; and
- exact validators for dated walked-journey evidence rows and arrangement
  lists.

Use executable check-mode/E2E behavior for the first class, typed pagination
contracts plus query-plan review for the second, and the live report parser
plus the retained human evidence record for the third. Do not recreate these
tests under new filenames.

If a durable behavior was discovered, write the smallest independent test for
that behavior in its proper owner and location. The replacement should not
need the probe's incidental setup.

## 4. Opt-in disposable and live acceptance

Live acceptance is a maintained, repeatable oracle for a boundary that cannot
be honestly proven offline. It is deliberately opt-in because it may create
disposable containers, use private local credentials, wait for services, or
exercise an external/private renderer.

### What it proves

Only the named environment and stated boundary. Depending on the runner, that
can include SQLx migration application, PostgreSQL role grants and forced RLS,
transaction behavior, MinIO/object delivery, a private provider render and
grade round trip, a real PLE HTTP route, or a built browser speaking only to
the PLE same-origin gateway.

### What it cannot prove

- It is not evidence of a deployed production SLA, institutional tenancy
  configuration, or a different infrastructure provider.
- A container readiness probe does not prove the learner workflow.
- A direct provider probe does not prove PLE's gateway, secrecy, or browser
  boundary.
- A successful live run does not prove an unrelated service, deployment, or user journey.

### Location, activation, and result discipline

- Disposable shell/Python system oracles belong in `tests/e2e/` and run by an
  explicit `bash tests/e2e/e2e_<name>.sh` or documented Python command.
- Cargo fixtures requiring a disposable database declare the PostgreSQL
  feature and use `#[ignore = "requires the disposable PostgreSQL acceptance database"]`.
  They compile in the feature-enabled gate but run only when the documented
  disposable database command selects them.
- Rust checks that intentionally open a loopback HTTP listener or invoke an
  installed PDF/DOCX reader are also `#[ignore]` and run only through their
  named adapter or export acceptance command. They never execute in
  `check_rust.sh`'s ordinary workspace tests.
- Live Playwright specs remain in `tests/playwright/`, but must require
  explicit configuration rather than silently contacting a real service.
- `./run_playwright_tests.sh --build` owns a focused, fresh production-browser scenario against its
  disposable HTTPS stack. Run `source source_me.sh && python3 local_stack.py acceptance` for the
  complete browser Validation suite; it invokes that canonical lane once, retains two explicitly
  transitional visual-fixture receipts, owns its dedicated real-service runners, and treats every
  required skip as red.
- Temporary screenshots, traces, recordings, and Playwright results belong in
  ignored `test-results/` (or the runner's ignored output), not in permanent
  fixtures unless their durable, reviewed role is explicitly established.

Report the exact command, result, environment assumption, and any unrun gate.
Never call a live gate "passed" because its source compiled, nor call an
offline gate live because it used realistic fixture text.

### PLE examples

| Example                                                | Live claim                                                                                                                                             | Important boundary                                                                                              |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------- |
| `tests/e2e/e2e_database_baseline.sh`                   | The named disposable PostgreSQL baseline applies migrations and runs its selected database acceptance cases.                                           | It is a disposable local oracle, not a production availability or backup claim.                                 |
| `crates/learning-data-access/tests/postgres_*_live.rs` | A selected PostgreSQL behavior, including role/RLS or persistence semantics, works against `PLE_TEST_DATABASE_URL`.                                    | The fixtures are ignored by ordinary Cargo test runs and require their declared disposable environment.         |
| `tests/e2e/e2e_webwork_render_rpc.sh`                  | An isolated, capability-cleaned PLE renderer gateway can issue, replay/cache, grade, handle outage, and avoid leaking protected renderer material for the supported fixture. | It is not a claim of unrestricted WeBWorK compatibility.                                                        |
| `tests/playwright/webwork_run.spec.ts`                 | The configured live PLE/WebWork browser path uses visible learner controls and detects private upstream material in the browser trace.                 | It is opt-in acceptance for its licensed fixture and local configuration, not a generic provider certification. |

## 5. Independent automated and agent review

Independent review validates a defined artifact, fixture transition, architecture boundary, or
captured visual result. Agent and automated reviewers record their scope, criteria, conclusions,
limitations, and follow-up work, so packages have an autonomous completion path.

An independent review should name its scope, artifact or environment,
reviewer perspective, criteria, conclusions, limitations, and any follow-up
work. A dated audit or screenshot is evidence for that reviewed snapshot, not
ongoing proof. Preserve concise accepted findings in the durable document that
owns the rule; keep detailed historical review material under
`docs/active_plans/` when it explains a completed decision.

For visual and interaction work, V1 captures declared viewport and state combinations from the
production-browser scenario and applies automated image and interaction oracles. Until that
migration completes, retained visual-fixture captures remain focused evidence and do not establish
canonical screenshot provenance.

The [no-mouse accessibility contract](NO_MOUSE_ACCESSIBILITY_CONTRACT.md)
illustrates the division: automated primary-path and widget-extension tests
guard repeatable keyboard behavior, while captured fixture states and agent
review make completion reproducible. Optional human usability assessment can
inform later product decisions; it is outside automated package completion.

## Choosing the evidence before writing it

Ask these questions in order:

1. Is the behavior stable, maintained, deterministic, and cheap enough for a
   permanent offline test? If yes, add the smallest behavior/contract/security
   test in the owning test lane.
2. Does the claim require PostgreSQL, MinIO, a private renderer, a built
   browser, or another real boundary? If yes, use or extend the named opt-in
   disposable/live oracle, while retaining offline tests for logic that can be
   isolated.
3. Is the check useful only to guide this implementation step? If yes, make it
   a temporary probe, record its conclusion if it changes a durable decision,
   and remove it.
4. Does the acceptance criterion need an independent security, architecture,
   interaction, or visual reading? If yes, schedule the scoped automated or
   agent review with captured fixtures and suitable behavior evidence.

This produces a small, honest evidence set: fast checks protect enduring
behavior, live gates protect real boundaries, temporary probes guide current
work, and independently reproducible review covers the declared completion criterion.
