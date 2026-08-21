# Usage

Use the root local-stack controller for a complete local teaching-system test. Focused typed Python
lifecycle modules own build, migration, seed, health check, browser entry, native questions, and the
separate standalone WeBWorK PG renderer. PLE remains the only assignment
platform: the renderer is a private stateless engine, not WebWork2.

## Quick start

Build and open the local stack:

```bash
source source_me.sh && python3 local_stack.py start
```

The lifecycle prints the loopback application URL and the path to ignored local instructor and
student credentials. Paste one value from `containers/local-login.txt` into the local sign-in form.
The resulting browser session is HttpOnly; the bearer value is not stored in browser storage.
This local-file session exercises seeded course work, not passwordless account creation,
invitation claim, email sign-in, or passkey enrollment.

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

For a headless run or a quick restart with a known-current browser bundle:

```bash
source source_me.sh && python3 local_stack.py start --no-open
source source_me.sh && python3 local_stack.py start --skip-build --no-open
```

`--skip-build` requires an already-built `dist/index.html` and `dist/main.js`; use it only after a
successful `./build.sh` or normal controller run.

## Local stack controller

Run `source source_me.sh && python3 local_stack.py --help` to see the supported command surface.
The controller is the normal front door. It resolves the repository root and explicit Compose
target, keeps inspection read-only, and calls its focused lifecycle modules directly.

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py projects
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs gateway api worker
source source_me.sh && python3 local_stack.py validate
```

- `doctor` reports the Podman engine, rootless state, Compose provider, local machine on macOS,
  environment-file metadata, and labelled projects without printing secret values.
- `projects` lists labelled Compose projects, including a project that retains data volumes after a
  normal stop.
- `status` reports semantic readiness for the default project or an explicitly named read-only
  `--project`; `--json` is available on `doctor`, `projects`, `status`, and `validate`.
- `logs` scopes output to the selected project. It defaults to `gateway api worker`; pass
  `--tail N`, `--follow`, or supported service names as needed. Logs can contain private local
  diagnostics, so do not publish them.
- `validate` runs the typed lifecycle's canonical read-only configuration check, then reports observed
  runtime state. It does not bootstrap a missing environment file.

Start, stop, and restart use the default `containers` project. `start` is the only ordinary path
that initializes, migrates, seeds, checks the renderer, and waits for readiness.

```bash
source source_me.sh && python3 local_stack.py start --no-open
source source_me.sh && python3 local_stack.py start --skip-build --no-open
source source_me.sh && python3 local_stack.py restart webwork-renderer
source source_me.sh && python3 local_stack.py stop
```

`start` accepts `--release`, `--skip-build`, `--no-open`, `--env-file PATH`, and `--with-smtp`.
`restart` is deliberately limited to `api`, `worker`, `gateway`, or `webwork-renderer`; it routes
through the lifecycle-owned readiness path and refuses a stack that is not ready. `stop` runs a
project-scoped Compose shutdown and retains the named PostgreSQL, MinIO, and local identity data
volumes for the next `start`.

Reset is the separate destructive operation. Preview its exact labelled project, resources,
database-bound Chapter 1 manifest, and Compose command first; only the confirmation form removes
the default stack's named data volumes and then clears that manifest after the labelled resources
are gone.

```bash
source source_me.sh && python3 local_stack.py reset --dry-run
source source_me.sh && python3 local_stack.py reset --confirm-project containers
source source_me.sh && python3 local_stack.py start --no-open
```

Use `reset` for the displayed `containers` target. Global Podman cleanup and image lifecycle use
their dedicated commands. After a confirmed reset, use `start` to recreate and seed the disposable
pre-production teaching data.

Run the complete live browser Validation test suite only when there is no existing default or
walkthrough stack that the suite could mistake for its own:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

This is the only public aggregate command; it requires every live lane to
finish without skips and preserve a conflicting caller-owned stack by refusing before mutation.

## Instructor and student guides

- [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) follows visible course creation, canonical no-contact
  roster membership/enrollment for the fictional learner, corpus-backed assignment construction, and
  gradebook review. It also documents the accepted WP-PROF-S6 grade-settings, compact-total, and
  synchronous audited CSV workflow. Local-file configuration authenticates the fictional actor only.
- [STUDENT_GUIDE.md](STUDENT_GUIDE.md) follows the keyboard-only take, score, correction, and fresh
  practice loop.

Both guides describe the bounded local no-email pilot. They do not claim email registration or a
production deployment.

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

## Opt-in UI walkthrough

Run the real local UI walkthrough separately from the normal test baseline:

```bash
bash tests/walkthrough/run_ui_walkthrough.sh --master-seed 42
```

It uses only IPv4 loopback (`127.0.0.1` or `localhost`). AUTO reuses safe
`dist/` outputs when they exist and builds only when they do not; `--build`
forces a fresh build. The runner
writes its redacted result to `test-results/ui_walkthrough/` (default filename
`ui_walkthrough_seed_42.json`); the directory is mode 0700 and report file is
mode 0600. It is opt-in E2E evidence, not a baseline command.

Choose the walkthrough seed, selected Compose file, report name, build refresh,
and documentation screenshot directory with its documented arguments. The
runner does not read inherited `PLE_*` switches as hidden walkthrough input.
It passes fixed browser and Node children one generated schema-versioned private
input file by an explicit `--inputs` argument; this file is runner-owned,
mode 0600 inside a mode-0700 directory, and is not an operator configuration
file to edit or retain.

The current human-guidance acceptance run uses the four Genetics `AAA-BBBB` Question IDs in J13
and the explicit child-input boundary above. It also shows the
keyboard-focused J1/J2/J3/J4/J5/J8 outcomes and refreshed fake-user screenshots. Email,
canonical onboarding, J6/J7, all eight response families, multi-learner, and complete two-chapter
release acceptance remain outside this walkthrough. Run
`source source_me.sh && python3 tests/e2e/e2e_chapter_one_browser.py` for the separate complete Genetics
and Biochemistry eight-question learner gate.

## Configuration preflight

```bash
source source_me.sh && python3 local_stack.py validate
```

This checks tool availability, required environment values, and Compose configuration without
starting a Podman machine, building artifacts, creating local secrets, or changing containers. A
first local installation has no `containers/env.local`, so use `python3 local_stack.py start --no-open`
once to bootstrap it before expecting validation to succeed.

`validate` never bootstraps an environment. This is equally true for a custom
`--env-file path/to/env.local`: the file must already exist and satisfy the typed lifecycle contract.

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

Keep `containers/env.local` and `containers/local-login.txt` local. The renderer has no database,
persistent volume, or published host port. PLE does not run WebWork2 or MariaDB; see
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md).

Custom renderer settings are in [containers/env.example](../containers/env.example). PLE does not
require WebWork2 source pins, render-course credentials, or a MariaDB password.

## Build and validation commands

```bash
./build.sh                 # Rust, Wasm, generated contracts, fixtures, and Solid bundle
./check_codebase.sh        # vendored TypeScript and browser gate
./check_rust.sh            # repository-owned Cargo and Rust gate
./run_playwright_tests.sh --build       # focused production-browser real-stack scenario
source source_me.sh && python3 local_stack.py acceptance   # complete opt-in Playwright validation suite
```

The focused Playwright command owns a fresh disposable HTTPS PLE stack and runs production `dist/`
through the gateway, real authentication and authorization, API, PostgreSQL, MinIO, worker, and
renderer. It finishes with zero skipped tests and creates its scenario state through visible PLE
workflows. Use it for a selected browser behavior without reusing a locally running stack.

`source source_me.sh && python3 local_stack.py acceptance` is the complete Playwright Validation test suite. Begin
with no existing default or retained walkthrough stack; the command refuses either caller-owned
target instead of stopping it. It runs the canonical browser suite once, two transitional visual-fixture
checks, the canonical UI walkthrough, and the dedicated Chapter 1 and WebWork browser owners. The
visual-fixture checks do not provide canonical screenshot provenance; V1 migrates that evidence to the
suite-owned real origin. It accepts no URL, credential,
or Compose-project override. Any required lane that fails or skips is red. The command starts only
the disposable or canonical local stacks that its owning runners create and clean up.

Use a focused `cargo test -p <package> <filter>` while editing one Rust behavior, then run
`./check_rust.sh` for the complete offline Rust acceptance gate.

`npm run build` remains an alias for `./build.sh`; use the Python controller for local-stack lifecycle.
Both accept `--release` for optimized artifacts.

To validate or run a pre-existing non-default environment file, pass its path explicitly. The
lifecycle does not bootstrap, rewrite, seed, or create credentials for it. Its required values are
listed in [containers/env.example](../containers/env.example). The invitation issuer enables
copy-link enrollment without SMTP. Production now uses the PLE passwordless/account/session graph
with secure first-party cookies; the local-file lifecycle is selected only by its exact development
flag. See the [current status report](active_plans/reports/project_status_report_2026-08-10.md).

A custom environment must set `PLE_QUESTION_ID_SECRET_HOST_FILE` to an absolute, regular,
non-symlink, mode-0600 file containing exactly one canonical 32-byte base64url secret. This is the
durable server-only HMAC key for human Question IDs. Its rotation invalidates every current human
Question ID, so treat a rotation as a coordinated identity change; see
[QUESTION_ID_SPEC.md](QUESTION_ID_SPEC.md#secret-handling).

```bash
source source_me.sh && python3 local_stack.py validate --env-file path/to/env.local
source source_me.sh && python3 local_stack.py start --env-file path/to/env.local --no-open
```

Fastmail is the intended future external email provider, but no SMTP or email-activation path is
configured today. The local teaching walkthrough therefore remains deliberately no-email. After an
operator account, authorized sender, and application credential exist, fill the six `PLE_SMTP_*` fields and
`PLE_PUBLIC_APP_BASE_URL` in the custom environment as described in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md#external-smtp-provider), then select the opt-in
overlay explicitly:

```bash
source source_me.sh && python3 local_stack.py validate --env-file path/to/env.local --with-smtp
source source_me.sh && python3 local_stack.py start --env-file path/to/env.local --with-smtp --no-open
```

This connects to the selected provider with authenticated encrypted submission; it does not start a
PLE mail service. Production composition can now enter the canonical account/session route graph,
but a live provider send and browser acceptance remain open. Omitting `--with-smtp` leaves copy-link
invitations available. Local-file credentials cannot claim an invitation or register a first passkey.

## Stack inspection

After a local run, health is served through the one loopback gateway origin. Use the printed URL,
or read `PLE_GATEWAY_HOST_PORT` in `containers/env.local`; the default is `8080` and fallback uses
the `8000-8099` gateway range. Existing explicit local values remain valid until changed.

```bash
source source_me.sh && python3 local_stack.py doctor
source source_me.sh && python3 local_stack.py status
source source_me.sh && python3 local_stack.py logs gateway api worker
curl -s http://127.0.0.1:8080/health
```

Use `source source_me.sh && python3 local_stack.py stop` for the normal data-retaining shutdown;
do not substitute a bare Compose command. See
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for health behavior and service-specific logs,
and [CONTAINER_PORT_MAPPING.md](CONTAINER_PORT_MAPPING.md) for host and private port mappings.

## Known gaps

- Complete account-provider composition, then verify the selected provider's real email delivery
  before asking learners to use canonical passwordless sign-in.
- PG/PGML compatibility beyond the four reviewed Chapter 1 MC/MATCH sources requires its own source
  and live evidence.
