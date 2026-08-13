# Usage

Use the root launcher for a complete local teaching-system test, including build, migration, seed,
health check, browser entry, native questions, and the separate standalone WeBWorK PG renderer.
PLE remains the only assignment platform: the renderer is a private stateless engine, not WebWork2.

## Quick start

Build and open the local stack:

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
`containers/local-chapter-one-pilot.json`; every instructor-readable `displayId` is a direct
immutable `P-...-v1` publication. The reviewed WeBWorK sources provide retry correctness without
answer disclosure; UUIDs remain internal routing fields.

When composing an assignment, copy each visible `P-<number>-v<version>` ID with **Copy ID** from
the published problem library and paste all four Genetics Chapter 1 IDs into **Add by question ID**.
The editor accepts exact IDs separated by commas or new lines and resolves each to that exact
immutable published version. Confirm the selected list contains the WeBWorK MC, WeBWorK MATCH,
PLE flat MC, and PLE flat MATCH versions, then keep **Timed** selected with **15** minutes per
practice run before creating the assignment. UUIDs are not an instructor input. A malformed,
unavailable, unauthorized, or already-selected ID leaves both the pasted text and the assignment
unchanged so the instructor can correct and retry it.

For a headless run or a quick restart with a known-current browser bundle:

```bash
./launch_local_stack.sh --no-open
./launch_local_stack.sh --skip-build --no-open
```

`--skip-build` requires an already-built `dist/index.html` and `dist/main.js`; use it only after a
successful `./build.sh` or normal launcher run.

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

The current human-guidance acceptance run visibly copies and pastes all four Genetics `P-n-vn`
references in J13 and uses the explicit child-input boundary above. It also shows the
keyboard-focused J1/J2/J3/J4/J5/J8 outcomes and refreshed fake-user screenshots. Email,
canonical onboarding, J6/J7, all eight response families, multi-learner, and complete two-chapter
release acceptance remain outside this walkthrough. Run
`bash tests/e2e/e2e_chapter_one_browser.sh` for the separate complete Genetics
and Biochemistry eight-question learner gate.

## Configuration preflight

```bash
./launch_local_stack.sh --check
```

This checks tool availability, required environment values, and Compose configuration without
starting a Podman machine, building artifacts, creating local secrets, or changing containers. A
first local installation has no `containers/env.local`, so use the normal launcher once to bootstrap
it before expecting `--check` to succeed.

`--check` never bootstraps an environment. This is equally true for
`./launch_local_stack.sh --env-file path/to/env.local --check`: the custom file must already exist
and satisfy the launcher contract.

## Standalone WeBWorK PG renderer

The normal launcher starts PLE with the private external PG renderer, waits for its semantic
render-and-grade probe, publishes only the exact two-assignment Chapter 1 teaching corpus, and then
starts the application. The browser
communicates with PLE only; it does not receive renderer credentials, source, or upstream state.
The renderer image must already be available locally under `PLE_WEBWORK_RENDERER_IMAGE` (normally
`localhost/pg-renderer:latest`), having been built or obtained from the separate
`webwork-pg-renderer` project.

Keep `containers/env.local` and `containers/local-login.txt` local. The renderer has no database,
persistent volume, or published host port. PLE does not run WebWork2 or MariaDB; see
[LOCAL_STACK_ARCHITECTURE.md](LOCAL_STACK_ARCHITECTURE.md).

Custom renderer settings are in [containers/env.example](../containers/env.example). PLE does not
require WebWork2 source pins, render-course credentials, or a MariaDB password.

## Build and validation commands

```bash
./build.sh                 # Rust, Wasm, generated contracts, fixtures, and Solid bundle
./check_codebase.sh        # repository-wide quality gate
cargo test --workspace     # Rust tests
npm run test:playwright    # built-browser tests
```

`npm run build` and `npm run launch` are aliases for `./build.sh` and `./launch_local_stack.sh`.
Both accept `--release` for optimized artifacts.

To validate or run a pre-existing non-default environment file, pass its path explicitly. The
launcher does not bootstrap, rewrite, seed, or create credentials for it. Its required values are
listed in [containers/env.example](../containers/env.example). The invitation issuer enables
copy-link enrollment without SMTP. Production now uses the PLE passwordless/account/session graph
with secure first-party cookies; the local-file launcher is selected only by its exact development
flag. See the [current status report](active_plans/reports/project_status_report_2026-08-10.md).

```bash
./launch_local_stack.sh --env-file path/to/env.local --check
./launch_local_stack.sh --env-file path/to/env.local --no-open
```

Fastmail is the intended future external email provider, but no SMTP or email-activation path is
configured today. The local teaching walkthrough therefore remains deliberately no-email. After an
operator account, authorized sender, and application credential exist, fill the six `PLE_SMTP_*` fields and
`PLE_PUBLIC_APP_BASE_URL` in the custom environment as described in
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md#external-smtp-provider), then select the opt-in
overlay explicitly:

```bash
./launch_local_stack.sh --env-file path/to/env.local --with-smtp --check
./launch_local_stack.sh --env-file path/to/env.local --with-smtp --no-open
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
curl -s http://127.0.0.1:8080/health
podman compose -f containers/compose.yaml --env-file containers/env.local ps
podman compose -f containers/compose.yaml --env-file containers/env.local down
```

The normal `down` command retains named data volumes. See
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for health behavior and service-specific logs,
and `docs/CONTAINER_PORT_MAPPING.md` for host and private port mappings.

## Known gaps

- Complete account-provider composition, then verify the selected provider's real email delivery
  before asking learners to use canonical passwordless sign-in.
- PG/PGML compatibility beyond the four reviewed Chapter 1 MC/MATCH sources requires its own source
  and live evidence.
