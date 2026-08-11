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

For a headless run or a quick restart with a known-current browser bundle:

```bash
./launch_local_stack.sh --no-open
./launch_local_stack.sh --skip-build --no-open
```

`--skip-build` requires an already-built `dist/index.html` and `dist/main.js`; use it only after a
successful `./build.sh` or normal launcher run.

## Opt-in UI walkthrough

Run the real local UI walkthrough separately from the normal test baseline:

```bash
bash tests/e2e/e2e_ui_walkthrough.sh --master-seed 42
```

It uses only IPv4 loopback (`127.0.0.1` or `localhost`). AUTO reuses safe
`dist/` outputs when they exist and builds only when they do not; `--build`
forces a fresh build and `--skip-build` requires both built outputs. The runner
writes its redacted result to `test-results/ui_walkthrough/` (default filename
`ui_walkthrough_seed_42.json`); the directory is mode 0700 and report file is
mode 0600. It is opt-in E2E evidence, not a baseline command.

The schema-v1 retained-stack learner slice is accepted: visible native keyboard pagination
reaches current targets after the first page, and manager plus independent
`--build` runs passed J1/J2/J3/J4/J5/J8. The cursor session keeps opaque
cursors and retries/deduplicates/fails closed without direct routes or API
shortcuts. The corrected walkthrough remains in progress until an instructor
visibly creates the course, activates the local student roster member, and
constructs the corpus-backed assignment before the student take/score/repeat
path. It intentionally has no email or canonical-onboarding prerequisite; see
the active plan and pagination audits under `docs/active_plans/audits/`.

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
render-and-grade probe, seeds the bounded pilot, and then starts the application. The browser
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

To prepare and validate future external email delivery, fill the six `PLE_SMTP_*` fields and
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
- Broader PG/PGML compatibility, including MATCH, requires source and live evidence beyond the
  accepted bounded radio-button path.
