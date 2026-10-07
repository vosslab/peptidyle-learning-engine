# M23 Available Question Statistics Browser Preparation

## Scope and status

This report prepares the M23/M29 real-browser proof for available statistics on
one exact Native Matching Question Revision and its originating Question Pool.
The implementation and source checks are prepared; browser and database runtime
remain **unrun**. This is not runtime proof or M23 acceptance.

The scenario creates its own uniquely named Course through the Instructor UI.
It leaves the seeded Course and its existing three Students unchanged. The
existing five-contributor privacy threshold remains in force; six distinct
temporary Student Accounts contribute to the fixture.

## Registered journey

- Scenario ID: `m23_available_statistics`.
- Provider: `tests/e2e/e2e_browser_scenarios_m23.py`.
- Registry: `tests/e2e/e2e_browser_scenario_registry.py`.
- Browser spec: `tests/playwright/e2e/m23_available_statistics.spec.ts`.
- Helper: `tests/e2e/m23_available_statistics_fixture.py`.
- Standalone owner command:

  ```bash
  source ./source_me.sh && python3 tests/e2e/e2e_live_demo_production_browser.py --scenario m23_available_statistics
  ```

The standalone command starts a fresh `ple-live-demo-browser` owner. The Playwright
scenario creates the Course, then calls the helper with that Course Instance ID,
the scenario namespace, and a session sidecar path derived from the runner's private
scenario input. The helper accepts only the fixed `ple-live-demo-browser` PostgreSQL
service and a sidecar inside its mode-0700 owner workspace. It validates the Course
ID and namespace before constructing SQL or filesystem paths.

## Fixture data and real application paths

The helper creates six low-level synthetic Student identities and their Student
records, Course memberships, and ordinary authenticated-session rows. It inserts
`U00000009`, the existing Account mint placeholder. The Account trigger
`ple_private.account_public_id_is_minted` calls
`ple_private.assign_public_id('U')`, which generates a Crockford suffix and reserves
the unique Account ID in `ple_private.public_id_reservation`; repeated use of the
placeholder therefore creates distinct Accounts. The helper verifies canonical ID
shape, while the browser spec checks that all six resulting Account IDs differ.

Each raw session token is a fresh 32-byte random value encoded as a host cookie. The
database stores only its SHA-256 token hash. The raw cookie is written only to a
mode-0600 sidecar in the private owner workspace. The browser installs each cookie
in a separate HTTPS context with `Secure`, `HttpOnly`, host-only `__Host-` scope, and
`SameSite=Lax`. Tokens do not appear in command arguments, process output, URLs, or
logs. The helper validates fixed command/container ownership and only passes
validated public IDs, generated UUIDs, and lowercase hash bytes to SQL. These
boundaries follow ASVS 1.2.4, 2.2.1-2.2.2, 3.3.1-3.3.4, 13.3.2, and 14.2.1/14.2.4.

The Instructor creates and publishes a unique Native Matching Question, creates a
Pool containing that exact Revision, creates an Assessment on the new Course with
one selection from the Pool, turns partial-credit points off, and releases it. Six
separate Student browser contexts then start an Attempt and receive the Question
through the ordinary delivery path. The spec captures
`GET /api/assessment-attempts/{attempt}/student-question?position=1` and checks its
`publishedQuestionRevisionTuple` against the exact Question ID and Revision Number
created above. Five Students save responses through
`PUT /api/assessment-attempts/{attempt}/responses/1` and finalize through
`POST /api/assessment-attempts/{attempt}/submission`. The sixth Student commits the
presentation but leaves the response ungraded. No test route, mock statistics,
aggregate insertion, or Assessment point conversion is used to create outcomes.

The deterministic stored-credit outcomes are `0, 0.5, 1, 0, 1`; the final committed
delivery is ungraded. The half-credit Student's Assessment result is checked as
`0 of 1 points`, showing that stored Question credit remains separate from awarded
Assessment points.

## Expected browser evidence

The Question detail page route and detail API response are bound to the exact
`{ publishedQuestionId, revisionNumber }`. Its "By revision" row must show:

| Measure               | Expected value |
| --------------------- | -------------: |
| Received              |              6 |
| Graded responses      |              5 |
| Average stored credit |        50% (5) |
| Full credit           |        40% (5) |
| Zero credit           |        40% (5) |

The exact Pool ID is read from the visible creation confirmation, checked in the
Pool detail route and API response, and used for the same expected values in the
Pool statistics panel. This Pool is the source of every one of the six deliveries.

## Teardown and runtime handoff

The Playwright `finally` path closes the isolated browser contexts and, whenever
the canonical session sidecar or its `.update` temporary file exists, runs cleanup
even if fixture preparation did not return successfully. Initial sidecar creation
writes and syncs the complete mode-0600 JSON to `.update` before atomic publication;
cleanup removes a leftover `.update` on every exit. If the canonical sidecar exists,
cleanup preserves it whenever validation or database revocation fails so a retry can
validate the token hashes and revoke the sessions. After successful revocation it
removes the canonical raw-token sidecar before writing the origin receipt. The
configured worst-case operation timeout budget is 164 seconds: 12 seconds for the
container lookup, six serial psql calls capped at 20 seconds each, and a possible
12-second container lookup plus 20-second revoke when preparation cleanup runs. This
fits inside the 180-second child timeout. Synthetic
Account and membership history and privacy-safe aggregate rows are not deleted by
the helper. After the standalone browser run, stop the dedicated owner to reset its
disposable database and return the environment to its normal state:

```bash
source ./source_me.sh && python3 local_stack.py stop
```

Root owns the fresh browser/database execution and central ledger update. Until that
run succeeds, the exact metrics, Course/Question authoring path, Student session
fixture, and visible statistics remain source expectations only.
