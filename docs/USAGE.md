# Usage

Use the root local-stack controller for the fixed developer browser session.
The owner builds production `dist/`, creates a fresh `ple-live-demo-browser`
HTTPS stack, and runs the seeded production-auth path. PLE remains the only
assignment platform: the renderer is a private stateless engine, not WebWork2.

## Quick start

Build and open the local stack:

```bash
source source_me.sh && python3 local_stack.py start
source source_me.sh && python3 local_stack.py start --no-open
```

The first command opens the canonical HTTPS origin. The second keeps the
browser closed and prints the origin. Follow the visible seeded production-auth
flow; this entry point has no alternate credential form or auth switch.

When creating a teaching course, enter its title, inclusive start and end calendar dates, and the
exact case-sensitive IANA time zone used for that course (for example, `America/Chicago`). The form
does not infer the browser or account zone. An invalid date order or unknown zone keeps every value,
announces the specific correction, focuses its field, and permits a direct retry without creating a
partial course.

The normal local launch publishes the reviewed Genetics Chapter 1 and Biochemistry Chapter 1
Mastery assignments. Each has exactly four questions in the documented order: WeBWorK MC, WeBWorK
MATCH, PLE flat MC, and PLE flat MATCH. The answer-free seed manifest is written with mode 0600 to
`containers/local-chapter-one-pilot.json`; every instructor-readable `displayId` is the ID of that
exact published `AAA-BBBB` question. The reviewed WeBWorK sources provide retry correctness without
answer disclosure; UUIDs remain internal routing fields.

When composing an assignment, prefer **Reuse questions from an existing assignment** and either
add the whole set or select questions from its checklist. For direct lookup, copy a visible
`AAA-BBBB` ID with **Copy ID** from the library. The editor accepts IDs separated by commas or new
lines and resolves each to that exact published question; it never resolves a successor or "latest"
version. Confirm the selected list contains the WeBWorK MC, WeBWorK MATCH, PLE flat MC, and PLE
flat MATCH questions, then create the Draft assignment. Open its **Teaching operations** panel to
add student instructions, publish it, and set the course-local availability/due/close schedule,
**15** minutes per run, attempt limit, late behavior, and deadline behavior. This separate revisioned
save converts local timestamps on the server using the course IANA zone; it never trusts the browser
clock. UUIDs are not an instructor input. A malformed,
unavailable, unauthorized, or already-selected ID leaves both the pasted text and the assignment
unchanged so the instructor can correct and retry it.

An authored correction is a new Question ID, optionally linked to its source
through provenance. Existing assignments remain pinned to their selected
questions until the Instructor deliberately makes a revision-checked
replacement. The editor shows the existing and replacement Question IDs before
confirmation. Issued runs retain their original exact evidence; future runs use
the selected replacement.

Use `--no-open` for a headless run. The fixed owner always builds the
production browser bundle as part of startup.

## Local stack controller

The developer session uses the fixed `ple-live-demo-browser` project. `start`
initializes, migrates, seeds, checks the renderer, builds production `dist/`,
and waits for HTTPS readiness. `stop` authenticates to the active supervisor,
then verifies owner-scoped cleanup.

```bash
source source_me.sh && python3 local_stack.py stop
```

Developer and browser tests serialize through the same fixed owner lease. Do not
use a project selector, alternate environment, SMTP switch, or skipped-build
option with this session.

The fixed owner performs exact cleanup for its own disposable
resources. Do not use a project selector or global Podman cleanup to recover a
developer session; preserve the private failure receipt and follow
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

Run the complete live browser Validation test suite through its canonical owner:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

This is the only public aggregate command; it requires every live lane to
finish without skips and preserve a conflicting caller-owned stack by refusing before mutation.

## Instructor and student guides

- [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) follows visible course creation, roster
  membership/enrollment, corpus-backed assignment construction, and gradebook review. It also
  documents the accepted WP-PROF-S6 grade-settings, compact-total, and synchronous audited CSV
  workflow.
- [STUDENT_GUIDE.md](STUDENT_GUIDE.md) follows the keyboard-only take, score, correction, and fresh
  practice loop.

Both guides describe the bounded developer pilot. They do not claim a production deployment.

## Course-grade routes

The instructor course-grade boundary is mounted at these same-origin paths:

- `GET/PUT /api/courses/{course}/grade-scheme` reads and revision-CAS saves the title-free scheme
  settings. Reads include current server-owned assignment titles. The selector currently offers total
  points and weighted categories with drop-lowest; completion-based grading is deferred.
- `GET /api/courses/{course}/gradebook-totals` returns compact server-calculated totals from one scheme
  snapshot and maintained assignment summaries. The no-store response excludes email and raw summary
  data and preserves explicit unavailable states.
- `POST /api/courses/{course}/grade-export.csv` accepts an empty body and synchronously returns a
  bounded CSV. The response is no-store; only PII-free audit metadata is durable.

These routes are the accepted WP-PROF-S6 capability. The ordinary `containers` demo, live
PostgreSQL/browser evidence, and all seven aggregate acceptance lanes are green. Use the local
full-stack demo when testing the networked service boundary; fast offline checks remain deterministic
test gates only.

## Configuration preflight

The fixed owner validates its complete production-auth configuration before
starting. Do not substitute a custom environment, identity, SMTP, or build
selector; use [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for a failed preflight.

## Standalone WeBWorK PG renderer

The normal typed lifecycle starts PLE with the private external PG renderer, waits for its semantic
render-and-grade probe, publishes only the exact two-assignment Chapter 1 teaching corpus, and then
starts the application. The browser
communicates with PLE only; it does not receive renderer credentials, source, or upstream state.
The renderer image must already be available locally under the reviewed name
selected by `PLE_WEBWORK_RENDERER_IMAGE` (normally
`localhost/pg-renderer:reviewed`), having been built or obtained from the
separate `webwork-pg-renderer` project. Build that sibling under that designated
name. The lifecycle resolves its OCI configuration ID before startup, confirms
the renderer container uses the same ID, and atomically records the selected
name plus ID as renderer-version provenance. A published deployment may select
a pullable `repository@sha256:<64-lowercase-hex>` value through the same
setting.

The renderer has no database, persistent volume, or published host port. PLE does
not run WebWork2 or MariaDB; see
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md).

Custom renderer settings are in [containers/env.example](../containers/env.example). PLE does not
require WebWork2 source pins, render-course credentials, or a MariaDB password.

## Build and validation commands

```bash
./build.sh                 # Rust, Wasm, generated contracts, fixtures, and Solid bundle
./check_codebase.sh        # vendored TypeScript and browser gate
./check_rust.sh            # repository-owned Cargo and Rust gate
./run_playwright_tests.sh --build       # canonical production-browser selection
source source_me.sh && python3 local_stack.py acceptance   # complete no-skip validation suite
```

The focused Playwright command owns a fresh disposable HTTPS PLE stack and runs production `dist/`
through the gateway, real authentication and authorization, API, PostgreSQL, MinIO, worker, and
renderer. It finishes with zero skipped tests and creates its scenario state through visible PLE
workflows. Use it for a selected browser behavior without reusing a locally running stack.

`source source_me.sh && python3 local_stack.py acceptance` is the complete Playwright Validation test suite.
It invokes the canonical browser selection and the browser-free service lanes;
every required lane must pass with no skips. Browser selection remains through
`run_playwright_tests.sh`, which owns the production HTTPS stack and cleanup.

Use a focused `cargo test -p <package> <filter>` while editing one Rust behavior, then run
`./check_rust.sh` for the complete offline Rust acceptance gate.

`npm run build` remains an alias for `./build.sh`; use the fixed owner through
`local_stack.py start` for the developer browser session. The start/stop
boundary has no custom environment, identity, SMTP, or build selector.

## Stack inspection

After a run, health is served through the printed HTTPS origin owned by the
developer session. Use the controller's read-only diagnostics when investigating
the active owner; do not infer a port or reuse a different project.

```bash
source source_me.sh && python3 local_stack.py start --no-open
```

Use `source source_me.sh && python3 local_stack.py stop` for authenticated
owner cleanup; do not substitute a bare Compose command. See
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for the owner contract.

## Known gaps

- PG/PGML compatibility beyond the four reviewed Chapter 1 MC/MATCH sources requires its own source
  and live evidence.
