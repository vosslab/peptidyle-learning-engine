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
copy-link enrollment without SMTP. The current startup root still selects local-file development
identity, so canonical PLE email sign-in remains unavailable; see the
[current status report](active_plans/reports/project_status_report_2026-08-10.md).

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
PLE mail service. In the current executable local and production composition, it prepares delivery
only: canonical PLE email sign-in remains unavailable until the account-provider composition task
lands. Omitting `--with-smtp` leaves copy-link invitations available. Local-file credentials cannot
claim an invitation or register a first passkey.

## Stack inspection

After a local run, health is served through the one loopback gateway origin. Use the printed URL,
or read `PLE_GATEWAY_HOST_PORT` in `containers/env.local`; first-run bootstrap selects 3000-3099.

```bash
curl -s http://127.0.0.1:3000/health
podman compose -f containers/compose.yaml --env-file containers/env.local ps
podman compose -f containers/compose.yaml --env-file containers/env.local down
```

The normal `down` command retains named data volumes. See
[LOCAL_STACK_OPERATIONS.md](LOCAL_STACK_OPERATIONS.md) for health behavior and service-specific logs.

## Known gaps

- Complete account-provider composition, then verify the selected provider's real email delivery
  before asking learners to use canonical passwordless sign-in.
- Broader PG/PGML compatibility, including MATCH, requires source and live evidence beyond the
  accepted bounded radio-button path.
