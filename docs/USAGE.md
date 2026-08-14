# Usage

Use the root local-stack controller for a complete local teaching-system test. It delegates startup
to the maintained launcher, which owns build, migration, seed, health check, browser entry, native
questions, and the separate standalone WeBWorK PG renderer. PLE remains the only assignment
platform: the renderer is a private stateless engine, not WebWork2.

## Quick start

Build and open the local stack:

```bash
source source_me.sh && python3 local_stack.py start
```

The direct launcher is reserved for recovering or diagnosing launcher behavior:

```bash
./launch_local_stack.sh
```

The launcher prints the loopback application URL and the path to ignored local instructor and
student credentials. Paste one value from `containers/local-login.txt` into the local sign-in form.
The resulting browser session is HttpOnly; the bearer value is not stored in browser storage.
This local-file session exercises seeded course work, not passwordless account creation,
invitation claim, email sign-in, or passkey enrollment.

The normal local launch publishes the reviewed Genetics Chapter 1 and Biochemistry Chapter 1
Mastery assignments. Each has exactly four questions in the documented order: WeBWorK MC, WeBWorK
MATCH, PLE flat MC, and PLE flat MATCH. The answer-free seed manifest is written with mode 0600 to
`containers/local-chapter-one-pilot.json`; every instructor-readable `displayId` is a current
`AAA-BBBB` Question ID. The reviewed WeBWorK sources provide retry correctness without
answer disclosure; UUIDs remain internal routing fields.

When composing an assignment, prefer **Reuse questions from an existing assignment** and either
add the whole set or select questions from its checklist. For direct lookup, copy a visible
`AAA-BBBB` ID with **Copy ID** from the library. The editor accepts IDs separated by commas or new
lines and resolves each to the current question. Confirm the selected list contains the WeBWorK MC,
WeBWorK MATCH, PLE flat MC, and PLE flat MATCH questions, then keep **Timed** selected with **15** minutes per
practice run before creating the assignment. UUIDs are not an instructor input. A malformed,
unavailable, unauthorized, or already-selected ID leaves both the pasted text and the assignment
unchanged so the instructor can correct and retry it.

For a headless run or a quick restart with a known-current browser bundle:

```bash
source source_me.sh && python3 local_stack.py start --no-open
source source_me.sh && python3 local_stack.py start --skip-build --no-open
```

If the controller itself is being diagnosed, the direct launcher equivalents are:

```bash
./launch_local_stack.sh --no-open
./launch_local_stack.sh --skip-build --no-open
```

`--skip-build` requires an already-built `dist/index.html` and `dist/main.js`; use it only after a
successful `./build.sh` or normal launcher run.

## Local stack controller

Run `source source_me.sh && python3 local_stack.py --help` to see the supported command surface.
The controller is the normal front door. It resolves the repository root and explicit Compose
target, keeps inspection read-only, and delegates initialization and service readiness to the
launcher instead of reproducing them in a second startup path.

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
- `validate` runs the launcher's canonical read-only configuration check, then reports observed
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
through the launcher-owned readiness path and refuses a stack that is not ready. `stop` runs a
project-scoped Compose shutdown and retains the named PostgreSQL, MinIO, and local identity data
volumes for the next `start`.

Reset is the separate destructive operation. Preview its exact labelled project, resources, and
Compose command first; only the confirmation form removes the default stack's named data volumes.

```bash
source source_me.sh && python3 local_stack.py reset --dry-run
source source_me.sh && python3 local_stack.py reset --confirm-project containers
source source_me.sh && python3 local_stack.py start --no-open
```

Do not use reset for an unknown, caller-owned, or disposable project. It never performs global
Podman cleanup and does not remove images. After a confirmed reset, use `start` to recreate and
seed the disposable pre-production teaching data.

Run the complete live browser Validation test suite only when there is no existing default or
walkthrough stack that the suite could mistake for its own:

```bash
source source_me.sh && python3 local_stack.py acceptance
```

This is equivalent to `./run_playwright_validation.sh --live`; both require every live lane to
finish without skips and preserve a conflicting caller-owned stack by refusing before mutation.

## Instructor and student guides

- [INSTRUCTOR_GUIDE.md](INSTRUCTOR_GUIDE.md) follows visible course creation, canonical no-contact
  roster membership/enrollment for the fictional learner, corpus-backed assignment construction, and
  gradebook review. Local-file configuration authenticates the fictional actor only.
- [STUDENT_GUIDE.md](STUDENT_GUIDE.md) follows the keyboard-only take, score, correction, and fresh
  practice loop.

Both guides describe the bounded local no-email pilot. They do not claim email registration or a
production deployment.

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
`bash tests/e2e/e2e_chapter_one_browser.sh` for the separate complete Genetics
and Biochemistry eight-question learner gate.

## Configuration preflight

```bash
source source_me.sh && python3 local_stack.py validate
```

This checks tool availability, required environment values, and Compose configuration without
starting a Podman machine, building artifacts, creating local secrets, or changing containers. A
first local installation has no `containers/env.local`, so use `local_stack.py start --no-open`
once to bootstrap it before expecting validation to succeed.

`validate` never bootstraps an environment. This is equally true for a custom
`--env-file path/to/env.local`: the file must already exist and satisfy the launcher contract.

## Standalone WeBWorK PG renderer

The normal launcher starts PLE with the private external PG renderer, waits for its semantic
render-and-grade probe, publishes only the exact two-assignment Chapter 1 teaching corpus, and then
starts the application. The browser
communicates with PLE only; it does not receive renderer credentials, source, or upstream state.
The renderer image must already be available locally under the immutable
`PLE_WEBWORK_RENDERER_IMAGE` reference (normally
`localhost/pg-renderer@sha256:d606c4b5d82d425729643c4f36d093d549759a416d0527f0340ae0a7319a8456`),
having been built or obtained from the separate `webwork-pg-renderer` project. Build that sibling
under a convenient tag, copy its reviewed `RepoDigests` manifest reference, then place that full
`repository@sha256:<64-lowercase-hex>` value in `containers/env.local`; PLE never builds the
renderer and rejects mutable tags. The launcher records the separately resolved image configuration
ID as renderer-version provenance after it resolves the configured manifest reference.

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
./run_playwright_tests.sh --build       # ordinary mock-backed browser suite
./run_playwright_validation.sh --live   # complete opt-in Playwright validation suite
```

The ordinary Playwright command uses the mock preview server and proves built-browser behavior, not
the Podman stack. It finishes with zero skipped tests: real-stack, walkthrough, and visual cases
are deliberately not ordinary collection. Use it for the daily browser gate, even when a local
stack happens to be running.

`./run_playwright_validation.sh --live` is the complete Playwright Validation test suite. Begin
with no existing PLE stack; the command refuses a caller-owned default or disposable stack instead
of stopping it. It runs the ordinary suite, temporary-only visual checks, the canonical UI
walkthrough, and the dedicated Chapter 1 and WebWork browser owners. It accepts no URL, credential,
or Compose-project override. Any required lane that fails or skips is red. The command starts only
the disposable or canonical local stacks that its owning runners create and clean up.

Use a focused `cargo test -p <package> <filter>` while editing one Rust behavior, then run
`./check_rust.sh` for the complete offline Rust acceptance gate.

`npm run build` and `npm run launch` are aliases for `./build.sh` and `./launch_local_stack.sh`.
Both accept `--release` for optimized artifacts.

To validate or run a pre-existing non-default environment file, pass its path explicitly. The
launcher does not bootstrap, rewrite, seed, or create credentials for it. Its required values are
listed in [containers/env.example](../containers/env.example). The invitation issuer enables
copy-link enrollment without SMTP. Production now uses the PLE passwordless/account/session graph
with secure first-party cookies; the local-file launcher is selected only by its exact development
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
